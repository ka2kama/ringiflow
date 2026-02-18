# 2026-01-17_13 PostToolUse hooks 改善

## 概要

Claude Code の PostToolUse hooks が動作しない問題を調査・解決し、stdin 入力方式への統一と外部スクリプト化を実施した。

## 変更内容

### 新規作成

- `.claude/hooks/post-write-format.sh` - PostToolUse hook スクリプト

### 変更

- `.claude/settings.json`
  - PreToolUse: `$TOOL_INPUT` 環境変数 → stdin 入力方式（`.tool_input.*`）
  - PostToolUse: インラインコマンド → 外部スクリプト参照
  - sandbox.excludedCommands に `just` を追加
- `docs/06_技術ノート/ClaudeCode_Hooks.md` - stdin 入力方式の解説を追加

## 調査結果

### 問題

PostToolUse hooks が Claude Code から呼ばれていなかった。

### 原因

jq でのパス指定が誤っていた。

- 誤: `.file_path`（トップレベルの file_path を参照）
- 正: `.tool_input.file_path`（tool_input 内の file_path を参照）

stdin から受け取る JSON は以下の構造:

```json
{
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "/path/to/file.rs",
    ...
  }
}
```

### 解決

jq のパスを `.tool_input.*` 形式に修正したところ、hooks が正常に動作することを確認。

## 学んだこと

1. hooks の stdin JSON は `tool_input` でラップされている
2. hooks の stdout は通常表示されない（verbose モードでのみ表示）
3. デバッグにはログファイル出力が有効
4. 複雑な hook は外部スクリプト化することで可読性とデバッグ性が向上

## 関連

- PR: #67
- 技術ノート: [ClaudeCode_Hooks.md](../../../docs/06_ナレッジベース/devtools/ClaudeCode_Hooks.md)
