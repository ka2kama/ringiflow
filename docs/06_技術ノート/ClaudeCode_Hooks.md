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
            "command": "if echo \"$TOOL_INPUT\" | jq -r '.command' | grep -q 'git commit'; then echo 'コミット前の確認'; fi"
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
            "command": "cargo fmt --quiet -- \"$file_path\""
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

## 環境変数

フック内で利用できる環境変数:

| 変数 | 内容 |
|------|------|
| `$TOOL_INPUT` | ツールに渡された入力（JSON 形式） |
| `$TOOL_OUTPUT` | ツールの出力（PostToolUse のみ） |

### $TOOL_INPUT の例

**Write ツールの場合:**
```json
{
  "file_path": "/path/to/file.rs",
  "content": "fn main() { ... }"
}
```

**Bash ツールの場合:**
```json
{
  "command": "git commit -m \"message\""
}
```

### jq でのパース例

```bash
# file_path を取得
file_path=$(echo "$TOOL_INPUT" | jq -r '.file_path // empty')

# command を取得
command=$(echo "$TOOL_INPUT" | jq -r '.command // empty')
```

## プロジェクト設定例

### Rust/Elm ファイルの自動フォーマット（PostToolUse）

`.rs` / `.elm` ファイルを Write/Edit した後に自動でフォーマットする。

```json
{
  "PostToolUse": [
    {
      "matcher": "Write|Edit",
      "hooks": [
        {
          "type": "command",
          "command": "file_path=$(echo \"$TOOL_INPUT\" | jq -r '.file_path // empty' 2>/dev/null); if [[ \"$file_path\" == *.rs ]]; then just fmt-rust \"$file_path\" 2>/dev/null || true; elif [[ \"$file_path\" == *.elm ]]; then just fmt-elm \"$file_path\" 2>/dev/null || true; fi"
        }
      ]
    }
  ]
}
```

**コマンドの解説:**
1. `echo "$TOOL_INPUT" | jq -r '.file_path // empty'` - JSON から `file_path` を取得
2. `2>/dev/null` - jq のエラーを抑制
3. `[[ "$file_path" == *.rs ]]` - `.rs` ファイルなら `just fmt-rust`
4. `[[ "$file_path" == *.elm ]]` - `.elm` ファイルなら `just fmt-elm`
5. `|| true` - フォーマット失敗時もエラーにしない

**ポイント:** フォーマット設定を justfile に集約することで、手動実行時と hook 実行時で同じ設定が使われる。

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
          "command": "if echo \"$TOOL_INPUT\" | jq -r '.command // empty' 2>/dev/null | grep -q 'git commit'; then ./.claude/hooks/pre-commit-check.sh || exit 1; fi"
        }
      ]
    }
  ]
}
```

**コマンドの解説:**
1. `jq -r '.command // empty'` - Bash ツールの `command` を取得
2. `grep -q 'git commit'` - `git commit` が含まれているか確認
3. `./.claude/hooks/pre-commit-check.sh` - 外部スクリプトを実行
4. `|| exit 1` - スクリプトが失敗したら hook を失敗させる

## 注意点

- セッション中に設定を変更しても、次回起動まで反映されない
- Stop はユーザーが `Ctrl+C` で中断した場合は発火しない
- SessionEnd は `/exit` や `Ctrl+D` での正常終了時に発火する
- PostToolUse の hook はツール実行後に同期的に実行される
- hook が失敗しても（`|| true` がなければ）ツール実行自体は成功扱いになる
