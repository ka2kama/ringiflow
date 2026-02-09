# Issue #151: 構造劣化の検出メカニズムを導入する

## Context

コードベースの構造劣化（未使用依存、ファイル肥大化、責務重複）は連続的に進行し、人間にも AI エージェントにも認知しにくい（茹でガエル問題）。既存の防御策（clippy、cargo-deny、改善記録）はあるが、Cargo.toml の未使用依存やファイルサイズの肥大化を機械的に検出する仕組みがない。

Issue #151 は「優先度の高いものを選択して実装する」方針。全4カテゴリを実装する。

## 設計判断

### cargo-machete を採用（cargo-udeps ではなく）

| 観点 | cargo-machete | cargo-udeps |
|------|---------------|-------------|
| ツールチェイン | stable | nightly 必須 |
| 検出方式 | ヒューリスティック（正規表現） | コンパイルアーティファクト分析 |
| 速度 | 高速（数秒） | 低速（フルコンパイル） |
| 偽陽性対策 | `[workspace.metadata.cargo-machete]` ignore | N/A |
| CI 統合 | `cargo install` + `cargo machete` | nightly コンパイル必要 |

選択理由: プロジェクトの nightly は rustfmt 専用。cargo-udeps のためだけに CI で nightly コンパイルを行うのは CI 時間の浪費。偽陽性は ignore リストで管理可能。

### CI 統合: `cargo install` 方式（GitHub Action ではなく）

`bnjbvr/cargo-machete` Action はタグ付きリリースの信頼性が不明。`cargo install` + cargo キャッシュ（`~/.cargo/bin/`）で十分高速。GitHub Actions 許可設定の変更も不要。

### ファイルサイズ: シェルスクリプト（専用ツールではなく）

閾値チェックは `wc -l` + シェルスクリプトで十分。tokei 等の追加ツール導入は KISS に反する。

### 閾値: 500 行（警告のみ、CI 非ブロッキング）

現状 12 ファイルが 500 行超（最大 1938 行）。即時のリファクタリングは求めず、今後の肥大化を可視化する。

## 対象外

- モジュール結合度の計測: 現コードベース規模（6 クレート）では過剰。ファイルサイズ閾値で代替
- Elm の未使用依存検出: elm-review が未使用インポートを検出済み
- 依存関係の健全性チェック（完了基準 3）: cargo-machete が実質的にこの機能を提供

## Phase 構成

### Phase 1: cargo-machete による未使用依存検出

ADR-038 作成 → ツール導入 → justfile 統合 → CI 統合

#### 確認事項
- パターン: `lint-rust` レシピの構造 → `justfile` L217-219
- パターン: `check-tools` のツール確認パターン → `justfile` L28-49
- パターン: `rust` ジョブのステップ構成 → `.github/workflows/ci.yaml` L58-112
- パターン: ADR フォーマット → `docs/05_ADR/template.md`
- パターン: 開発環境構築の記載形式 → `docs/04_手順書/01_開発参画/01_開発環境構築.md`
- ライブラリ: cargo-machete の CLI オプション → `cargo machete --help`（ローカルインストール後に確認）

#### 変更対象ファイル

1. `docs/05_ADR/038_未使用依存検出ツールの選定.md` — 新規作成（cargo-machete vs cargo-udeps）
2. `justfile` — 変更:
   - `check-tools` に `cargo-machete` を追加
   - `check-unused-deps` レシピを追加
   - `lint` に `check-unused-deps` を追加
3. `.github/workflows/ci.yaml` — 変更:
   - `rust` ジョブに `cargo-machete` インストール + 実行ステップを追加
4. `docs/04_手順書/01_開発参画/01_開発環境構築.md` — セクション追加（cargo-machete）
5. `backend/Cargo.toml` — 偽陽性があれば `[workspace.metadata.cargo-machete]` 追加

#### テストリスト
- [ ] `cargo install cargo-machete` でローカルインストール成功
- [ ] `cd backend && cargo machete` が正常に実行され結果を返す
- [ ] 偽陽性があれば ignore 設定で抑制されることを確認
- [ ] `just check-unused-deps` が正常に動作する
- [ ] `just check-tools` が cargo-machete のインストールを確認する
- [ ] `just lint` に check-unused-deps が含まれることを確認

### Phase 2: ファイルサイズ閾値アラート

#### 確認事項
- パターン: 既存シェルスクリプトの構造 → `scripts/check-rule-files.sh`
- パターン: justfile のシェルスクリプトブロック → `justfile` 内の `#!/usr/bin/env bash` パターン
- 技術的前提: ShellCheck でパスすること

#### 変更対象ファイル

1. `scripts/check-file-size.sh` — 新規作成:
   - 対象: `backend/**/*.rs` + `frontend/src/**/*.elm`
   - テストファイル除外: `tests/` 配下、`*_test.rs`
   - 閾値: 500 行（警告のみ、exit 0）
   - 超過ファイル一覧と行数を出力
2. `justfile` — 変更:
   - `check-file-size` レシピを追加
   - `check` に `check-file-size` を追加（非ブロッキングなので可）

