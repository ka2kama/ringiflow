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

## マイグレーションの仕組み

### 冪等性（Idempotency）

`sqlx migrate run` は冪等な操作であり、何度実行しても安全。

仕組み:

1. 初回実行時、SQLx は `_sqlx_migrations` テーブルを作成
2. マイグレーション適用時、このテーブルに記録を追加
3. 再実行時、既に記録があるマイグレーションはスキップ

```
初回実行:     001_create_users → 適用、記録
2回目実行:    001_create_users → スキップ（記録済み）
新規追加後:   001_create_users → スキップ、002_add_column → 適用、記録
```

### _sqlx_migrations テーブル

```sql
-- SQLx が自動作成するテーブル
SELECT * FROM _sqlx_migrations;

--  version |      description       |    installed_on     | success | checksum | execution_time
-- ---------+------------------------+---------------------+---------+----------+----------------
--        1 | create_users_table     | 2026-01-14 12:00:00 | t       | \x...    | 32123456
```

| カラム | 説明 |
|--------|------|
| `version` | マイグレーション番号（タイムスタンプ） |
| `description` | マイグレーション名 |
| `installed_on` | 適用日時 |
| `success` | 成功したか |
| `checksum` | ファイルのハッシュ値 |
| `execution_time` | 実行時間（ナノ秒） |

### 注意点

- 適用済みマイグレーションは編集しない — checksum が変わるとエラーになる
- 本番では revert を慎重に — データが失われる可能性がある
- マイグレーションの順序は変えない — version 順に実行される

## 使い方

### 1. 環境変数の設定

```bash
# .env ファイルに設定
DATABASE_URL=postgres://user:password@localhost:15432/ringiflow
```

### 2. マイグレーションの作成

```bash
# migrations/ ディレクトリがある場所で実行
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
cargo sqlx prepare --workspace

# CI: キャッシュを使ってビルド
SQLX_OFFLINE=true cargo build
```

## オフラインモード詳細

### 仕組み

`sqlx::query!` マクロはコンパイル時に以下を行う：

1. **通常モード**: DB に接続し、SQL の構文・型を検証
2. **オフラインモード（`SQLX_OFFLINE=true`）**: `.sqlx/` ディレクトリのキャッシュを参照

```
開発環境                               CI 環境
┌──────────────────┐                  ┌──────────────────┐
│ cargo build      │                  │ cargo build      │
│     ↓            │                  │     ↓            │
│ sqlx::query!     │                  │ sqlx::query!     │
│     ↓            │                  │     ↓            │
│ DB に接続して検証 │                  │ .sqlx/ を参照    │
└──────────────────┘                  └──────────────────┘
```

### キャッシュファイル

`cargo sqlx prepare` で生成される `.sqlx/` ディレクトリの内容：

```
.sqlx/
├── query-{hash1}.json   # クエリ1のメタデータ
├── query-{hash2}.json   # クエリ2のメタデータ
└── ...
```

各 JSON ファイルには以下が含まれる：
- SQL クエリ文字列
- 引数の型情報
- 戻り値のカラム情報

### 必須運用ルール

**SQL クエリを追加・変更したら必ず `cargo sqlx prepare` を実行する。**

```bash
# DB を起動した状態で
just setup-db

# キャッシュを再生成（テストコード含む）
cd backend && cargo sqlx prepare --workspace -- --all-targets

# コミットに含める
git add backend/.sqlx/
git commit -m "SQLx オフラインキャッシュを更新"
```

重要: `--all-targets` オプションを忘れると、テストコード内の `sqlx::query!` がキャッシュされず、CI でテストビルドが失敗する。

### よくあるミス

| 症状 | 原因 | 対処 |
|------|------|------|
| CI で `SQLX_OFFLINE=true but there is no cached data` | 新しいクエリのキャッシュがない | `cargo sqlx prepare` を実行 |
| CI でのみビルドが失敗する | `.sqlx/` をコミットし忘れ | キャッシュをコミットに含める |
| キャッシュ生成時にエラー | DB が起動していない | `just setup-db` で DB を起動 |

### CI での設定

```yaml
# .github/workflows/ci.yml
env:
  SQLX_OFFLINE: true  # オフラインモードを有効化
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

`migrations/` ディレクトリが存在するディレクトリで `sqlx migrate` コマンドを実行する必要がある。

## 関連リソース

- [SQLx 公式リポジトリ](https://github.com/launchbadge/sqlx)
- [SQLx ドキュメント](https://docs.rs/sqlx/latest/sqlx/)
- [ADR-005: データベースツールキットの選定](../05_ADR/005_データベースツールキットの選定.md)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-14 | 初版作成 |
| 2026-01-16 | マイグレーションの仕組み（冪等性）セクションを追加、プロジェクト固有の記述を削除 |
| 2026-01-17 | オフラインモード詳細セクションを追加 |
