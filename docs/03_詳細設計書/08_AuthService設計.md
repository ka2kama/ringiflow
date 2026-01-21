# Auth Service 設計

## 概要

本ドキュメントは、ADR-017 に基づく Auth Service 分離の詳細設計を定義する。

### 関連ドキュメント

- [ADR-017: Auth Service 分離の方針](../05_ADR/017_AuthService分離の方針.md)
- [07_認証機能設計.md](./07_認証機能設計.md)（現行設計、Phase 1）

### Phase 2 スコープ

| 含めるもの | 含めないもの（Phase 3 以降） |
|-----------|---------------------------|
| メール/パスワード認証の移行 | MFA（TOTP） |
| credentials テーブル分離 | SSO（OIDC/SAML） |
| Auth Service API | パスワードリセット |
| サービス間通信設計 | SCIM |

---

## 責務分担

### 分離後のアーキテクチャ

```mermaid
flowchart TB
    subgraph Frontend["フロントエンド"]
        Browser["ブラウザ"]
    end

    subgraph BFFLayer["BFF (port 13000)"]
        Session["セッション管理"]
        Cookie["Cookie/CSRF"]
        Proxy["API プロキシ"]
    end

    subgraph AuthService["Auth Service (port 13002)"]
        AuthAPI["認証 API"]
        CredRepo["Credentials Repository"]
    end

    subgraph CoreAPI["Core API (port 13001)"]
        UserAPI["ユーザー API"]
        UserRepo["User Repository"]
        Domain["ドメインロジック"]
    end

    subgraph DataLayer["データ層"]
        Redis[(Redis)]
        AuthDB[(PostgreSQL<br/>auth スキーマ)]
        CoreDB[(PostgreSQL<br/>public スキーマ)]
    end

    Browser --> BFFLayer
    Session --> Redis
    Proxy --> AuthService
    Proxy --> CoreAPI
    AuthAPI --> CredRepo
    CredRepo --> AuthDB
    UserAPI --> UserRepo
    UserRepo --> CoreDB
```

### コンポーネント責務

| コンポーネント | 責務 | 所有データ |
|--------------|------|----------|
| **BFF** | セッション管理、Cookie/CSRF、API 中継、レート制限 | セッション（Redis） |
| **Auth Service** | パスワード認証、将来の SSO/MFA | credentials |
| **Core API** | ユーザー情報管理、ビジネスロジック、認可 | users, roles, permissions |

### 現状との比較

| 観点 | 現状（Phase 1） | 分離後（Phase 2） |
|-----|----------------|------------------|
| パスワード検証 | Core API | **Auth Service** |
| credentials 保存 | users テーブル | **credentials テーブル（auth スキーマ）** |
| ユーザー情報取得 | Core API | Core API（変更なし） |
| セッション管理 | BFF | BFF（変更なし） |

---

## データモデル設計

### スキーマ分離

認証情報（credentials）を Core API の users テーブルから分離し、Auth Service 専用の `auth` スキーマに配置する。

#### DB 接続方式

Auth Service と Core API は**同一の PostgreSQL データベース**に接続し、スキーマで論理的に分離する。

```
PostgreSQL サーバー
└── ringiflow（データベース）
    ├── public（スキーマ）← Core API が所有
    │   ├── users
    │   ├── tenants
    │   └── ...
    └── auth（スキーマ）← Auth Service が所有
        └── credentials
```

接続文字列（両サービスとも同じ DB を指定）:

```bash
# Core API
DATABASE_URL=postgres://user:pass@db-host:5432/ringiflow

# Auth Service
DATABASE_URL=postgres://user:pass@db-host:5432/ringiflow
```

スキーマへのアクセスは SQL で明示的に指定する:

```sql
-- Core API: public スキーマ（デフォルト）
SELECT * FROM users;

-- Auth Service: auth スキーマを明示
SELECT * FROM auth.credentials;
```

この方式により、将来的に Auth Service を独立したデータベースに分離する際も、接続文字列の変更のみで対応可能。

