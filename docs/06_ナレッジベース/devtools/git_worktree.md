# git worktree

git worktree は、1つのリポジトリから複数の作業ディレクトリを作成する Git の公式機能。

## 基本概念

通常、Git リポジトリには1つの作業ディレクトリしかない。
worktree を使うと、同じリポジトリから複数の作業ディレクトリを持てる。

```
.git/                    # 共有される Git データベース
├── worktrees/
│   ├── ringiflow-auth/  # worktree のメタデータ
│   └── ringiflow-fix/
│
~/projects/
├── ringiflow/           # メイン worktree（main ブランチ）
├── ringiflow-auth/      # 追加 worktree（feature/auth ブランチ）
└── ringiflow-fix/       # 追加 worktree（fix/issue-123 ブランチ）
```

各 worktree は独立した作業ディレクトリだが、`.git` データベースは共有される。

## 基本コマンド

### worktree の追加

```bash
# 既存ブランチを指定
git worktree add ../ringiflow-auth feature/auth

# 新規ブランチを作成しながら追加
git worktree add -b feature/new ../ringiflow-new
```

### worktree の一覧

```bash
git worktree list
```

出力例:
```
/home/user/ringiflow        9639dfd [main]
/home/user/ringiflow-auth   a1b2c3d [feature/auth]
```

### worktree の削除

```bash
git worktree remove ../ringiflow-auth

# 強制削除（未コミットの変更がある場合）
git worktree remove ../ringiflow-auth --force
```

## 制約

### 1ブランチ1worktree の原則

同じブランチを複数の worktree でチェックアウトすることはできない。

```bash
# 既に feature/auth が他の worktree でチェックアウトされていると失敗
git worktree add ../another feature/auth
# fatal: 'feature/auth' is already checked out at '/home/user/ringiflow-auth'
```

### メイン worktree は削除できない

最初の worktree（通常はクローン時に作成されたもの）は削除できない。

## Docker Compose との組み合わせ

worktree だけでは Docker コンテナは分離されない。
Docker Compose のプロジェクト名機能と組み合わせることで、コンテナ・ボリューム・ネットワークを分離できる。

```bash
# プロジェクト名を指定して起動
docker compose -p ringiflow-auth up -d
```

これにより:
- コンテナ名: `ringiflow-auth-postgres-1`
- ボリューム名: `ringiflow-auth_postgres_data`
- ネットワーク: `ringiflow-auth_default`

が作成され、他の worktree と完全に分離される。

## ポート分離の自動化

Docker コンテナは分離できても、ポートは依然として競合する可能性がある。
これを解決するには:

1. 各 worktree で異なる `.env` ファイルを生成
2. ポートにオフセットを加算（例: main=15432, auth=15532）

このプロジェクトでは `scripts/env/generate.sh` でこれを自動化している。

## ユースケース

### 1. 並行開発

異なる機能を同時に開発する。

```bash
# 機能 A の開発
cd ringiflow-feature-a
just dev-bff  # ポート 13100

# 機能 B の開発（別ターミナル）
cd ringiflow-feature-b
just dev-bff  # ポート 13200
```

### 2. レビュー中の開発継続

PR がレビュー待ちの間に、次の機能を開発する。

```bash
# PR 待ちのブランチ
cd ringiflow
# （何もしない、レビュー待ち）

# 次の機能を開発
cd ringiflow-next
# （開発を継続）
```

### 3. ホットフィックス

機能開発中に緊急の修正が必要になった場合。

```bash
# 機能開発中
cd ringiflow-feature

# 緊急修正（ブランチ切り替え不要）
cd ../ringiflow
git checkout -b hotfix/critical
# 修正 → コミット → PR → マージ

# 機能開発に戻る（何も失われていない）
cd ../ringiflow-feature
```

## 注意点

### ディスク使用量

worktree ごとに以下が独立して存在する:
- `node_modules/`（フロントエンド）
- `target/`（Rust ビルドキャッシュ）

Rust のビルドキャッシュは sccache で worktree 間共有される（`backend/.cargo/config.toml` で設定済み）。`target/` ディレクトリは独立だが、コンパイル済みオブジェクトが sccache のローカルキャッシュからヒットするため、2回目以降のビルドは高速。

```bash
# キャッシュ統計を確認
sccache --show-stats
```

注意: `CARGO_TARGET_DIR` を共有する方法は非推奨。並行ビルドでファイルロック競合やキャッシュスラッシングが起きるため、sccache の方が安全で並行開発に適している。

### クリーンアップ

worktree を削除する際は、関連する Docker コンテナも削除する:

```bash
docker compose -p ringiflow-auth down -v
git worktree remove ../ringiflow-auth
```

このプロジェクトでは `just worktree-remove` でこれを自動化している。

## 参考

- [git-worktree 公式ドキュメント](https://git-scm.com/docs/git-worktree)
- [Pro Git - Git Tools - Worktrees](https://git-scm.com/book/en/v2/Git-Tools-Worktrees)
