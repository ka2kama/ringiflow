# #971 Claude CI レビューの並列化による高速化

## Context

Claude Auto Review と Rules Check の CI ワークフローを並列化・最適化し、壁時間短縮とコスト削減を実現する。

### 計測データ

直近14回の実行データ（Draft スキップを除外）:

| ワークフロー | モデル | Claude Call 平均 | Claude Call 範囲 | Total 範囲 |
|-------------|--------|-----------------|-----------------|-----------|
| Auto Review | Opus | ~125s | 90-165s | 72-199s |
| Rules Check | Sonnet | ~206s | 129-327s | 35-365s |

壁時間 = max(Auto Review, Rules Check) → **Rules Check がボトルネック**。

Rules Check が遅い根本原因: match-rules.rs が14-15ルールの全文（120-220KB、40-50K+ tokens）をプロンプトに注入。Sonnet がターンごとに全 tokens を再処理する。

### 期待される効果

| 改善 | Auto Review 分割 | Rules Check 並列化 | 合計 |
|------|-----------------|-------------------|------|
| 壁時間 | ~20s 改善（限定的） | ~60-130s 改善（効果大） | ~80-150s 改善 |
| コスト | Validation を Opus→Sonnet | 変更なし | 削減 |
| 保守性 | V/V プロンプト独立調整 | ルールグループ独立調整 | 向上 |

## 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `.github/workflows/claude-auto-review.yaml` | 4ジョブ構成に書き換え |
| `.github/workflows/claude-rules-check.yaml` | 4ジョブ構成に書き換え |
| `.github/scripts/match-rules.rs` | `--groups N` フラグ追加 |

変更なし（参照のみ）:
- `scripts/ci/fetch-validation-context.sh` — setup で引き続き使用
- `.github/workflows/ci.yaml` — `needs` + `outputs` パターンの参考

## 設計判断

### 1. Validation は `gh pr review` を使わない

`gh pr comment` のみ（Rules Check と同じ方式）。理由: `claude[bot]` の最新 review state は1つしか表示されず、Validation の `--approve` が Verification の `--request-changes` を上書きするリスクがある。

### 2. メタデータ type の分割

`type: auto-review` → `type: verification` + `type: validation` に分割。消費者（パーサー）は現在存在しないため互換性の問題なし。

### 3. Rules Check のルールグループ分割

`match-rules.rs` に `--groups N` フラグを追加し、マッチしたルールを内容サイズで均等に N グループに分割出力する。各グループを別の Claude ジョブで並列チェックする。

### 4. ステータス報告は各ワークフローの finalize に集約

Auto Review: finalize が Verification の review state を確認してステータスを報告。
Rules Check: finalize が各グループの結果コメントから `rules-check-result:pass/fail` マーカーを確認してステータスを報告。

---

## Phase 1: Auto Review の4ジョブ分割

変更対象: `.github/workflows/claude-auto-review.yaml`

### ジョブ構成

```
setup ──→ verification (Opus, コード品質)     ──→ finalize
      └─→ validation  (Sonnet, 完了基準突合)  ──┘
```

| ジョブ | 責務 | モデル | max-turns |
|--------|------|--------|-----------|
| setup | draft check, checkout, コンテキスト収集 | — | — |
| verification | コード品質レビュー + `gh pr review` | Opus | 30 |
| validation | 完了基準突合 + `gh pr comment` | Sonnet | 15 |
| finalize | review state 確認 + ステータス報告 | — | — |

### setup の outputs

| output | 内容 | 推定サイズ |
|--------|------|-----------|
| `is_draft` | Draft フラグ | ~5B |
| `pr_number` | PR 番号 | ~5B |
| `PR_COMMENTS` | PR コメント JSON（最新20件） | ~10-50KB |
| `REVIEW_COMMENTS` | レビューコメント JSON | ~10-50KB |
| `REVIEW_MODE` | レビューモード（初回/増分 + diff） | ~1-60KB |
| `VALIDATION_CONTEXT` | PR本文 + Issue + 計画ファイル | ~10-30KB |

合計 ~190KB。GitHub Actions outputs の 1MB 制限内。

### Verification プロンプト

現在のプロンプトから抽出:
- PR コメント + レビューモード（setup outputs 注入）
- 言語指定、基本方針（CLAUDE.md の理念）
- Verification チェック観点（バグ、セキュリティ、パフォーマンス、型、テスト、設計）
- 学習機会の提供
- 指摘しないこと / 過去の議論判断
- フィードバック: インラインコメント + PR コメント + **`gh pr review`**
- 重大度判定 + 承認判断ロジック
- メタデータ: `type: verification`

allowedTools: 現在と同じ（`gh pr review` 含む）

### Validation プロンプト