```mermaid
erDiagram
    %% auth スキーマ（Auth Service が所有）
    auth_credentials {
        uuid id PK "Credential ID"
        uuid user_id FK "User ID（外部キー制約なし）"
        uuid tenant_id "Tenant ID"
        varchar credential_type "認証種別: password, totp, oidc, saml"
        text credential_data "暗号化された認証データ"
        boolean is_active "有効フラグ"
        timestamp last_used_at "最終使用日時"
        timestamp created_at "作成日時"
        timestamp updated_at "更新日時"
    }

    %% public スキーマ（Core API が所有）
    users {
        uuid id PK "User ID"
        uuid tenant_id FK "Tenant ID"
        varchar email "メールアドレス"
        varchar name "ユーザー名"
        varchar status "ステータス"
        timestamp last_login_at "最終ログイン日時"
        timestamp created_at "作成日時"
        timestamp updated_at "更新日時"
    }

    auth_credentials ||--o| users : "user_id"
```

#### 外部キー制約を設けない理由

`auth.credentials.user_id` から `public.users.id` への FK 制約は**意図的に設けない**。詳細は [技術ノート: マイクロサービス間のデータ整合性](../06_技術ノート/マイクロサービス間のデータ整合性.md) を参照。

| 理由 | 説明 |
|------|------|
| サービス境界の独立性 | FK があると、users 削除前に credentials 削除が必須となり、サービス間の操作順序が強制される |
| 将来の DB 分離 | Auth Service を独立した DB に分離する際、FK は別 DB 間では設定できない |
| 障害時の影響局所化 | FK 制約違反で一方のサービスの操作が他方に影響するのを防ぐ |

代わりの整合性担保:

- ユーザー作成時: Core API → Auth Service API で credentials を作成
- ユーザー削除時: Core API → Auth Service API で credentials を削除
- テナント退会時: `tenant_id` で両テーブルを並列削除（[テナント退会時データ削除設計](./06_テナント退会時データ削除設計.md)）

### credentials テーブル設計

#### credential_type の種別

| 種別 | 説明 | credential_data の内容 |
|-----|------|----------------------|
| `password` | パスワード認証 | Argon2id ハッシュ |
| `totp` | TOTP（Phase 3） | 暗号化されたシークレット |
| `oidc` | OIDC SSO（Phase 4） | subject, issuer |
| `saml` | SAML SSO（Phase 5） | name_id, issuer |

#### マイグレーション

```sql
-- auth スキーマの作成
CREATE SCHEMA IF NOT EXISTS auth;

-- credentials テーブル
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

    -- 同一ユーザー・同一種別の認証情報は一意
    CONSTRAINT uq_credentials_user_type UNIQUE (user_id, credential_type)
);

-- インデックス
CREATE INDEX idx_credentials_user_id ON auth.credentials(user_id);
CREATE INDEX idx_credentials_tenant_id ON auth.credentials(tenant_id);

-- テナント退会時削除用
COMMENT ON TABLE auth.credentials IS 'テナント退会時は tenant_id で削除';
```

#### users テーブルからの password_hash 削除

```sql
-- Phase 2 完了後に実行
ALTER TABLE users DROP COLUMN password_hash;
```

### テナント退会時の削除

[テナント退会時データ削除設計](./06_テナント退会時データ削除設計.md) に auth.credentials を追加する。

```sql
-- 削除順序: credentials → users（依存関係なし、並列削除可能）
DELETE FROM auth.credentials WHERE tenant_id = $1;
```

---

## API 設計

### Auth Service API

Auth Service は内部 API のみを提供し、BFF からのみアクセス可能。

#### POST /internal/auth/verify

パスワード認証を実行する。

**リクエスト:**
```json
{
  "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "password": "password123"
}
```

**レスポンス（200 OK）:**
```json
{
  "verified": true,
  "credential_id": "550e8400-e29b-41d4-a716-446655440002"
}
```

