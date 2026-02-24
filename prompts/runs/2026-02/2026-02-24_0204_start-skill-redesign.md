# /start スキル再設計

## 概要

Issue #858「コンテキストクリア後のワークフローステップ喪失への対策」として、`/start` スキルを「情報提示スキル」から「ワークフロー基盤構築スキル」に再設計した。ブランチと Draft PR の作成を `/start` 内で完結させることで、コンテキストクリア後にステップが喪失する問題を解消する。

## 実施内容

### 問題分析

改善記録 `process/improvements/2026-02/2026-02-24_2106_コンテキストクリア後のDraft-PR作成漏れ.md` に基づく対策。`/start` → 計画作成 → コンテキストクリア → 実装、という流れで、ブランチ作成・Draft PR 作成のステップが喪失する構造的問題。#583（計画ファイルコミット漏れ）と同じ「コンテキスト境界でのステップ喪失」パターン。

### 対策設計

3 つの方向性を検討し、方向性 2（クリア前に完結）+ 方向性 3（復元時に自動検知）の組み合わせを採用。

### ワークフロー状態マシン

4 つの状態を定義し、`/start` 実行時にブランチと PR の有無から状態を検出する:

| 状態 | ブランチ | PR | アクション |
|------|---------|-----|----------|
| A: 新規着手 | なし | なし | Issue 精査 → ブランチ作成 → Draft PR 作成 |
| B: PR 欠落 | あり | なし | Draft PR 復旧 |
| C: 通常再開 | あり | あり（Draft） | コンテキスト提示 |
| D: レビュー中 | あり | あり（Ready） | レビュー状態表示、`/review-and-merge` 案内 |

### SKILL.md 改修

Step 構造を 6 ステップに再構成:

1. main を最新化（変更なし）
2. Issue を特定（変更なし）
3. ワークフロー状態を検出（新規）
4. 状態別フロー（新規）
5. セッションログを参照（変更なし）
6. コンテキストを提示して作業開始（状態別の出力形式に変更）

## 判断ログ

- Issue 精査をブランチ作成前に配置した。精査結果が「破棄」の場合に不要なブランチを作成しない
- worktree 環境の検出に `.worktree-slot` ファイルの存在を使用。既存のレシピ（`prompts/recipes/worktree環境でのブランチ作成.md`）と一致
- 冪等性を状態検出で保証。ブランチ・PR の既存チェックにより、`/start` の重複実行が安全

## 成果物

コミット:
- `#858 WIP: Workflow step persistence across context clear`（空コミット）
- `#858 Redesign /start skill to execute workflow foundation steps`（SKILL.md 改修）
- `#858 Add implementation plan for /start skill redesign`（計画ファイル）

PR: #869

ファイル:
- `.claude/skills/start/SKILL.md` — 主要変更（情報提示 → ワークフロー基盤構築に拡張）
- `prompts/plans/858_workflow-step-persistence.md` — 計画ファイル
