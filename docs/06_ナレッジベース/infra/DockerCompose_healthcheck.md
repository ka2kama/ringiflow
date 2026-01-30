# Docker Compose の healthcheck と --wait

## 概要

Docker Compose でサービスの起動完了を確実に待機するための仕組み。
`docker compose up -d --wait` は各サービスの healthcheck が通るまでブロックする。

## なぜ必要か

### sleep の問題

```bash
# 悪い例: 固定時間の待機
docker compose up -d
sleep 5
sqlx migrate run
```

問題点:
- マシン負荷やネットワーク状況で待機時間が不足/過剰になる
- 「ポートが開いている」と「サービスが使用可能」は異なる
- PostgreSQL は起動直後に接続を受け付けても、初期化が完了していないことがある

### healthcheck の利点

```bash
# 良い例: healthcheck による待機
docker compose up -d --wait
sqlx migrate run
```

- サービスが「本当に使用可能か」を検証
- 条件が満たされた瞬間に次へ進む（無駄な待機がない）
- 条件が満たされなければタイムアウトでエラー（問題の早期検出）

## 使い方

### 1. docker-compose.yaml で healthcheck を定義

```yaml
services:
  postgres:
    image: postgres:17
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s      # チェック間隔
      timeout: 5s       # タイムアウト
      retries: 5        # リトライ回数
      start_period: 10s # 起動猶予期間
```

### 2. --wait フラグで起動

```bash
docker compose up -d --wait
```

このコマンドは:
1. 全サービスをバックグラウンドで起動
2. 各サービスの healthcheck が `healthy` になるまで待機
3. 全サービスが healthy になったら終了

## healthcheck の設定項目

| 項目 | 説明 | デフォルト |
|------|------|-----------|
| `test` | 実行するコマンド | - |
| `interval` | チェック間隔 | 30s |
| `timeout` | コマンドのタイムアウト | 30s |
| `retries` | 失敗許容回数 | 3 |
| `start_period` | 起動猶予期間（この間は失敗をカウントしない） | 0s |

## 代表的なサービスの healthcheck 例

### PostgreSQL

```yaml
healthcheck:
  test: ["CMD-SHELL", "pg_isready -U postgres"]
  interval: 5s
  timeout: 5s
  retries: 5
```

`pg_isready` は PostgreSQL が接続を受け付けるかチェックするユーティリティ。

### Redis

```yaml
healthcheck:
  test: ["CMD", "redis-cli", "ping"]
  interval: 5s
  timeout: 5s
  retries: 5
```

`redis-cli ping` が `PONG` を返せば正常。

### MySQL

```yaml
healthcheck:
  test: ["CMD", "mysqladmin", "ping", "-h", "localhost"]
  interval: 5s
  timeout: 5s
  retries: 5
```

### HTTP サービス

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
  interval: 10s
  timeout: 5s
  retries: 3
```

## depends_on との組み合わせ

`depends_on` だけでは「コンテナが起動した」ことしか保証しない。
`condition: service_healthy` を組み合わせることで、healthcheck ベースの依存関係を定義できる。

```yaml
services:
  app:
    build: .
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy

  postgres:
    image: postgres:17
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5
```

この設定では、`app` は `postgres` と `redis` の両方が healthy になるまで起動しない。

## トラブルシューティング

### healthcheck が常に失敗する

```bash
# コンテナ内でコマンドを直接実行してみる
docker compose exec postgres pg_isready -U postgres
```

### 状態の確認

```bash
# 全サービスの状態を確認
docker compose ps

# 特定サービスの詳細を確認
docker inspect <container_id> --format='{{json .State.Health}}'
```

### ログの確認

```bash
# healthcheck の結果はコンテナのログに出力されないが、
# ヘルスステータスの履歴は inspect で確認できる
docker inspect <container_id> | jq '.[0].State.Health'
```

## 関連リソース

- [Docker Compose: healthcheck](https://docs.docker.com/reference/compose-file/services/#healthcheck)
- [Docker: HEALTHCHECK instruction](https://docs.docker.com/reference/dockerfile/#healthcheck)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-19 | 初版作成 |