**エラー（401 Unauthorized）:**
```json
{
  "type": "https://ringiflow.example.com/errors/authentication-failed",
  "title": "Authentication Failed",
  "status": 401,
  "detail": "認証に失敗しました"
}
```

#### POST /internal/auth/credentials

認証情報を登録する（ユーザー作成時に呼び出し）。

**リクエスト:**
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
  "credential_type": "password",
  "credential_data": "plain_password_here"
}
```

**レスポンス（201 Created）:**
```json
{
  "credential_id": "550e8400-e29b-41d4-a716-446655440002"
}
```

#### DELETE /internal/auth/credentials/{user_id}

ユーザーの全認証情報を削除する（ユーザー削除時に呼び出し）。

**レスポンス（204 No Content）:** 空

---

### BFF API（変更点）

現行の `/auth/login` フローを変更し、Core API と Auth Service の両方を呼び出す。

#### 変更後のログインフロー

```mermaid
sequenceDiagram
    autonumber
    participant Browser as ブラウザ
    participant BFF as BFF
    participant Core as Core API
    participant Auth as Auth Service
    participant Redis as Redis

    Browser->>BFF: POST /auth/login (email, password)
    BFF->>Core: GET /internal/users/by-email?email=xxx&tenant_id=yyy
    Core-->>BFF: User or 404
    alt ユーザー存在
        BFF->>Auth: POST /internal/auth/verify (user_id, password)
        Auth-->>BFF: verified: true or 401
        alt 認証成功
            BFF->>Redis: SET session:{tenant_id}:{session_id}
            BFF-->>Browser: Set-Cookie + UserResponse
        else 認証失敗
            BFF-->>Browser: 401 Unauthorized
        end
    else ユーザー不存在
        Note over BFF: タイミング攻撃対策で<br/>ダミーの遅延を入れる
        BFF-->>Browser: 401 Unauthorized
    end
```

### Core API API（変更点）

#### GET /internal/users/by-email

メールアドレスでユーザーを検索する（新規追加）。

**クエリパラメータ:**
- `email`: メールアドレス
- `tenant_id`: テナント ID

**レスポンス（200 OK）:**
```json
{
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
    "email": "user@example.com",
    "name": "山田 太郎",
    "status": "active"
  }
}
```

**エラー（404 Not Found）:** ユーザーが存在しない

---

## サービス間通信設計

### 通信パターン

| 呼び出し元 | 呼び出し先 | プロトコル | 目的 |
|-----------|-----------|----------|------|
| BFF | Auth Service | HTTP/REST | パスワード認証 |
| BFF | Core API | HTTP/REST | ユーザー情報取得 |
| Core API | Auth Service | HTTP/REST | 認証情報の CRUD（ユーザー作成/削除時） |

### エラーハンドリング

#### Auth Service 障害時

Auth Service が応答しない場合、ログイン機能は利用不可となる（フェイルクローズ）。

```mermaid
flowchart TB
    A[BFF: ログインリクエスト受信] --> B{Auth Service 応答?}
    B -->|Yes| C[認証処理]
    B -->|No| D[503 Service Unavailable]
    D --> E["detail: 認証サービスが一時的に利用できません"]
```

#### サーキットブレーカ

BFF は Auth Service への接続にサーキットブレーカを適用する。

| 状態 | 条件 | 動作 |
|-----|------|------|
| Closed | 正常 | リクエストを通す |
| Open | 5秒間で3回連続失敗 | 即座に 503 を返す |
| Half-Open | Open から 30 秒後 | 1リクエストを試行 |

---

## 実装方針

### 初回デプロイ前のため新規構築

本番環境への初回デプロイがまだ行われていないため、段階的移行は不要。最初から Auth Service 分離済みの構成で構築する。

### 実装順序

```mermaid
flowchart LR
    subgraph Phase1["Phase 1: インフラ"]
        A1["auth スキーマ作成"]
        A2["credentials テーブル作成"]
    end

    subgraph Phase2["Phase 2: Auth Service"]
        B1["auth-service クレート作成"]
        B2["verify API 実装"]
        B3["credentials CRUD 実装"]
    end

    subgraph Phase3["Phase 3: 統合"]
        C1["BFF から Auth Service 呼び出し"]
        C2["Core API から認証コード削除"]
        C3["users.password_hash 削除"]
    end

    Phase1 --> Phase2 --> Phase3
