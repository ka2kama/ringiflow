# Issue #34: ユーザー認証（ログイン/ログアウト）実装計画

## 完了基準

- [ ] POST /auth/login でログインできる
- [ ] POST /auth/logout でログアウトできる
- [ ] GET /auth/me で現在のユーザー情報を取得できる

---

## アーキテクチャ

```
Browser → BFF (port 13000) → Core API (port 13001) → PostgreSQL
              ↓
            Redis (セッション)
```

| レイヤー | 責務 |
|---------|------|
| BFF | セッション管理、Cookie 操作、Core API プロキシ |
| Core API | ユーザー認証（email + password 検証）、last_login_at 更新 |
| Domain | User エンティティ、認証ビジネスルール |
| Infra | UserRepository、パスワードハッシング、Redis セッションストア |

---

## 技術選定

### パスワードハッシング: Argon2

- OWASP 推奨
- メモリハード（GPU/ASIC 攻撃に強い）

### セッション管理: 手動実装

**理由:**
1. 学習効果の最大化（プロジェクト理念）
2. MVP では複雑な機能不要
3. 既存の Redis ConnectionManager を活用

---

## 必要なクレート追加

```toml
# backend/Cargo.toml [workspace.dependencies]
argon2 = "0.5"
rand = "0.8"
async-trait = "0.1"

# backend/apps/bff/Cargo.toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
axum-extra = { version = "0.10", features = ["cookie", "typed-header"] }
```

---

## 実装ステップ

### Step 1: ドメイン層の拡充

**修正ファイル:**
- `backend/crates/domain/src/error.rs` - `AuthenticationFailed` エラー追加
- `backend/crates/domain/src/lib.rs` - `repository` モジュール追加

**新規ファイル:**
- `backend/crates/domain/src/repository.rs` - `UserRepository` トレイト定義

### Step 2: インフラ層の実装

**新規ファイル:**
- `backend/crates/infra/src/password.rs` - Argon2 ハッシング
- `backend/crates/infra/src/repository/mod.rs`
- `backend/crates/infra/src/repository/user_repository.rs` - PgUserRepository
- `backend/crates/infra/src/session.rs` - RedisSessionStore

### Step 3: Core API の実装

**新規ファイル:**
- `backend/apps/core-api/src/state.rs` - AppState（DB プール、リポジトリ）
- `backend/apps/core-api/src/dto/mod.rs`
- `backend/apps/core-api/src/dto/auth.rs` - リクエスト/レスポンス型
- `backend/apps/core-api/src/handler/auth.rs` - `POST /internal/auth/verify`

**修正ファイル:**
- `backend/apps/core-api/src/main.rs` - ルーティング追加
- `backend/apps/core-api/src/error.rs` - 認証エラー対応

### Step 4: BFF の実装

**新規ファイル:**
- `backend/apps/bff/src/state.rs` - BffState
- `backend/apps/bff/src/client/mod.rs`
- `backend/apps/bff/src/client/core_api.rs` - Core API クライアント
- `backend/apps/bff/src/session.rs` - セッション管理
- `backend/apps/bff/src/middleware/mod.rs`
- `backend/apps/bff/src/middleware/auth.rs` - 認証ミドルウェア
- `backend/apps/bff/src/dto/mod.rs`
- `backend/apps/bff/src/dto/auth.rs` - API レスポンス型
- `backend/apps/bff/src/handler/auth.rs` - `/auth/login`, `/auth/logout`, `/auth/me`

**修正ファイル:**
- `backend/apps/bff/src/main.rs` - ルーティング追加
- `backend/apps/bff/src/error.rs` - 認証エラー対応

### Step 5: テスト

- パスワードハッシングのユニットテスト
- UserRepository の統合テスト
- E2E テスト（ログイン → /auth/me → ログアウト）

### Step 6: ドキュメント

- ADR: セッション管理の選定理由

---

## セキュリティ考慮事項

| 項目 | 対策 |
|------|------|
| パスワード保存 | Argon2 ハッシュ化 |
| セッション Cookie | HttpOnly, Secure, SameSite=Lax |
| 認証エラー | メッセージ統一（情報漏洩防止） |
| セッション有効期限 | 8時間 + スライディングウィンドウ |

---

## 検証方法

### 1. ユニットテスト

```bash
just test
```

### 2. 手動検証

```bash
# サーバー起動
just dev

# ログイン
curl -X POST http://localhost:13000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"password123"}' \
  -c cookies.txt -v

# 現在ユーザー取得
curl http://localhost:13000/auth/me -b cookies.txt

# ログアウト
curl -X POST http://localhost:13000/auth/logout -b cookies.txt -c cookies.txt

# ログアウト後の確認（401 が返る）
curl http://localhost:13000/auth/me -b cookies.txt
```

---

## 重要ファイル（実装時参照）

- `backend/crates/domain/src/user.rs` - User エンティティ
- `backend/crates/domain/src/error.rs` - DomainError
- `backend/crates/infra/src/redis.rs` - Redis 接続
- `backend/apps/bff/src/main.rs` - BFF エントリーポイント
- `docs/02_設計書/03_API設計_MVP.md` - API 仕様
