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

### 開発環境（dev-deps）

| オフセット | PostgreSQL | Redis | DynamoDB | BFF | Core Service | Auth Service | Vite |
|-----------|------------|-------|----------|-----|-------------|-------------|------|
| 0（main） | 15432 | 16379 | 18000 | 13000 | 13001 | 13002 | 15173 |
| 1 | 15532 | 16479 | 18100 | 13100 | 13101 | 13102 | 15273 |
| 2 | 15632 | 16579 | 18200 | 13200 | 13201 | 13202 | 15373 |
| ... | ... | ... | ... | ... | ... | ... | ... |

### API テスト / E2E テスト環境（api-test-deps）

| オフセット | PostgreSQL | Redis | DynamoDB | BFF | Core Service | Auth Service | E2E Vite |
|-----------|------------|-------|----------|-----|-------------|-------------|----------|
| 0（main） | 15433 | 16380 | 18001 | 14000 | 14001 | 14002 | 15174 |
| 1 | 15533 | 16480 | 18101 | 14100 | 14101 | 14102 | 15274 |
| 2 | 15633 | 16580 | 18201 | 14200 | 14201 | 14202 | 15374 |
| ... | ... | ... | ... | ... | ... | ... | ... |

開発環境とテスト環境のポートはオフセット内で衝突しないよう設計されている（インフラは +1、サービスは +1000）。

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

`worktree-add` 実行時に依存サービスの起動・DB マイグレーション・依存関係インストールが自動で行われるため、追加の手順は不要。

```bash
cd ../ringiflow-auth

# サーバーを起動
just dev-bff    # ポート 13100
just dev-web    # ポート 15273
```

セットアップをスキップしたい場合（例: ブランチ作成のみ）:
```bash
just worktree-add auth feature/auth --no-setup

# 後からセットアップを実行
cd ../ringiflow-auth
just setup-worktree
```

注意: worktree は独立したディレクトリのため、以下が共有されない:
- `node_modules/`: `setup-worktree` で自動インストールされる
- `target/`: 初回ビルドは走るが、sccache によりキャッシュヒットで高速化される
- DB データ: `setup-worktree` で自動マイグレーションされる

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
ringiflow         running  /home/user/ringiflow/infra/docker/docker-compose.yaml
ringiflow-auth    running  /home/user/ringiflow-auth/infra/docker/docker-compose.yaml
```

### worktree を削除する

```bash
# メインworktree で実行
just worktree-remove auth
```

Docker コンテナ・ボリュームも一緒に削除される。

## テスト実行

API テスト・E2E テストもポートオフセットが適用されるため、worktree 間で独立して実行できる。

```bash
# API テスト（hurl）
just test-api

# E2E テスト（Playwright）
just test-e2e
```

テスト環境は `backend/.env.api-test` からポート番号を読み込む。このファイルは `generate-env.sh` により、開発環境と同じオフセットで動的生成される。

注意:
- テスト実行前に `just api-test-deps` で Docker コンテナを起動する
- 初回は `just api-test-reset-db` で DB マイグレーションを適用する
- テスト環境の Docker プロジェクト名は `<ディレクトリ名>-api-test`（例: `ringiflow-api-test`, `ringiflow-auth-api-test`）

## 注意事項

### worktree の最大数

ポートオフセットは 1-9 の範囲で自動割り当てされるため、
同時に作成できる worktree は最大 9 個まで。

### ブランチの競合

1つのブランチは1つの worktree にしか割り当てられない。
同じブランチで複数の worktree を作成しようとするとエラーになる。

### cargo build のキャッシュ

`target/` ディレクトリは worktree ごとに独立しているが、sccache によりコンパイル結果がキャッシュされるため、2回目以降のビルドは高速に完了する。

```bash
# キャッシュ統計を確認
sccache --show-stats
```

注意: `CARGO_TARGET_DIR` を共有する方法は非推奨。並行ビルドでロック競合やキャッシュスラッシングが起きるため、sccache の方が安全。

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
docker compose -p ringiflow-auth -f infra/docker/docker-compose.yaml down -v
```

## 参考

- [git worktree 公式ドキュメント](https://git-scm.com/docs/git-worktree)
- Docker Compose のプロジェクト名機能: `docker compose -p PROJECT_NAME`