現在のプロンプトから抽出:
- PR コメント + レビューモード + Validation コンテキスト（setup outputs 注入）
- 言語指定、基本方針
- Validation チェック観点（欠落・乖離の検出、PR本文品質確認セクション検証）
- フィードバック: インラインコメント + **`gh pr comment` のみ**（`gh pr review` は使用しない）
- メタデータ: `type: validation`

allowedTools: `gh pr review` を**除外**、`Read` を追加

### finalize のロジック

```
verification_result = needs.verification.result
validation_result = needs.validation.result
review_state = claude[bot] の最新 review state（API 取得）

if verification_result == failure || validation_result == failure:
  → failure: "Review job failed"
elif review_state == CHANGES_REQUESTED:
  → failure: "Changes requested"
else:
  → success: "Review completed"
```

`if: always() && needs.setup.outputs.is_draft == 'false'`

### 確認事項

- パターン: `ci.yaml` の `needs` + `outputs` → 確認済み
- パターン: `claude-rules-check.yaml` の `gh pr comment` のみ方式 → 確認済み
- GitHub Actions: multiline outputs のジョブ間渡し（heredoc） → 確認済み
- GitHub Actions: outputs 1MB 制限 → 確認済み（~190KB で制限内）
- メタデータ: `type: auto-review` の消費者 → 確認済み（存在しない）

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | Draft PR で全ジョブがスキップ | 正常系 | 手動 E2E |
| 2 | Verification がコメント + approve | 正常系 | 手動 E2E |
| 3 | Validation がコメントのみ投稿 | 正常系 | 手動 E2E |
| 4 | finalize が success ステータス報告 | 正常系 | 手動 E2E |
| 5 | Verification Critical/High → request-changes + failure | 準正常系 | 手動 E2E |
| 6 | action error → failure ステータス | 異常系 | 手動 E2E |

### テストリスト

ユニットテスト（該当なし — CI ワークフロー）
ハンドラテスト（該当なし）
API テスト（該当なし）

E2E テスト（手動検証）:
- [ ] Draft PR で setup が skipped を報告し他ジョブ不実行
- [ ] Verification が Opus でインラインコメント + PR コメント + `gh pr review` を実行
- [ ] Validation が Sonnet で PR コメントのみ投稿（`gh pr review` なし）
- [ ] finalize が Verification の review state に基づきステータス報告
- [ ] メタデータ `type` が `verification` / `validation` に分離
- [ ] Verification Critical/High 時に finalize が failure 報告

---

## Phase 2: Rules Check の並列化

変更対象: `.github/workflows/claude-rules-check.yaml`, `.github/scripts/match-rules.rs`

### ジョブ構成

```
setup ──→ rules-group-1 (Sonnet)  ──→ finalize
      └─→ rules-group-2 (Sonnet)  ──┘
```

| ジョブ | 責務 | モデル | max-turns |
|--------|------|--------|-----------|
| setup | draft check, checkout, Rust toolchain, ルールマッチ + グループ分割 | — | — |
| rules-group-1 | グループ1のルール準拠チェック + `gh pr comment` | Sonnet | 15 |
| rules-group-2 | グループ2のルール準拠チェック + `gh pr comment` | Sonnet | 15 |
| finalize | 結果マーカー確認 + ステータス報告 | — | — |

### match-rules.rs の変更

`--groups N` CLI フラグを追加:

```
# 現在（変更なし）
rust-script match-rules.rs changed-files.txt

# 新規: グループ分割モード
rust-script match-rules.rs --groups 2 changed-files.txt
```

**分割アルゴリズム**: マッチしたルールを content size（バイト数）で降順ソートし、各グループの合計サイズが均等になるよう greedy に振り分ける（Longest Processing Time first アルゴリズム）。

**出力形式**（`--groups 2` 時）:

```
<!-- group:1 -->
マッチしたルール: N 件

- `.claude/rules/rust.md`
- ...

### .claude/rules/rust.md

(ルール本文)

<!-- group:2 -->
マッチしたルール: M 件

- `.claude/rules/docs.md`
- ...

### .claude/rules/docs.md

(ルール本文)
```

**エッジケース**:
- マッチ 0 件 → `<!-- no-matching-rules -->` （現在と同じ）
- マッチ 1 件 → グループ1 のみに配置、グループ2 は `<!-- no-matching-rules -->`

### setup の outputs

| output | 内容 |
|--------|------|
| `is_draft` | Draft フラグ |
| `pr_number` | PR 番号 |
| `has_matching_rules` | マッチルールの有無 |
| `PR_COMMENTS` | PR コメント JSON |
| `REVIEW_COMMENTS` | レビューコメント JSON |
| `REVIEW_MODE` | レビューモード |
| `RULES_GROUP_1` | グループ1のルール + 本文 |
| `RULES_GROUP_2` | グループ2のルール + 本文 |

各グループ推定サイズ: ~60-110KB（現在の半分）。1MB 制限内。

### Rust toolchain の配置

