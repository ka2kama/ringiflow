# worktree 環境でのブランチ作成

## 状況

Git worktree 環境で新しいブランチを作成したいが、`main` ブランチが他の worktree で使用中のためチェックアウトできない。

```bash
$ git checkout main
fatal: 'main' is already used by worktree at '/path/to/other/worktree'
```

## 解決策

`origin/main` から直接ブランチを作成する。

```bash
git fetch origin
git checkout -b <branch-name> origin/main
```

## 例

```bash
# origin から最新を取得
git fetch origin

# origin/main から新しいブランチを作成
git checkout -b chore/ai-agent-improvements origin/main
```

## 補足

- worktree 環境では、同じブランチを複数の worktree でチェックアウトできない
- `origin/main` はリモート追跡ブランチなので、この制限を受けない
- 作成したブランチは自動的に `origin/main` を追跡する
