# Phase 4: レガシーカラム削除

## 概要

Auth Service 分離の最終フェーズ。`users.password_hash` カラムを削除し、認証情報の管理を完全に Auth Service（`auth.credentials` テーブル）に移行する。

## 変更内容

### 1. マイグレーション

`20260122000003_drop_users_password_hash.sql`:

```sql
ALTER TABLE users DROP COLUMN password_hash;
```

Phase 2 のマイグレーション（`20260122000002_migrate_password_to_credentials.sql`）でパスワードハッシュは `auth.credentials` テーブルに移行済みのため、安全に削除可能。

### 2. ドメイン層の変更

`User` エンティティから `password_hash` フィールドを削除:

- `User::new()`: `password_hash` 引数を削除
- `User::from_db()`: `password_hash` 引数を削除
- `password_hash()` getter: 削除
- `with_password_hash()`: 削除

### 3. インフラ層の変更

`PostgresUserRepository` のクエリから `password_hash` カラムを削除:

- `find_by_email()`: SELECT 句から削除
- `find_by_id()`: SELECT 句から削除

### 4. テストの更新

- `user_repository_test.rs`: INSERT 文から `password_hash` を削除
- `core-api/handler/auth.rs`: テスト用の `User::from_db()` 呼び出しを更新

## 設計判断

### カラム削除のタイミング

Phase 3 で BFF 統合が完了し、認証フローが Auth Service 経由に切り替わった後に削除を実施。この順序により:

1. 既存の認証フローへの影響を最小化
2. ロールバック時の安全性を確保
3. 段階的な移行による検証が可能

### データ移行の一貫性

Phase 2 でパスワードハッシュを `auth.credentials` にコピー済み。元データ（`users.password_hash`）は Phase 3 完了後の Phase 4 まで保持し、万が一の問題発生時に備えた。

## 関連ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/migrations/20260122000003_drop_users_password_hash.sql` | マイグレーション |
| `backend/crates/domain/src/user.rs` | `password_hash` 関連の削除 |
| `backend/crates/infra/src/repository/user_repository.rs` | SQL クエリ更新 |
| `backend/crates/infra/tests/user_repository_test.rs` | テスト更新 |
| `backend/apps/core-api/src/handler/auth.rs` | テスト更新 |
