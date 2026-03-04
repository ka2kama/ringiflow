# #1036 PR レビュープロンプトのベストプラクティス起点での再設計

## コンテキスト

### 目的
- Issue: #1036
- Want: PR レビュープロンプトの設計が methodology-design.md の原則に従い、レビューの検出力と効率が実測データに基づいて改善されること
- 完了基準:
  - 既知手法との対応をナレッジベースに記録する
  - Verification プロンプトのレビュー観点を既知手法を参照して再構成する
  - Rules Check の重大度モデルを再設計し、ドキュメントスタイル違反が Critical にならないようにする
  - レビュー効率を改善する（LGTM 簡略化、再レビューの最適化）
  - ADR に設計判断を記録する

### ブランチ / PR
- ブランチ: `feature/1036-pr-review-best-practices`
- PR: #1037（Draft）

### As-Is（探索結果の要約）

レビューシステムの構成:
- `claude-auto-review.yaml`: Verification（Opus）+ Validation（Sonnet）+ Finalize
- `claude-rules-check.yaml`: Rules Check Group 1 + Group 2 + Finalize
- 合計 4 つのレビューコメントが 1 PR に付く

直近 20 PR の分析結果:
- Rules Check: 全違反が Critical 扱い → 太字使用やASCII art 等のスタイル問題で action-required: true
- Verification: Critical 0、High 0、Medium 2（1 PRのみ）、Low ~8。セキュリティ/パフォーマンス検出 0
- コメント膨張: 再レビュー時に全4種類を再実行、LGTM コメントが大量

### 進捗
- [x] Phase 1: ナレッジベース追記（既知手法との対応）
- [x] Phase 2: ADR 作成
- [x] Phase 3: Rules Check 重大度モデルの再設計
- [x] Phase 4: Verification プロンプトの再構成
- [x] Phase 5: レビュー効率改善

## 設計判断

### 判断 1: Rules Check の重大度モデル

現状: 全ルール違反が Critical → action-required: true

選択肢:
1. **禁止事項のみ blocking、その他は non-blocking（採用）** — Rules Check プロンプトで違反を2階層に分類。`**禁止:**` に該当する違反のみ severity-high（blocking）、その他は severity-medium/low（non-blocking）
2. Rules Check 全体を non-blocking にする — Rules Check の意義が薄れる。禁止事項の違反は止めるべき
3. 現状維持 — 実測データが問題を示している

根拠: Google Code Review Guidelines の Nit: 概念、Conventional Comments の blocking/non-blocking デコレータ。ルール違反にも重大度の差がある。

### 判断 2: Verification 観点の再構成

現状の 6 観点を既知手法ベースで再構成する。

| 現行 | 再構成後 | 根拠 |
|------|---------|------|
| 1. バグ・正確性 | 1. 正確性・ロジック | Google: Functionality |
| 2. セキュリティ | 2. セキュリティ（高確信度のみ） | AI研究: FPR を最小化、確信度の明示 |
| 3. パフォーマンス | 3. パフォーマンス（明白なもののみ） | SmartBear: 確信度が低い指摘を避ける |
| 4. 型システムの活用 | 4. 型安全性・堅牢性 | 維持（プロジェクト固有の強み） |
| 5. テスト | 5. テストの妥当性 | Google: Tests |
| 6. 設計 | 6. 設計・複雑さ | Google: Design + Complexity |
| （なし） | 7. コード改善（non-blocking） | Bacchelli: 実際のレビュー成果で最多 |

追加の設計原則:
- 全コメントに重大度ラベルを必須化（Google Nit/FYI 相当）
- 問題指摘には理由（なぜ問題か）と修正提案を含める（Google + Conventional Comments）
- 確信度が低い指摘は明示する（AI 研究）
- PR あたりのコメント数ガイドライン: 重要な問題に絞る（SmartBear: alert fatigue 防止）

### 判断 3: LGTM コメントの簡略化