```

### Phase 1: インフラ

1. `auth` スキーマを作成
2. `auth.credentials` テーブルを作成
3. シードデータに credentials を追加

### Phase 2: Auth Service

1. `backend/apps/auth-service` クレートを作成
2. `/internal/auth/verify` API を実装
3. `/internal/auth/credentials` CRUD API を実装
4. 単体テスト・統合テストを作成

### Phase 3: 統合

1. BFF の認証フローを Auth Service 経由に変更
2. Core API から `/internal/auth/verify` を削除
3. `users.password_hash` カラムを削除
4. E2E テストで全体の動作を確認

---

## 実装コンポーネント

### ディレクトリ構成

```
backend/
├── apps/
│   ├── bff/                    # 既存
│   ├── core-api/               # 既存
│   └── auth-service/           # 新規
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── config.rs
│           ├── handler.rs
│           ├── handler/
│           │   └── auth.rs
│           ├── usecase.rs
│           ├── usecase/
│           │   └── verify.rs
│           └── error.rs
└── crates/
    ├── domain/                 # 既存
    ├── infra/                  # 既存
    └── auth-domain/            # 新規（必要に応じて）
        └── src/
            └── credential.rs
```

### 依存関係

```mermaid
flowchart TB
    subgraph Apps
        BFF["bff"]
        Core["core-api"]
        Auth["auth-service"]
    end

    subgraph Crates
        Domain["domain"]
        Infra["infra"]
        AuthDomain["auth-domain"]
        Shared["shared"]
    end

    BFF --> Domain
    BFF --> Infra
    Core --> Domain
    Core --> Infra
    Auth --> AuthDomain
    Auth --> Infra
    AuthDomain --> Shared
    Domain --> Shared
    Infra --> Shared
```

---

## セキュリティ考慮事項

### 認証情報の保護

| 対策 | 実装 |
|------|------|
| パスワードハッシュ | Argon2id（既存と同じ） |
| 通信暗号化 | サービス間は VPC 内、将来的に mTLS 検討 |
| アクセス制御 | Auth Service は内部 API のみ公開 |

### タイミング攻撃対策

ユーザーが存在しない場合も、パスワード検証と同等の時間を消費する。

固定の sleep ではなく、**実際にダミーハッシュで Argon2id 検証を実行**する。これにより CPU/メモリ状況による自然な変動も含めて、実際の検証と同じ時間特性になる。

```rust
// Auth Service でのダミー検証
if user_not_found {
    // ダミーハッシュで検証（処理時間を均一化）
    let dummy_hash = PasswordHash::new(
        "$argon2id$v=19$m=65536,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$...",
    );
    let _ = password_checker.verify(password, &dummy_hash);
    return Err(AuthenticationFailed);
}
```

固定 sleep（例: 100ms）は統計的分析で検出される可能性があるため採用しない。

---

## テスト計画

### 単体テスト

| 対象 | テストケース |
|------|-------------|
| CredentialsRepository | CRUD 操作、テナント分離 |
| VerifyUseCase | 正常認証、パスワード不一致、無効な credential |

### 統合テスト

| シナリオ | テストケース |
|---------|-------------|
| ログイン（新フロー） | BFF → Core API → Auth Service の連携 |
| 認証情報作成 | ユーザー作成時に credentials も作成 |
| テナント退会 | credentials が削除される |

### E2E テスト

| シナリオ | テストケース |
|---------|-------------|
| 完全なログインフロー | ブラウザ → BFF → Auth Service → セッション確立 |
| Auth Service 障害時 | 503 が返り、ユーザーに適切なメッセージ表示 |

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-22 | 初版作成（責務整理、データモデル設計） |
