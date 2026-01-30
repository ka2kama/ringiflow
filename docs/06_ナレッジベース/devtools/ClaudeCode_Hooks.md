# Claude Code Hooks

Claude Code のフック機能。特定のイベント発生時にカスタムコマンドを実行できる。

## イベント一覧

| イベント | 発火タイミング |
|---------|--------------|
| PreToolUse | ツール実行前 |
| PostToolUse | ツール実行後 |
| PostToolUseFailure | ツール実行失敗後 |
| Stop | 応答完了時（毎ターン） |
| SessionStart | セッション開始時 |
| SessionEnd | セッション終了時 |
| SubagentStart | サブエージェント開始時 |
| SubagentStop | サブエージェント終了時 |
| Notification | 通知時 |
| UserPromptSubmit | ユーザー入力送信時 |

## 設定例

`.claude/settings.json` で設定する。

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "input=$(cat); if echo \"$input\" | jq -r '.tool_input.command' | grep -q 'git commit'; then echo 'コミット前の確認'; fi"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "./.claude/hooks/post-write-format.sh"
          }
        ]
      }
    ]
  }
}
```

## フックの種類

### command

シェルコマンドを実行する。

```json
{
  "type": "command",
  "command": "echo 'Hello'",
  "timeout": 30
}
```

### prompt

LLM にプロンプトを評価させる。

```json
{
  "type": "prompt",
  "prompt": "タスクが完了したか確認してください。"
}
```

### agent

エージェントを起動して検証を行う。

```json
{
  "type": "agent",
  "prompt": "テストが通っているか確認してください。"
}
```

## matcher

PreToolUse / PostToolUse で特定のツールにのみフックを適用する。

```json
{
  "matcher": "Write|Edit",
  "hooks": [...]
}
```

正規表現パターンでツール名をマッチさせる。

## 入力データの取得

hook コマンドは stdin から JSON 形式でツール情報を受け取る。

### JSON 構造

```json
{
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "/path/to/file.rs",
    "old_string": "...",
    "new_string": "..."
  }
}
```

| フィールド | 内容 |
|-----------|------|
| `tool_name` | ツール名 |
| `tool_input` | ツールに渡された入力 |

### tool_input の例

**Write ツールの場合:**
```json
{
  "tool_input": {
    "file_path": "/path/to/file.rs",
    "content": "fn main() { ... }"
  }
}
```

**Bash ツールの場合:**
```json
{
  "tool_input": {
    "command": "git commit -m \"message\""
  }
}
```

### jq でのパース

stdin から読み取って jq でパースする。

```bash
# stdin を変数に読み込む
input=$(cat)

# file_path を取得
file_path=$(echo "$input" | jq -r '.tool_input.file_path // empty')

# command を取得
command=$(echo "$input" | jq -r '.tool_input.command // empty')
```

パスは `.tool_input.file_path` のように `.tool_input.*` 形式でアクセスする。

## 外部スクリプト化

hook が複雑になる場合は外部スクリプトに分離することを推奨する。

### メリット

- 可読性向上
- デバッグ容易
- ログ出力可能
- シェル機能をフル活用可能

### スクリプト例

`.claude/hooks/post-write-format.sh`:

```bash
#!/bin/bash
set -euo pipefail

LOG_FILE="/tmp/claude/post-write-format.log"
mkdir -p /tmp/claude

# stdin から file_path を抽出
file_path=$(cat | jq -r '.tool_input.file_path // empty' 2>/dev/null || echo "")
[[ -z "$file_path" ]] && exit 0

case "$file_path" in
    *.rs)
        just fmt-rust "$file_path" >> "$LOG_FILE" 2>&1 \
            && echo "[$(date '+%H:%M:%S')] fmt-rust: $file_path" >> "$LOG_FILE"
        ;;
    *.elm)
        just fmt-elm "$file_path" >> "$LOG_FILE" 2>&1 \
            && echo "[$(date '+%H:%M:%S')] fmt-elm: $file_path" >> "$LOG_FILE"
        ;;
esac
exit 0
```

settings.json での参照:

```json
{
  "PostToolUse": [
    {
      "matcher": "Write|Edit",
      "hooks": [
        {
          "type": "command",
          "command": "./.claude/hooks/post-write-format.sh"
        }
      ]
    }
  ]
}
```

## プロジェクト設定例

### Rust/Elm ファイルの自動フォーマット（PostToolUse）

`.rs` / `.elm` ファイルを Write/Edit した後に自動でフォーマットする。

外部スクリプト（推奨）:

```json
{
  "PostToolUse": [
    {
      "matcher": "Write|Edit",
      "hooks": [
        {
          "type": "command",
          "command": "./.claude/hooks/post-write-format.sh"
        }
      ]
    }
  ]
}
```

インライン（簡易版）:

```json
{
  "PostToolUse": [
    {
      "matcher": "Write|Edit",
      "hooks": [
        {
          "type": "command",
          "command": "input=$(cat); file_path=$(echo \"$input\" | jq -r '.tool_input.file_path // empty' 2>/dev/null); if [[ \"$file_path\" == *.rs ]]; then just fmt-rust \"$file_path\" 2>/dev/null || true; elif [[ \"$file_path\" == *.elm ]]; then just fmt-elm \"$file_path\" 2>/dev/null || true; fi"
        }
      ]
    }
  ]
}
```

### コミット前の lint 実行（PreToolUse）

`git commit` 前にステージされたファイルに応じて lint/test を実行する。

```json
{
  "PreToolUse": [
    {
      "matcher": "Bash",
      "hooks": [
        {
          "type": "command",
          "command": "input=$(cat); if echo \"$input\" | jq -r '.tool_input.command // empty' 2>/dev/null | grep -q 'git commit'; then ./.claude/hooks/pre-commit-check.sh || exit 1; fi"
        }
      ]
    }
  ]
}
```

**コマンドの解説:**
1. `input=$(cat)` - stdin から JSON を読み込む
2. `jq -r '.tool_input.command // empty'` - Bash ツールの `command` を取得
3. `grep -q 'git commit'` - `git commit` が含まれているか確認
4. `./.claude/hooks/pre-commit-check.sh` - 外部スクリプトを実行
5. `|| exit 1` - スクリプトが失敗したら hook を失敗させる

## 注意点

- セッション中に設定を変更しても、次回起動まで反映されない
- Stop はユーザーが `Ctrl+C` で中断した場合は発火しない
- SessionEnd は `/exit` や `Ctrl+D` での正常終了時に発火する
- PostToolUse の hook はツール実行後に同期的に実行される
- hook が失敗しても（`|| true` がなければ）ツール実行自体は成功扱いになる
- hook の stdout は通常表示されない（verbose モードでのみ表示）

## デバッグ方法

### ログファイル出力

hook 内でログファイルに出力することで動作を確認できる。

```bash
LOG_FILE="/tmp/claude/hook-debug.log"
mkdir -p /tmp/claude
echo "[$(date)] Hook executed" >> "$LOG_FILE"
```

### 確認方法

```bash
cat /tmp/claude/hook-debug.log
```

### verbose モード

`Ctrl+O` で verbose モードを有効にすると、hook の stdout が表示される。
