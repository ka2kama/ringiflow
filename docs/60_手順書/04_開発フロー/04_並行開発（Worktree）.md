# 並行開発（Worktree）

複数のタスクを同時に進めるための手順。

## 概要

永続スロット方式の git worktree で、複数の独立した作業ディレクトリを維持する。
各スロットは独自の Docker コンテナ・ボリューム・ポートを持ち、互いに干渉しない。

ユースケース:
- 複数のターミナル/IDE で異なるブランチを同時に開発
- AI エージェント（Claude Code 等）を複数並行して動かす
- 機能開発中にホットフィックスが必要になった場合

```
~/ghq/github.com/ka2kama/
├── ringiflow/              # メインworktree（main ブランチ）
│   └── .env               # ポート: 15432, 16379, 13000...
├── ringiflow-1/            # スロット 1（feature/auth ブランチ）
│   ├── .env               # ポート: 15532, 16479, 13100...
│   └── .worktree-slot     # マーカーファイル（内容: "1"）
└── ringiflow-2/            # スロット 2（fix/issue-123 ブランチ）
    ├── .env               # ポート: 15632, 16579, 13200...
    └── .worktree-slot     # マーカーファイル（内容: "2"）
```

### 永続スロットの特長

- スロットは一度作成したら削除しない（ディスク容量が問題になったときのみ）
- `node_modules/`、`target/`（sccache あり）、Docker ボリュームが永続化
- Issue 着手時はスロット内でブランチを切り替えるだけ

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

スロット番号がそのままポートオフセットになる（決定的マッピング）。

## 手順

### スロットを作成する（初回のみ）

```bash
# メインworktree で実行
just worktree-create 1
```

引数:
- `1`: スロット番号（1-9）。ポートオフセットとしても使用

実行結果:
- `../ringiflow-1/` ディレクトリが作成される（detached HEAD）
- `.env` ファイルが自動生成される（オフセット適用済み）
- Docker コンテナ起動、DB マイグレーション、依存関係インストールが自動実行

### Issue に着手する

```bash
# メインworktree で実行
just worktree-issue 321 1
```

引数:
- `321`: Issue 番号（GitHub Issue のタイトルからブランチ名を自動生成）
- `1`: スロット番号

スロット内で実行する場合、スロット番号は省略可能:
```bash
cd ../ringiflow-1
just worktree-issue 321
```

### ブランチを手動で切り替える

```bash
just worktree-switch 1 feature/321-add-user-auth
```

引数:
- `1`: スロット番号
- `feature/321-add-user-auth`: ブランチ名

切り替え時に自動で行われること:
- DB マイグレーションの実行
- `pnpm-lock.yaml` に差分がある場合、`pnpm install` の実行

切り替えは未コミットの変更がない場合のみ可能。変更がある場合はコミットまたは stash してから切り替える。

### スロットで開発を開始する

```bash
cd ../ringiflow-1

# サーバーを起動
just dev-bff    # ポート 13100
just dev-web    # ポート 15273
```

注意: スロットは独立したディレクトリのため、以下が共有されない:
- `node_modules/`: スロット作成時に自動インストール済み。ブランチ切り替え時に差分があれば自動更新
- `target/`: 初回ビルドは走るが、sccache によりキャッシュヒットで高速化される
- DB データ: スロット作成時に自動マイグレーション済み。ブランチ切り替え時に自動実行

### 並行作業の例

ターミナル 1（メインworktree）:
```bash
cd ~/ghq/github.com/ka2kama/ringiflow
just dev-deps && just dev-bff  # ポート 13000
```

ターミナル 2（スロット 1）:
```bash
cd ~/ghq/github.com/ka2kama/ringiflow-1
just dev-deps && just dev-bff  # ポート 13100
```

各作業環境は異なるディレクトリ・ポート・DB を使用するため、互いに干渉しない。

### スロットの状態を確認する

```bash
just worktree-list
```

出力例:
```
=== Worktree 一覧 ===
/home/user/ringiflow        9639dfd [main]
/home/user/ringiflow-1      a1b2c3d [feature/auth]
/home/user/ringiflow-2      (detached HEAD)

=== Docker プロジェクト一覧 ===
NAME              STATUS   CONFIG FILES
ringiflow         running  /home/user/ringiflow/infra/docker/docker-compose.yaml
ringiflow-1       running  /home/user/ringiflow-1/infra/docker/docker-compose.yaml
```

### マージ後のクリーンアップ

PR がマージされた後:

```bash
just cleanup
```

永続スロットは削除されず、detached HEAD にリセットされる。ローカルブランチのみ削除。

### スロットを削除する（緊急時のみ）

```bash
# メインworktree で実行
just worktree-remove 1
```

Docker コンテナ・ボリュームも一緒に削除される。通常は不要。

## テスト実行

API テスト・E2E テストもポートオフセットが適用されるため、スロット間で独立して実行できる。

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
- テスト環境の Docker プロジェクト名は `<ディレクトリ名>-api-test`（例: `ringiflow-api-test`, `ringiflow-1-api-test`）

## DB マイグレーションの扱い

sqlx は前進のみ（down migration なし）。ブランチ切り替え時の影響:

| 切り替え方向 | 影響 | 対応 |
|-------------|------|------|
| 古い → 新しい | 新しいマイグレーションが適用される | `worktree-switch` が自動実行 |
| 新しい → 古い | 余分なテーブル/カラムが残る | 通常は無害。問題時は `just reset-db` |

## 注意事項

### スロットの最大数

ポートオフセットは 1-9 の範囲で、スロット番号と一致する。
同時に作成できるスロットは最大 9 個まで。

### ブランチの競合

1つのブランチは1つの worktree にしか割り当てられない。
同じブランチで複数のスロットを使用しようとするとエラーになる。

### cargo build のキャッシュ

`target/` ディレクトリはスロットごとに独立しているが、sccache によりコンパイル結果がキャッシュされるため、2回目以降のビルドは高速に完了する。

```bash
# キャッシュ統計を確認
sccache --show-stats
```

注意: `CARGO_TARGET_DIR` を共有する方法は非推奨。並行ビルドでロック競合やキャッシュスラッシングが起きるため、sccache の方が安全。

### DB スキーマの独立

各スロットは独自の PostgreSQL ボリュームを持つため、
マイグレーションはスロットごとに実行する必要がある。
`worktree-switch` が自動で `just db-migrate` を実行する。

## トラブルシューティング

### ポートが使用中と表示される

```bash
# どのプロセスがポートを使っているか確認
lsof -i :15532

# Docker コンテナを確認
docker ps --filter "name=ringiflow"
```

### ブランチ切り替えが失敗する

```bash
# 未コミットの変更を確認
cd ../ringiflow-1
git status

# 変更をコミットまたは stash
git stash
just worktree-switch 1 feature/new-branch
```

### スロットが削除できない

```bash
# 強制削除
git worktree remove ../ringiflow-1 --force

# Docker コンテナを手動で削除
docker compose -p ringiflow-1 -f infra/docker/docker-compose.yaml down -v
```

## 参考

- [git worktree 公式ドキュメント](https://git-scm.com/docs/git-worktree)
- Docker Compose のプロジェクト名機能: `docker compose -p PROJECT_NAME`
