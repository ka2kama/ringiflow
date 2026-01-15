# sqlx-cli（エスキューエルエックス・シーエルアイ）

## 概要

sqlx-cli は、Rust 用非同期 SQL ツールキット [SQLx](https://github.com/launchbadge/sqlx) のコマンドラインツール。
データベースのマイグレーション管理と、コンパイル時クエリ検証のためのキャッシュ生成を行う。

## インストール

```bash
# PostgreSQL + rustls（TLS）のみを有効化（軽量）
cargo install sqlx-cli --no-default-features --features postgres,rustls

# 確認
sqlx --version
```

### 機能フラグ

| フラグ | 説明 |
|--------|------|
| `postgres` | PostgreSQL サポート |
| `mysql` | MySQL サポート |
| `sqlite` | SQLite サポート |
| `rustls` | Rust 製 TLS（推奨） |
| `native-tls` | OS ネイティブ TLS |

## 主なコマンド

### データベース操作

| コマンド | 説明 |
|---------|------|
| `sqlx database create` | DATABASE_URL のデータベースを作成 |
| `sqlx database drop` | データベースを削除 |
| `sqlx database reset` | drop → create → migrate run |

### マイグレーション

| コマンド | 説明 |
|---------|------|
| `sqlx migrate add <name>` | 新しいマイグレーションファイルを作成 |
| `sqlx migrate run` | 未適用のマイグレーションを実行 |
| `sqlx migrate revert` | 最後のマイグレーションを巻き戻す |
| `sqlx migrate info` | マイグレーション状態を表示 |

### オフラインモード

| コマンド | 説明 |
|---------|------|
| `sqlx prepare` | クエリキャッシュを生成（`.sqlx/` ディレクトリ） |
| `sqlx prepare --check` | キャッシュが最新か検証（CI 用） |

## 使い方

### 1. 環境変数の設定

```bash
# .env ファイルに設定
DATABASE_URL=postgres://user:password@localhost:15432/ringiflow
```

### 2. マイグレーションの作成

```bash
cd apps/api

# マイグレーションファイルを作成
sqlx migrate add create_users_table

# 生成されるファイル:
# migrations/
#   └── 20260114120000_create_users_table.sql
```

### 3. マイグレーションの記述

```sql
-- migrations/20260114120000_create_users_table.sql

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

### 4. マイグレーションの実行

```bash
sqlx migrate run
# Applied 20260114120000_create_users_table (32.123ms)
```

### 5. オフラインモード（CI 用）

SQLx はデフォルトでコンパイル時に DB 接続してクエリを検証する。
CI など DB がない環境では、事前生成したキャッシュを使う。

```bash
# 開発時: キャッシュを生成
sqlx prepare

# CI: キャッシュを使ってビルド
SQLX_OFFLINE=true cargo build
```

## マイグレーションファイルの規約

### ファイル名形式

```
{タイムスタンプ}_{説明}.sql
```

例:
- `20260114120000_create_users_table.sql`
- `20260115100000_add_tenant_id_to_users.sql`

### Up/Down マイグレーション

デフォルトは単一ファイル（up のみ）。
可逆マイグレーションが必要な場合:

```bash
sqlx migrate add -r create_users_table
# 生成:
#   20260114120000_create_users_table.up.sql
#   20260114120000_create_users_table.down.sql
```

## プロジェクトでの使用

### ディレクトリ構成

```
apps/api/
├── migrations/           # マイグレーションファイル
│   └── *.sql
├── .sqlx/                # オフラインキャッシュ（git 管理）
│   └── query-*.json
└── .env                  # DATABASE_URL
```

### justfile タスク

```bash
just setup-db    # sqlx migrate run を実行
```

## トラブルシューティング

### "database does not exist"

```bash
sqlx database create
```

### "error communicating with database"

1. PostgreSQL が起動しているか確認
2. DATABASE_URL が正しいか確認
3. ネットワーク/ファイアウォール設定を確認

### "migrations directory not found"

```bash
# migrations ディレクトリがあるディレクトリで実行
cd apps/api
sqlx migrate run
```

## 関連リソース

- [SQLx 公式リポジトリ](https://github.com/launchbadge/sqlx)
- [SQLx ドキュメント](https://docs.rs/sqlx/latest/sqlx/)
- [ADR-005: データベースツールキットの選定](../04_ADR/005_データベースツールキットの選定.md)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-14 | 初版作成 |