Rust toolchain setup、sccache、Cargo cache、rust-script install は **setup ジョブ**に集約。並列ジョブでは不要（match-rules.rs の実行は setup で完了）。

### 各グループのプロンプト構造

現在のプロンプトとほぼ同じ。差異:
- マッチルール部分が `${{ needs.setup.outputs.RULES_GROUP_N }}` に変更
- max-turns: 20 → 15（ルール数が半分のため）

### finalize のロジック

```bash
# 各グループの結果マーカーを確認
# claude[bot] の最新コメント2件から rules-check-result:pass/fail を抽出
# いずれかに fail があれば → failure
# 全て pass → success
```

`if: always() && needs.setup.outputs.is_draft == 'false' && needs.setup.outputs.has_matching_rules == 'true'`

### 確認事項

- 型: `match-rules.rs` の `MatchedRule` 構造体 → 確認済み（path + body）
- パターン: 現在の `match-rules.rs` の出力形式 → 確認済み（サマリー + 各ルール本文）
- ライブラリ: `globset` crate のグループ化機能 → 不要（標準の Vec 操作で実装）
- GitHub Actions: outputs の合計サイズ制限 → 各グループ ~60-110KB、制限内

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | Draft PR で全ジョブがスキップ | 正常系 | 手動 E2E |
| 2 | マッチルールなしで全ジョブスキップ | 正常系 | 手動 E2E |
| 3 | 両グループが pass → success | 正常系 | 手動 E2E |
| 4 | グループ1が fail → failure | 準正常系 | 手動 E2E |
| 5 | マッチ1件でグループ2が空 | エッジケース | ユニットテスト |

### テストリスト

ユニットテスト:
- [ ] `match-rules.rs`: `--groups 2` でルールが均等にグループ分割される
- [ ] `match-rules.rs`: マッチ0件で `<!-- no-matching-rules -->` 出力
- [ ] `match-rules.rs`: マッチ1件でグループ1のみに配置
- [ ] `match-rules.rs`: グループ分割がサイズベースで均等（LPT アルゴリズム）
- [ ] `match-rules.rs`: フラグなしで現在の出力形式を維持（後方互換）

ハンドラテスト（該当なし）
API テスト（該当なし）

E2E テスト（手動検証）:
- [ ] Draft PR で setup が skipped を報告
- [ ] マッチルールなしで skipped を報告
- [ ] 両グループが並列実行されている（GitHub Actions UI で確認）
- [ ] 各グループが正しい PR コメント + メタデータを投稿
- [ ] finalize が両グループの結果に基づきステータス報告
- [ ] ルール違反時に finalize が failure 報告

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Validation の `gh pr review` が Verification を上書きするリスク | 競合 | Validation は `gh pr comment` のみに変更 |
| 2回目 | `type: auto-review` メタデータの消費者の有無 | 既存手段 | Grep で確認 → 消費者なし、分割安全 |
| 3回目 | finalize が draft PR 時にも実行される | 不完全なパス | `is_draft == 'false'` 条件追加 |
| 4回目 | Issue の前提「Opus がボトルネック」が計測データと矛盾 | 曖昧 | 計測データを計画に記載、Rules Check 最適化を追加 |
| 5回目 | Rules Check 改善が対象外になっていた | スコープ境界 | Phase 2 として Rules Check 並列化を追加 |
| 6回目 | Rules Check の setup に Rust toolchain が必要 | 技術的前提 | Rust toolchain を setup ジョブに集約、並列ジョブでは不要 |
| 7回目 | マッチ1件時のグループ分割エッジケース | 競合・エッジケース | グループ2が空の場合の処理を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全ジョブ・全ファイルの変更が記載 | OK | Auto Review 4ジョブ + Rules Check 4ジョブ + match-rules.rs |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各ジョブの責務、モデル、max-turns、allowedTools が明確 |
| 3 | 設計判断の完結性 | 全差異に判断と理由 | OK | Validation review不使用、メタデータ分割、LPT アルゴリズムに理由記載 |
| 4 | スコープ境界 | 対象と対象外 | OK | 対象: 2ワークフロー + match-rules.rs。対象外: プロンプト内容の実質変更 |
| 5 | 技術的前提 | 前提が確認済み | OK | outputs 1MB制限、Rust toolchain配置、claude-code-action 動作 |
| 6 | 既存ドキュメント整合 | 矛盾なし | OK | ADR-011 のステータス報告方式を維持 |

## 検証方法

1. `just check-all` で既存テストが通ることを確認（match-rules.rs のユニットテスト含む）
2. Draft PR で push → 両ワークフローが skipped になることを確認
3. Ready for Review → CI 通過後:
   - Auto Review: verification + validation が並列実行、別々のコメントを投稿
   - Rules Check: 2グループが並列実行、各グループがコメントを投稿
4. finalize が正しいステータスを報告することを確認
5. GitHub Actions UI でジョブの並列実行を目視確認
