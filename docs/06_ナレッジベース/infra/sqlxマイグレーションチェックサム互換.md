# sqlx マイグレーションチェックサム互換

## 概要

sqlx は `_sqlx_migrations` テーブルでマイグレーションの適用状態を管理し、各マイグレーションファイルの SHA-384 チェックサムを記録する。sqlx-cli が使えない環境（低メモリ環境等）で psql を使ってマイグレーションを適用する場合、このチェックサムを正しく記録しないとアプリ起動時に `VersionMismatch` エラーが発生する。

## sqlx のマイグレーション管理

### `_sqlx_migrations` テーブル

```sql
CREATE TABLE _sqlx_migrations (
    version BIGINT PRIMARY KEY,        -- ファイル名の数値プレフィックス
    description TEXT NOT NULL,         -- ファイル名のアンダースコア以降（拡張子なし）
    installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,           -- SHA-384 ハッシュ（48バイト）
    execution_time BIGINT NOT NULL     -- 実行時間（ナノ秒）
);
```

### チェックサムの計算

sqlx は内部で `sha2::Sha384` を使い、マイグレーションファイルの内容全体をハッシュする。

シェルで同等のチェックサムを計算する方法:

```bash
sha384sum 20260115000001_create_tenants.sql | cut -d' ' -f1
```

PostgreSQL の `BYTEA` 型に格納する際は hex エンコードで:

```sql
INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
VALUES (20260115000001, 'create_tenants', true, '\x<sha384hex>', 0);
```

### 検証

ローカル開発 DB の `_sqlx_migrations` テーブルの値と `sha384sum` コマンドの出力が一致することを確認済み。

## psql によるマイグレーション適用手順

sqlx-cli が使えない環境での手順:

1. `_sqlx_migrations` テーブルを作成
2. マイグレーションファイルをファイル名順に `psql` で適用
3. 各ファイルの SHA-384 チェックサムを計算して `_sqlx_migrations` に記録

```bash
for migration_file in $(find "$MIGRATIONS_DIR" -name '*.sql' | sort); do
    filename=$(basename "$migration_file")
    version=$(echo "$filename" | grep -oP '^\d+')
    description=$(echo "$filename" | sed -E 's/^[0-9]+_//' | sed 's/\.sql$//')
    checksum_hex=$(sha384sum "$migration_file" | cut -d' ' -f1)

    psql -U "$USER" -d "$DB" < "$migration_file"
    psql -U "$USER" -d "$DB" -c "
        INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
        VALUES ($version, '$description', true, '\\x${checksum_hex}', 0);
    "
done
```

## sqlx-cli が使えないケース

| 環境 | 理由 |
|------|------|
| 低メモリ環境（1GB 以下） | `cargo install sqlx-cli` の Rust コンパイルで OOM |
| Rust ツールチェーンなし環境 | psql のみ利用可能 |

## プロジェクトでの使用箇所

- `infra/lightsail/reset.sh`: Lightsail デモ環境（1GB RAM）でのデータリセット

## 関連リソース

- [sqlx ソースコード（migrate モジュール）](https://github.com/launchbadge/sqlx/tree/main/sqlx-core/src/migrate)
- [sqlx-cli ドキュメント](https://docs.rs/sqlx-cli/)
- セッションログ: [Lightsail デモ環境リセットスクリプト](../../prompts/runs/2026-02/2026-02-23_2108_Lightsailデモ環境リセットスクリプト.md)