#### テストリスト
- [ ] `scripts/check-file-size.sh` が 500 行超のファイルをリストアップする
- [ ] テストファイル（`auth_integration_test.rs` 等）が除外される
- [ ] 閾値以下のファイルのみの場合、超過なしのメッセージを出力する
- [ ] ShellCheck でスクリプトがパスする
- [ ] `just check-file-size` が正常に動作する

### Phase 3: 構造レビュー指針 + CLAUDE.md 統合

#### 確認事項
- パターン: `.claude/rules/` ファイルの paths フロントマター → `.claude/rules/api.md` L1-6
- パターン: CLAUDE.md からルールファイルへの参照方式 → CLAUDE.md 内の `→ 詳細:` パターン
- パターン: 品質チェックリストの構造 → `docs/04_手順書/04_開発フロー/01_Issue駆動開発.md` L354-401

#### 変更対象ファイル

1. `.claude/rules/structural-review.md` — 新規作成:
   - 新モジュール追加時のチェックポイント（責務重複確認）
   - ファイルサイズ閾値の指針（500 行超で分割検討）
   - Phase 完了時の構造確認（`just check-unused-deps` + `just check-file-size`）
2. `CLAUDE.md` — 変更: 構造レビュー指針への参照を追加
3. `docs/04_手順書/04_開発フロー/01_Issue駆動開発.md` — 変更: 品質チェックリストに構造確認項目を追加

#### テストリスト
- [ ] `.claude/rules/structural-review.md` が paths フロントマター付きで作成されている
- [ ] CLAUDE.md から structural-review.md へのリンクが有効
- [ ] 品質チェックリストに構造確認項目が追加されている
- [ ] `scripts/check-rule-files.sh` で新規ルールファイルが検出される

### Phase 4: ドキュメント整備 + CI 確認

#### 確認事項
- なし（既知のパターンのみ）

#### 変更対象ファイル

1. `docs/05_ADR/README.md` — 変更: ADR-038 を一覧に追加
2. Issue #151 — チェックボックスを更新

#### テストリスト
- [ ] `just check-all` が通る
- [ ] CI（PR 作成後）で cargo-machete ステップが成功する

## 完了基準との対応

| Issue の完了基準 | Phase | 達成方法 |
|-----------------|-------|---------|
| CI に cargo udeps or 同等チェック | 1 | cargo-machete を CI に導入 |
| `#[warn(dead_code)]` が CI で有効 | — | clippy `-D warnings` で既に有効（追加作業不要） |
| 定期的な未使用コードレビューの仕組み | 3 | Phase 完了時の構造確認項目として組み込み |
| ファイルサイズ閾値アラート | 2 | シェルスクリプト + justfile タスク |
| Cargo.toml vs import の整合性 | 1 | cargo-machete が提供 |
| 新モジュール追加時の責務重複確認 | 3 | `.claude/rules/structural-review.md` |
| Phase 完了時の構造レビュー | 3 | 品質チェックリストへの追加 |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | CI で cargo-machete GitHub Action を使う場合、Action 許可設定の管理が必要 | 既存手段の見落とし | `cargo install` 方式に変更。許可設定変更不要 |
| 2回目 | ファイルサイズ閾値でテストファイルの除外が未定義 | 未定義 | テストファイル除外を明記（テストは行数が多くなりやすい性質） |
| 3回目 | `check` に `check-file-size` を追加すると非ブロッキングでも実行順序に影響するか | 曖昧 | 非ブロッキング（exit 0）なので `check` の一部として問題なし |
| 4回目 | 既存ファイルの多くが 500 行超（12 ファイル）。閾値が低すぎないか | 技術的前提 | 警告のみ（CI 非ブロッキング）なので 500 行で妥当。可視化が目的 |
| 5回目 | `lint` に `check-unused-deps` を追加すると CI の `just lint-rust` では含まれない | アーキテクチャ不整合 | `lint` レシピに追加（`lint-rust` とは別のステップ）。CI は `just lint-rust` の後に別ステップで実行 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了基準がすべて計画に含まれている | OK | 4 カテゴリの完了基準すべてに Phase を割り当て。dead_code は既に有効と確認 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | cargo-machete 偽陽性は「あれば対応」と条件付き。閾値・除外ルールを明示 |
| 3 | 設計判断の完結性 | すべての差異に判断が記載されている | OK | 3 判断（machete vs udeps、install vs Action、スクリプト vs ツール）を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象外セクションで 3 項目を理由付きで除外 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | cargo キャッシュ、ShellCheck 互換、既存 500 行超ファイルの存在を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | CI 構造、justfile パターン、ADR 番号、ルールファイル形式と照合 |

## 検証方法

1. ローカル: `just check-all` が通ること
2. ローカル: `just check-unused-deps` が cargo-machete を実行すること
3. ローカル: `just check-file-size` が 500 行超ファイルをリストアップすること
4. CI: PR 作成後に `rust` ジョブで cargo-machete が実行されること
