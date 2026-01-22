# 並行開発（Worktree）

複数のタスクを同時に進めるための手順。

## 概要

git worktree を使って、1つのリポジトリから複数の独立した作業ディレクトリを作成する。
各 worktree は独自の Docker コンテナ・ボリューム・ポートを持ち、互いに干渉しない。

ユースケース:
- 複数のターミナル/IDE で異なるブランチを同時に開発
- AI エージェント（Claude Code 等）を複数並行して動かす
- 機能開発中にホットフィックスが必要になった場合

```
~/ghq/github.com/ka2kama/
├── ringiflow/              # メインworktree（main ブランチ）
│   └── .env               # ポート: 15432, 16379, 13000...
├── ringiflow-auth/         # worktree 1（feature/auth ブランチ）
│   └── .env               # ポート: 15532, 16479, 13100...
└── ringiflow-fix-123/      # worktree 2（fix/issue-123 ブランチ）
    └── .env               # ポート: 15632, 16579, 13200...
```

## ポートオフセット表

| オフセット | PostgreSQL | Redis | BFF | Core API | Vite |
|-----------|------------|-------|-----|----------|------|
| 0（main） | 15432 | 16379 | 13000 | 13001 | 15173 |
| 1 | 15532 | 16479 | 13100 | 13101 | 15273 |
| 2 | 15632 | 16579 | 13200 | 13201 | 15373 |
| 3 | 15732 | 16679 | 13300 | 13301 | 15473 |
| ... | ... | ... | ... | ... | ... |

## 手順

### worktree を追加する

```bash
# メインworktree で実行
just worktree-add auth feature/auth
```

引数:
- `auth`: worktree 名（ディレクトリ名の接尾辞）
- `feature/auth`: ブランチ名（存在しなければ新規作成）

ポートオフセットは **自動で空き番号が割り当てられる**（1-9 の範囲）。

実行結果:
- `../ringiflow-auth/` ディレクトリが作成される
- `.env` ファイルが自動生成される（オフセット適用済み）

### worktree で開発を開始する

```bash
cd ../ringiflow-auth

# 初回のみ: 依存関係をインストール
cd frontend && pnpm install && cd ..

# 依存サービスを起動（独自のコンテナ・ボリューム）
just dev-deps

# DB マイグレーションを適用（初回のみ）
just setup-db

# サーバーを起動
just dev-bff    # ポート 13100（初回は cargo build が走る）
just dev-web    # ポート 15273
```

注意: worktree は独立したディレクトリのため、以下が共有されない:
- `node_modules/`: 初回に `pnpm install` が必要
- `target/`: 初回に Rust のビルドが走る（数分かかる）
- DB データ: 初回に `just setup-db` でマイグレーションが必要

### 並行作業の例

ターミナル 1（メインworktree）:
```bash
cd ~/ghq/github.com/ka2kama/ringiflow
just dev-deps && just dev-bff  # ポート 13000
```

ターミナル 2（worktree-auth）:
```bash
cd ~/ghq/github.com/ka2kama/ringiflow-auth
just dev-deps && just dev-bff  # ポート 13100
```

各作業環境は異なるディレクトリ・ポート・DB を使用するため、互いに干渉しない。

### worktree の状態を確認する

```bash
just worktree-list
```

出力例:
```
=== Worktree 一覧 ===
/home/user/ringiflow        9639dfd [main]
/home/user/ringiflow-auth   a1b2c3d [feature/auth]

=== Docker プロジェクト一覧 ===
NAME              STATUS   CONFIG FILES
ringiflow         running  /home/user/ringiflow/infra/docker/docker-compose.yml
ringiflow-auth    running  /home/user/ringiflow-auth/infra/docker/docker-compose.yml
```

### worktree を削除する

```bash
# メインworktree で実行
just worktree-remove auth
```

Docker コンテナ・ボリュームも一緒に削除される。

## 注意事項

### worktree の最大数

ポートオフセットは 1-9 の範囲で自動割り当てされるため、
同時に作成できる worktree は最大 9 個まで。

### ブランチの競合

1つのブランチは1つの worktree にしか割り当てられない。
同じブランチで複数の worktree を作成しようとするとエラーになる。

### cargo build のキャッシュ

`target/` ディレクトリは worktree ごとに独立しているため、
初回ビルドは時間がかかる。

キャッシュを共有したい場合は、環境変数 `CARGO_TARGET_DIR` を設定する:
```bash
export CARGO_TARGET_DIR=~/.cargo/shared-target/ringiflow
```

### DB スキーマの独立

各 worktree は独自の PostgreSQL ボリュームを持つため、
マイグレーションは worktree ごとに実行する必要がある。

```bash
cd ../ringiflow-auth
just setup-db
```

## トラブルシューティング

### ポートが使用中と表示される

```bash
# どのプロセスがポートを使っているか確認
lsof -i :15532

# Docker コンテナを確認
docker ps --filter "name=ringiflow"
```

### worktree が削除できない

```bash
# 強制削除
git worktree remove ../ringiflow-auth --force

# Docker コンテナを手動で削除
docker compose -p ringiflow-auth -f infra/docker/docker-compose.yml down -v
```

## 参考

- [git worktree 公式ドキュメント](https://git-scm.com/docs/git-worktree)
- Docker Compose のプロジェクト名機能: `docker compose -p PROJECT_NAME`
