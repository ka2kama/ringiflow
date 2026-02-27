# #971 Claude CI レビューの並列化による高速化

## 概要

Auto Review と Rules Check の両 CI ワークフローを単一ジョブから4ジョブ並列構成に分割し、壁時間を短縮した。Auto Review では Validation を Sonnet に移行しコスト削減。Rules Check では LPT アルゴリズムでルールをグループ分割し並列チェック。

## 実施内容

### Phase 1: Auto Review の4ジョブ分割

`claude-auto-review.yaml` を4ジョブ構成にリライト:

| ジョブ | 責務 | モデル |
|--------|------|--------|
| setup | 共通前処理（draft check, コンテキスト収集） | — |
| verification | コード品質レビュー | Opus |
| validation | Issue/計画との完了基準突合 | Sonnet |
| finalize | 結果集約、ステータス報告、承認判断 | — |

設計上のポイント:
- verification と validation は `needs: [setup]` で並列実行
- Validation は `gh pr comment` のみ使用（`gh pr review` なし）。`claude[bot]` のレビュー状態は最後の `gh pr review` のみ表示されるため、Validation が Verification の `request-changes` を上書きしないようにする
- finalize は `if: always()` で常に実行し、verification の結果に基づき承認判断

### Phase 2: Rules Check の並列化

#### match-rules.rs への `--groups` フラグ追加

TDD で `split_into_groups` 関数を実装:
- LPT（Longest Processing Time first）アルゴリズムでルールをサイズベースで均等分割
- 出力形式: `<!-- group:1 -->` / `<!-- group:2 -->` マーカーで区切り
- 後方互換: `--groups` なしで従来と同一の出力形式を維持
- テスト 4 件追加（既存 11 件 + 新規 4 件 = 計 15 件）

#### claude-rules-check.yaml の4ジョブ分割

| ジョブ | 責務 | モデル |
|--------|------|--------|
| setup | 前処理 + Rust toolchain + ルールマッチ（グループ分割） | — |
| rules-group-1 | グループ1のルール準拠チェック | Sonnet |
| rules-group-2 | グループ2のルール準拠チェック | Sonnet |
| finalize | 結果集約、ステータス報告 | — |

設計上のポイント:
- setup で `match-rules.rs --groups 2` を実行し、awk でグループ別に分割して outputs に格納
- 各グループジョブは `<!-- no-matching-rules -->` を含む場合スキップ
- finalize は実行されたグループ数を数え、最新 N 件の `claude[bot]` コメントから `rules-check-result:fail` マーカーを検索

## 判断ログ

- Validation モデルを Sonnet に変更: 完了基準の突合作業は Opus の推論能力が不要。コスト削減が主目的
- review-metadata の維持: 現時点でコンシューマは存在しないが、後方互換のため維持。verification/validation それぞれに `metadata-type` を付与
- LPT アルゴリズム選択: ルールファイルのサイズ（≒トークン数）でバランスを取る。実装がシンプルで十分な均等性を提供
- finalize の集約ロジック: `concurrency` グループで同時実行を防ぎ、最新 N 件のコメントをチェックする方式。N = 実際に実行されたグループ数
- `gh pr review` の使用制限: Validation ジョブでは `gh pr review` を `allowedTools` から除外。Verification の承認判断を保護

## 成果物

### コミット

| コミット | 内容 |
|---------|------|
| `081f226f` | WIP: 空コミット（Draft PR 作成用） |
| `0ea5a26f` | 計画ファイル追加 |
| `772ed7ff` | Auto Review を4ジョブに分割 |
| `5e74f99d` | match-rules.rs に `--groups` フラグ追加 |
| `dab9afbf` | Rules Check を4ジョブに分割 |

### 作成・更新ファイル

| ファイル | 変更内容 |
|---------|---------|
| `.github/workflows/claude-auto-review.yaml` | 4ジョブ構成にリライト |
| `.github/workflows/claude-rules-check.yaml` | 4ジョブ構成にリライト |
| `.github/scripts/match-rules.rs` | `--groups N` フラグ、LPT 分割、CLI 解析追加 |
| `prompts/plans/971_parallelize-claude-review.md` | 実装計画 |

## 議論の経緯

- Issue #971 の精査時、当初 Auto Review のみのスコープだったが、計測データから Rules Check がボトルネックであることを確認し、Rules Check も含むスコープに拡大
- Rules Check の完了基準を Issue #971 に追記する方針を決定（別 Issue ではなく同一 Issue で管理）
- 計測データ（直近14回の実行）を設計判断の根拠として Issue に記録
