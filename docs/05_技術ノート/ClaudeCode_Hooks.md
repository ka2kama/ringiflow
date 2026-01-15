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
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo 'セッション終了'"
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

## 注意点

- セッション中に設定を変更しても、次回起動まで反映されない
- Stop はユーザーが `Ctrl+C` で中断した場合は発火しない
- SessionEnd は `/exit` や `Ctrl+D` での正常終了時に発火する
