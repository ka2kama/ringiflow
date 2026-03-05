# #757 Merge Queue 導入と品質保証の再設計

## コンテキスト

### 目的
- Issue: #757
- Want: 並行開発時のマージスループットを改善しつつ、品質保証レベルを維持する
- 完了基準:
  - 再レビューなしでも品質が担保される仕組みが設計・導入されている
  - Merge Queue が有効化され、`strict_required_status_checks_policy` が `false` になっている
  - ドキュメント・プロセスが更新されている

### ブランチ / PR
- ブランチ: `feature/757-merge-queue`
- PR: #1038（Draft）

### As-Is（探索結果の要約）
- Ruleset `main-protection`（ID: 11811117）: `strict_required_status_checks_policy: true`
- Required checks: `CI Success`（integration_id: 15368）, `Claude Auto Review`, `Claude Rules Check`
- CI: `ci.yaml` — `push`（main）+ `pull_request` トリガー、`merge_group` なし
- Claude Auto Review: `workflow_run` で CI 完了後にトリガー、`event == 'pull_request'` でフィルタ済み
- Claude Rules Check: 同上のフィルタ構造
- マージコマンド: `gh pr merge --squash`（`review-and-merge` スキル）

### 進捗
- [x] Phase 1: 品質保証の再設計（ADR）
- [ ] Phase 2: CI への `merge_group` トリガー追加
- [ ] Phase 3: Merge Queue 有効化とドキュメント更新

## Phase 1: 品質保証の再設計（ADR）

設計判断を ADR-064 に記録。

成果物: `docs/70_ADR/064_MergeQueue導入と品質保証の再設計.md`

## Phase 2: CI への `merge_group` トリガー追加

CI ワークフローに `merge_group` イベントを追加し、Merge Queue の merge commit で CI が実行されるようにする。

#### 確認事項
- パターン: CI ワークフローの `on` トリガー構造 → `.github/workflows/ci.yaml`
- ライブラリ: `dorny/paths-filter@v3` の `merge_group` サポート → Grep 既存使用

#### 変更内容

1. `.github/workflows/ci.yaml`: `on` に `merge_group` を追加

#### 操作パス: 該当なし（CI 設定のみ）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just lint-ci` で actionlint が通ること

## Phase 3: Merge Queue 有効化とドキュメント更新

#### 確認事項
- パターン: `review-and-merge` スキルのマージコマンド → `.claude/skills/review-and-merge/SKILL.md`
- パターン: Issue 駆動開発のマージ手順 → `.claude/rules/dev-flow-issue.md`
- パターン: CLAUDE.md のマージ関連記述 → `CLAUDE.md`

#### 変更内容

1. Ruleset の更新（GitHub API 経由）:
   - `merge_queue` ルール追加
   - `strict_required_status_checks_policy` を `false` に変更
2. `.claude/skills/review-and-merge/SKILL.md`: Merge Queue に関する注記追加
3. `.claude/rules/dev-flow-issue.md`: Merge Queue に関する注記追加
4. `CLAUDE.md`: Merge Queue の記述追加

#### 操作パス: 該当なし（設定・ドキュメント変更のみ）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: Ruleset API で設定が正しいことを確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Claude Auto Review が merge_group でも発火する可能性 | 不完全なパス | ワークフローのフィルタ条件を確認 → `event == 'pull_request'` で既にフィルタ済み、変更不要 |
| 2回目 | Merge Queue の required checks に Auto Review を含めると pending 永久化 | 競合・エッジケース | Merge Queue の required checks は `CI Success` のみに設計 |
| 3回目 | `gh pr merge --squash` が Merge Queue 環境で失敗する | 既存手段の見落とし | `--merge-queue` フラグに変更 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | CI、Ruleset、ドキュメント 3 点すべて Phase に含まれている |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 設定値、コマンド、ファイルパスが具体的に記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | ADR-064 で AI レビュー再実行の要否を判断済み |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: CI / Ruleset / ドキュメント。対象外: Auto Review / Rules Check ワークフロー変更 |
| 5 | 技術的前提 | 前提が考慮されている | OK | paths-filter の merge_group サポート、workflow_run のフィルタ条件を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-011（Claude Code Action 導入）と矛盾なし |
