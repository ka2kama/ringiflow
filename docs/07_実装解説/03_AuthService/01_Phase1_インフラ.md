# Phase 1: インフラ（auth スキーマ・credentials テーブル）

## 概要

Auth Service が使用するデータベーススキーマとテーブルを作成する。

### 対応 Issue

[#80 Auth Service を分離する](https://github.com/ka2kama/ringiflow/issues/80)

## 設計書との対応

- [08_AuthService設計.md - データモデル設計](../../03_詳細設計書/08_AuthService設計.md#データモデル設計)

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`20260122000001_create_auth_schema.sql`](../../../backend/migrations/20260122000001_create_auth_schema.sql) | auth スキーマと credentials テーブルの作成 |
| [`20260122000002_migrate_password_to_credentials.sql`](../../../backend/migrations/20260122000002_migrate_password_to_credentials.sql) | 既存パスワードの移行 |

## 実装内容

### auth スキーマの作成

```sql
CREATE SCHEMA IF NOT EXISTS auth;
```

Core Service（public スキーマ）と Auth Service（auth スキーマ）をスキーマで論理的に分離する。

### credentials テーブル

```sql
CREATE TABLE auth.credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    credential_type VARCHAR(20) NOT NULL,
    credential_data TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_credentials_user_type UNIQUE (user_id, credential_type),
    CONSTRAINT chk_credential_type CHECK (credential_type IN ('password', 'totp', 'oidc', 'saml'))
);
```

### credential_type の種別

| 種別 | 説明 | Phase |
|-----|------|-------|
| `password` | パスワード認証（Argon2id ハッシュ） | Phase 2（実装済み） |
| `totp` | TOTP 認証 | Phase 3（将来） |
| `oidc` | OIDC SSO | Phase 4（将来） |
| `saml` | SAML SSO | Phase 5（将来） |

### パスワードの移行

```sql
INSERT INTO auth.credentials (user_id, tenant_id, credential_type, credential_data, is_active)
SELECT
    id AS user_id,
    tenant_id,
    'password' AS credential_type,
    password_hash AS credential_data,
    true AS is_active
FROM users
WHERE password_hash IS NOT NULL
  AND password_hash != '$INVALID_HASH_PLEASE_SET_PASSWORD$';
```

移行期間中は `users.password_hash` と `auth.credentials` の両方にデータを保持する。

## テスト

マイグレーションの適用確認:

```bash
just reset-db  # データベースをリセットしてマイグレーションを再適用
```

---

## 設計解説

### 1. スキーマ分離

**場所**: [`20260122000001_create_auth_schema.sql:10`](../../../backend/migrations/20260122000001_create_auth_schema.sql)

**コード例**:

```sql
CREATE SCHEMA IF NOT EXISTS auth;
```

**なぜこの設計か**:

Auth Service と Core Service は同一の PostgreSQL データベースに接続しつつ、スキーマで論理的に分離する。これにより:

- 初期段階では運用コストを抑えられる（単一 DB）
- 将来的に DB を物理分離する際は接続文字列の変更のみで対応可能
- 各サービスが所有するテーブルが明確になる

**代替案**:

| 方式 | メリット | デメリット |
|------|---------|-----------|
| 同一スキーマ | シンプル | 所有者が曖昧 |
| **スキーマ分離（採用）** | 論理的に明確、移行容易 | 若干の複雑さ |
| 物理 DB 分離 | 完全な独立性 | 運用コスト増大 |

### 2. 外部キー制約を設けない設計

**場所**: [`20260122000001_create_auth_schema.sql:13-29`](../../../backend/migrations/20260122000001_create_auth_schema.sql)

**なぜこの設計か**:

`auth.credentials.user_id` から `public.users.id` への FK 制約は意図的に設けない。

| 理由 | 説明 |
|------|------|
| サービス境界の独立性 | FK があると、users 削除前に credentials 削除が必須となり、サービス間の操作順序が強制される |
| 将来の DB 分離 | Auth Service を独立した DB に分離する際、FK は別 DB 間では設定できない |
| 障害時の影響局所化 | FK 制約違反で一方のサービスの操作が他方に影響するのを防ぐ |

整合性はサービス間呼び出しで担保する:

- ユーザー作成時: Core Service → Auth Service
- ユーザー削除時: Core Service → Auth Service
- テナント退会時: `tenant_id` で並列削除

詳細: [技術ノート: マイクロサービス間のデータ整合性](../../06_技術ノート/マイクロサービス間のデータ整合性.md)

### 3. 移行期間中のデュアルライト

**場所**: [`20260122000002_migrate_password_to_credentials.sql`](../../../backend/migrations/20260122000002_migrate_password_to_credentials.sql)

**なぜこの設計か**:

`users.password_hash` を即座に削除せず、移行期間中は両方にデータを保持する。

```
Phase 1-2: users.password_hash ○  auth.credentials ○  ← 現在
Phase 3:   users.password_hash ○  auth.credentials ○  （BFF 統合中）
Phase 3完了: users.password_hash ×  auth.credentials ○  （カラム削除）
```

**メリット**:

- ロールバックが容易（credentials を削除するだけ）
- 段階的な移行が可能（一部機能から Auth Service を使い始められる）
- 既存の Core Service 認証ロジックがそのまま動作

**リスク**:

- データの二重管理による不整合の可能性
- → 移行期間中は credentials への書き込みのみ行い、users.password_hash は読み取り専用とする

## 関連ドキュメント

- 設計書: [08_AuthService設計.md](../../03_詳細設計書/08_AuthService設計.md)
- 技術ノート: [マイクロサービス間のデータ整合性.md](../../06_技術ノート/マイクロサービス間のデータ整合性.md)
- 技術ノート: [PostgreSQLマイグレーション構文.md](../../06_技術ノート/PostgreSQLマイグレーション構文.md)