Rules Check の「問題なし」コメントが冗長（チェックしたルール一覧を毎回表示）。

変更: LGTM 時は 1 行 + メタデータのみ。ルール一覧は省略。

## Phase 定義

### Phase 1: ナレッジベース追記

対象ファイル: `docs/80_ナレッジベース/methodology/独自フレームワークと既知手法の対応.md`

追記内容: 「claude-auto-review.yaml（PR レビュー）と既知手法」セクション

確認事項: なし（既知のパターンのみ）

操作パス: 該当なし（ドキュメントのみ）

テストリスト:
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 2: ADR 作成

対象ファイル: `docs/70_ADR/063_PRレビュープロンプトのベストプラクティス起点再設計.md`

確認事項:
- パターン: 既存 ADR のフォーマット → `docs/70_ADR/template.md`

操作パス: 該当なし（ドキュメントのみ）

テストリスト:
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 3: Rules Check 重大度モデルの再設計

対象ファイル: `.github/workflows/claude-rules-check.yaml`

変更内容:
- メタデータのルールを変更: 全違反 Critical → 禁止事項のみ High、その他は Medium/Low
- `action-required`: High 以上がある場合のみ true
- LGTM コメントの簡略化

確認事項:
- パターン: 現行の Rules Check プロンプト → `.github/workflows/claude-rules-check.yaml`（確認済み）
- パターン: determine-rules-check-status.sh のロジック → `scripts/ci/determine-rules-check-status.sh`

操作パス: 該当なし（CI 設定変更）

テストリスト:
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 4: Verification プロンプトの再構成

対象ファイル: `.github/workflows/claude-auto-review.yaml`（verification ジョブ）

変更内容:
- レビュー観点を既知手法ベースで再構成（7 観点）
- コメントの構造化（重大度ラベル + 理由 + 修正提案）
- 確信度の明示ルール
- コメント数のガイドライン
- AI の限界の明示

確認事項:
- パターン: 現行の Verification プロンプト → `.github/workflows/claude-auto-review.yaml`（確認済み）

操作パス: 該当なし（CI 設定変更）

テストリスト:
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 5: レビュー効率改善

対象ファイル:
- `.github/workflows/claude-auto-review.yaml`（Validation プロンプト）
- CLAUDE.md の PR レビューセクション

変更内容:
- Validation の LGTM 簡略化（確認済み項目の一覧は維持、形式を簡潔に）
- CLAUDE.md のレビュー承認基準を更新

確認事項: なし（Phase 3-4 で確認済みのパターンのみ）

操作パス: 該当なし（CI 設定変更 + ドキュメント）

テストリスト:
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | determine-rules-check-status.sh は結果マーカー（pass/fail）で判定しており、メタデータの severity は判定に使われていない。severity モデルを変更しても status 判定ロジックは影響を受けない | アーキテクチャ不整合 | Phase 3 の対象を結果マーカー基準の変更に焦点化。status 判定スクリプトは変更不要 |
| 2回目 | Verification の承認判断ロジック（Critical/High → request-changes）は変更不要。観点の再構成は承認判断に影響しない | 不完全なパス | Phase 4 のスコープを明確化：承認判断ロジックは維持、観点とコメント構造のみ変更 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 完了基準 5 項目が全て Phase に対応 | OK | Phase 1=ナレッジベース、Phase 2=ADR、Phase 3=Rules Check、Phase 4=Verification、Phase 5=効率改善 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更内容が具体的に定義されている |
| 3 | 設計判断の完結性 | 全ての判断が記載 | OK | 3 つの設計判断が選択肢・理由・根拠付きで記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象: Verification/Rules Check プロンプト、ナレッジベース、ADR。対象外: Validation のロジック変更、finalize スクリプトの変更 |
| 5 | 技術的前提 | 前提が考慮されている | OK | determine-rules-check-status.sh が結果マーカー基準であることを確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | CLAUDE.md のレビュー承認基準、ADR-011 との整合を確認 |
