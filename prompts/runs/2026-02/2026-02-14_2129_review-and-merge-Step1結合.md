# review-and-merge Step 1 の PR 状態確認と base branch 同期確認を結合

## 概要

`/review-and-merge` スキルの Step 1 で分離されていた PR 状態確認と base branch 同期確認を、一括実行として結合した。

## 実施内容

- SKILL.md の Step 1 を修正
  - `gh pr view` と `git fetch + log` を同一コードブロックに結合
  - サブセクション（`####`）を排除し、判定テーブルを PR 状態と base branch 同期に分離
  - 「分離して実行しないこと」の明示的指示を追加
  - 改善の経緯リンクに新しい改善記録を追加

## 判断ログ

- 特筆すべき判断なし（改善記録の対策をそのまま実施）

## 成果物

- コミット: `#518 Combine PR status check and base branch sync in review-and-merge Step 1`
- PR: #523（Draft）
- 変更ファイル: `.claude/skills/review-and-merge/SKILL.md`
