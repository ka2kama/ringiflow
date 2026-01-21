# 2026-01-16_01: Claude Code Action 権限修正

## 概要

PR #25 の Claude Code Action による自動レビューが失敗していた問題を調査し、権限設定を修正した。

## 背景と目的

PR #25 の CI で「Auto Review」ジョブが失敗していた。原因は Claude Code Action が `gh pr list` コマンドを実行しようとした際に、Bash ツールの権限がなく拒否されたこと。

## 実施内容

### 1. 問題の調査

GitHub Actions のログを確認し、以下の原因を特定：

```json
{
  "type": "result",
  "subtype": "error_max_turns",
  "permission_denials": [
    {
      "tool_name": "Bash",
      "tool_input": {
        "command": "gh pr list --state open --limit 5"
      }
    }
  ]
}
```

- Claude Code Action は**セキュリティ上の理由から Bash ツールをデフォルトで無効化**している
- `gh` コマンドを使うには `--allowedTools` で明示的に許可が必要
- `max-turns: 5` では PR レビューに必要なステップを完了できない

### 2. 解決策の検討

最新の Claude Code Action ドキュメントを調査し、以下を確認：

- `claude_args` の `--allowedTools` フラグで特定のツールを許可可能
- 形式: `Bash(command:*)` でコマンドパターンを指定
- `Bash(gh:*)` のような広範な許可は危険（`gh repo delete` 等も含まれる）

### 3. 権限設定の修正

`.github/workflows/claude-review.yml` を以下のように修正：

**permissions**:
- `actions: read` を追加（CI 結果確認用）

**claude_args**:
- `--max-turns 20` に増加
- `--allowedTools` で必要最小限のコマンドのみ許可：
  - 読み取り: `gh pr view/list/diff/checks`, `git log/diff/show/status`
  - 書き込み: `gh pr comment/review`, `mcp__github_inline_comment__create_inline_comment`

## 成果物

### 変更ファイル

| ファイル | 変更内容 |
|----------|----------|
| `.github/workflows/claude-review.yml` | 権限設定とツール許可を追加 |
| `prompts/runs/2026-01-16_01_ClaudeCodeAction権限修正.md` | セッションログ |
| `docs/05_技術ノート/ClaudeCodeAction_権限設定.md` | 技術ノート |

## 議論の経緯

### 権限設定の調査

OIDC エラーの原因を調査する中で、claude-review.yml 側の権限が原因ではないかという指摘があった。また、`claude_args` の `allowedTools` の設定についても確認があった。

### セキュリティの懸念

広範な権限を付与して大丈夫かという懸念があり、必要最小限の権限に絞る方針で設定を見直した。レビュー結果をコメントする権限や、インラインコメント作成の権限についても確認があった。

### 設定の調整

`max-turns` が少なすぎないかという確認があり、30 に増加した。

## 学んだこと

1. **Claude Code Action のセキュリティモデル**: Bash ツールはデフォルトで無効。CI 環境では `.claude/settings.json` の設定だけでは不十分で、`claude_args` での明示的許可が必要

2. **最小権限の原則**: `Bash(gh:*)` のような広範な許可は危険。レビュー用途では読み取り操作と必要な書き込み操作のみに限定すべき

3. **適切なターン数**: PR レビューには複数ステップ（PR 確認 → コード読み → 分析 → コメント投稿）が必要。5 ターンでは不足、20 ターン程度が妥当

## 次のステップ

- この変更をコミットし、PR #25 に適用
- レビューが正常に動作することを確認
