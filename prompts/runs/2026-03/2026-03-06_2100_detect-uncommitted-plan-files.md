# 計画ファイルのコミット漏れを lefthook pre-commit で検出する

- Issue: #1063
- PR: #1065
- ブランチ: `feature/1063-detect-uncommitted-plan-files`

## 概要

計画ファイル（`prompts/plans/`）のコミット漏れが 3 回再発している問題に対し、品質ゲート 6.4 の手動確認を lefthook pre-commit の自動検出に転換した。

## 変更内容

1. `scripts/check/check-uncommitted-plans.sh` を新規作成
   - feature/fix ブランチで `prompts/plans/` の unstaged/untracked ファイルを検出
   - 警告のみ（exit 0）でコミットはブロックしない
   - ステージ済みファイルは除外（false positive 防止）
2. `lefthook.yaml` の pre-commit に `check-uncommitted-plans` hook を追加
3. `.claude/rules/dev-flow-issue.md` の品質ゲート 6.4 を手動確認 → 自動検出に更新

## 判断ログ

| 判断 | 選択 | 理由 |
|------|------|------|
| 検出レベル | 警告（exit 0） | ブロックは開発中の全コミットを止めるため過剰 |
| hook の実装 | 外部スクリプト | 既存パターン（`scripts/check/`）に準拠 |
| ブランチ判定 | main 以外すべて | シンプルで安全 |

## 発見

- 既存の lefthook hooks は `{staged_files}` パターン（ステージ済みファイルの品質チェック）だが、今回は「ワーキングツリーの未コミットファイル検出」という異なる性質の hook
- `git status --porcelain` の XY 形式で、X（staging）と Y（working tree）を区別してステージ済みファイルを除外する必要があった
