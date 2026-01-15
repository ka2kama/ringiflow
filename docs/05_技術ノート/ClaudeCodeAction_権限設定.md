# Claude Code Action 権限設定

Claude Code Action を GitHub Actions で使用する際の権限設定についてまとめる。

## セキュリティモデル

Claude Code Action は**セキュリティ上の理由から Bash ツールをデフォルトで無効化**している。

- ファイル操作やコメント管理はデフォルトで使用可能
- Bash コマンド（`gh`, `git` など）は明示的な許可が必要
- `.claude/settings.json` の設定は CI 環境では効かない
- ワークフローファイルの `claude_args` で `--allowedTools` を指定する必要がある

## 基本設定

### GitHub トークン権限

```yaml
permissions:
  contents: read       # リポジトリ読み取り
  pull-requests: write # PR へのコメント
  issues: write        # Issue へのコメント
  actions: read        # CI 結果の確認（オプション）
  id-token: write      # OIDC 認証（OAuth 使用時に必要）
```

### ツール許可の構文

```yaml
claude_args: |
  --max-turns 20
  --allowedTools "Bash(gh pr view:*),Bash(git log:*)"
```

**形式**:
- `Bash(command:*)`: 特定コマンドのすべてのサブコマンド・引数を許可
- `Bash(command subcommand:*)`: より限定的な許可
- `mcp__server__tool`: MCP サーバーのツール

## PR レビュー用の推奨設定

### 読み取り操作

```
Bash(gh pr view:*)     # PR 詳細
Bash(gh pr list:*)     # PR 一覧
Bash(gh pr diff:*)     # PR 差分
Bash(gh pr checks:*)   # CI 状態
Bash(git log:*)        # コミット履歴
Bash(git diff:*)       # 差分
Bash(git show:*)       # コミット内容
Bash(git status:*)     # 状態
```

### 書き込み操作

```
Bash(gh pr comment:*)  # PR コメント
Bash(gh pr review:*)   # PR レビュー
mcp__github_inline_comment__create_inline_comment  # インラインコメント
```

### 完全な設定例

```yaml
- uses: anthropics/claude-code-action@v1
  with:
    claude_code_oauth_token: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
    prompt: |
      この PR をレビューしてください。
    claude_args: |
      --max-turns 20
      --allowedTools "Bash(gh pr view:*),Bash(gh pr list:*),Bash(gh pr diff:*),Bash(gh pr checks:*),Bash(gh pr comment:*),Bash(gh pr review:*),Bash(git log:*),Bash(git diff:*),Bash(git show:*),Bash(git status:*),mcp__github_inline_comment__create_inline_comment"
```

## 注意事項

### 広範な権限は避ける

```yaml
# NG: 危険なコマンドも含まれる
--allowedTools "Bash(gh:*)"   # gh repo delete 等も許可される
--allowedTools "Bash(git:*)"  # git push --force 等も許可される

# OK: 必要な操作のみ許可
--allowedTools "Bash(gh pr view:*),Bash(gh pr list:*)"
```

### max-turns の設定

PR レビューには複数ステップが必要：
1. PR 情報取得
2. 差分確認
3. コード読み込み
4. 分析
5. コメント投稿

5 ターンでは不足する場合が多い。20 ターン程度を推奨。

### MCP ツールの許可

MCP サーバーのツールは `mcp__<server>__<tool>` 形式で指定：

```yaml
--allowedTools "mcp__github_inline_comment__create_inline_comment"
```

## トラブルシューティング

### permission_denials エラー

```json
{
  "permission_denials": [{
    "tool_name": "Bash",
    "tool_input": { "command": "gh pr list" }
  }]
}
```

**原因**: `--allowedTools` で許可されていないコマンドを実行しようとした

**解決**: 必要なコマンドを `--allowedTools` に追加

### error_max_turns エラー

```json
{
  "subtype": "error_max_turns",
  "num_turns": 5
}
```

**原因**: 設定されたターン数内で処理が完了しなかった

**解決**: `--max-turns` を増やす

## 参考リンク

- [Claude Code Action リポジトリ](https://github.com/anthropics/claude-code-action)
- [Claude Code GitHub Actions ドキュメント](https://code.claude.com/docs/en/github-actions)
