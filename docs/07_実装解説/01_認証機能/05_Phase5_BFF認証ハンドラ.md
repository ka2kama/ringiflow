# Phase 5: BFF 認証ハンドラ

## 概要

BFF（Backend for Frontend）の認証エンドポイントを実装した。
ブラウザからのログイン/ログアウトリクエストを受け、Core Service との連携とセッション管理を行う。

### 対応 Issue

- [#34 ユーザー認証（ログイン/ログアウト）](https://github.com/ka2kama/ringiflow/issues/34)

## 設計書との対応

| 設計書セクション | 実装内容 |
|-----------------|---------|
| [BFF 公開 API](../../03_詳細設計書/07_認証機能設計.md#bff-公開-api) | `/auth/login`, `/auth/logout`, `/auth/me` |
| [責務分担](../../03_詳細設計書/07_認証機能設計.md#責務分担) | BFF: セッション管理、Cookie 処理、Core Service への中継 |
| [Cookie 属性](../../03_詳細設計書/07_認証機能設計.md#cookie-属性) | HttpOnly, SameSite=Lax, Path=/, Max-Age=28800 |

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`client.rs`](../../../backend/apps/bff/src/client.rs) | クライアントモジュール定義 |
| [`client/core_api.rs`](../../../backend/apps/bff/src/client/core_api.rs) | Core Service との HTTP 通信 |
| [`handler/auth.rs`](../../../backend/apps/bff/src/handler/auth.rs) | 認証エンドポイント |
| [`config.rs`](../../../backend/apps/bff/src/config.rs) | 設定（Redis URL, Core Service URL 追加） |

## 実装内容

### CoreApiClient

Core Service との通信を担当するクライアント。テスト容易性のためトレイトで定義。

```rust
#[async_trait]
pub trait CoreApiClient: Send + Sync {
    /// 認証情報を検証（POST /internal/auth/verify）
    async fn verify_credentials(
        &self,
        tenant_id: Uuid,
        email: &str,
        password: &str,
    ) -> Result<VerifyResponse, CoreApiError>;

    /// ユーザー情報を取得（GET /internal/users/{user_id}）
    async fn get_user(&self, user_id: Uuid) -> Result<UserWithPermissionsResponse, CoreApiError>;
}
```

### 認証ハンドラ

| エンドポイント | 処理内容 |
|---------------|---------|
| `POST /auth/login` | Core Service で認証 → セッション作成 → Cookie 設定 |
| `POST /auth/logout` | セッション削除 → Cookie クリア |
| `GET /auth/me` | セッション取得 → Core Service でユーザー情報取得 |

### テナント ID の取得

MVP では `X-Tenant-ID` ヘッダーでテナント ID を指定する方式を採用。
本番環境ではサブドメインから解決する予定。

## テスト

### テストケース

| テスト | 説明 |
|-------|------|
| `test_login_成功時にセッションcookieが設定される` | ログイン成功で Cookie が返る |
| `test_login_成功時にユーザー情報が返る` | レスポンスにユーザー情報が含まれる |
| `test_login_認証失敗で401` | 認証失敗で 401 Unauthorized |
| `test_logout_セッションが削除されてcookieがクリアされる` | ログアウトで Cookie が削除される |
| `test_me_認証済みでユーザー情報が返る` | 認証済みでユーザー情報を取得 |
| `test_me_未認証で401` | 未認証で 401 Unauthorized |
| `test_login_テナントIDヘッダーなしで400` | ヘッダーなしで 400 Bad Request |

### 実行方法

```bash
cargo test -p ringiflow-bff
```

## 関連ドキュメント

- [07_認証機能設計.md](../../03_詳細設計書/07_認証機能設計.md)
- [Phase 4: Core Service 認証エンドポイント](./04_Phase4_CoreAPI認証エンドポイント.md)
- [Phase 3: SessionManager](./03_Phase3_SessionManager.md)

---

## 設計解説

### 1. トレイトによる依存関係の抽象化

**場所:** [`client/core_api.rs:86-111`](../../../backend/apps/bff/src/client/core_api.rs#L86-L111)

```rust
#[async_trait]
pub trait CoreApiClient: Send + Sync {
    async fn verify_credentials(...) -> Result<VerifyResponse, CoreApiError>;
    async fn get_user(...) -> Result<UserWithPermissionsResponse, CoreApiError>;
}
```

**なぜこの設計か:**

テスト時にスタブを使用し、外部 API に依存せずにテストできる。
実際の HTTP 通信を行う `CoreApiClientImpl` と、テスト用の `StubCoreApiClient` を切り替えられる。

**代替案:**

- mockall 等のモックライブラリを使用する
  - トレードオフ: 依存関係が増える、設定が複雑になる場合がある
- テスト時のみ実際の Core Service を起動する
  - トレードオフ: テストが遅くなる、環境依存が増える

トレイトベースの設計は Rust の慣習に沿っており、シンプルで理解しやすい。

### 2. ジェネリクスを活用したハンドラ定義

**場所:** [`handler/auth.rs:131-172`](../../../backend/apps/bff/src/handler/auth.rs#L131-L172)

```rust
pub async fn login<C, S>(
    State(state): State<Arc<AuthState<C, S>>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse
where
    C: CoreApiClient,
    S: SessionManager,
```

**なぜこの設計か:**

ハンドラをジェネリクスにすることで、依存関係を型パラメータとして注入できる。
テスト時はスタブ、本番時は実装を注入する。

**代替案:**

- `dyn Trait` を使用する（動的ディスパッチ）
  - トレードオフ: ランタイムオーバーヘッド、Object Safety の制約
- 環境変数でモックモードを切り替える
  - トレードオフ: 型安全性が低下、テストの信頼性が下がる

ジェネリクスは axum のパターンに沿っており、コンパイル時に型チェックされる。

### 3. X-Tenant-ID ヘッダーによるテナント識別

**場所:** [`handler/auth.rs:299-326`](../../../backend/apps/bff/src/handler/auth.rs#L299-L326)

```rust
fn extract_tenant_id(headers: &HeaderMap) -> Result<Uuid, axum::response::Response> {
    let tenant_id_str = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ...)?;
    // ...
}
```

**なぜこの設計か:**

MVP ではシンプルにヘッダーでテナント ID を渡す方式を採用。
本番環境ではサブドメイン（`tenant1.ringiflow.com`）から解決する予定だが、
MVP では開発・テストの容易さを優先した。

**代替案:**

- サブドメインから解決する
  - トレードオフ: DNS 設定が必要、ローカル開発が複雑になる
- リクエストボディに含める
  - トレードオフ: RESTful でない、GET リクエストで使えない
- JWT トークンに含める
  - トレードオフ: セッション方式と相性が悪い

ヘッダー方式はマルチテナント SaaS でよく使われるパターンであり、
将来的にミドルウェアでサブドメインからヘッダーに変換する設計にも対応しやすい。

### 4. Cookie 設定のセキュリティ属性

**場所:** [`handler/auth.rs:329-339`](../../../backend/apps/bff/src/handler/auth.rs#L329-L339)

```rust
fn build_session_cookie(session_id: &str) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, session_id.to_string()))
        .path("/")
        .max_age(time::Duration::seconds(SESSION_MAX_AGE))
        .http_only(true)
        .same_site(SameSite::Lax)
        // .secure(true) // TODO: 本番環境で有効化
        .build()
}
```

**なぜこの設計か:**

セキュリティ設計書の要件に従い、以下の属性を設定:

- `HttpOnly`: JavaScript からのアクセスを禁止（XSS 対策）
- `SameSite=Lax`: クロスサイトリクエストを制限（CSRF 対策）
- `Path=/`: 全パスで有効
- `Max-Age=28800`: 8 時間（セッション有効期限と同期）

`Secure` 属性は本番環境（HTTPS）でのみ有効にする。
開発環境では HTTP を使用するため、現時点ではコメントアウト。

**代替案:**

- `SameSite=Strict` にする
  - トレードオフ: 外部サイトからのリンク遷移でセッションが切れる
- `SameSite=None` にする
  - トレードオフ: CSRF に脆弱になる（クロスサイトでも Cookie が送られる）

`Lax` は利便性とセキュリティのバランスが取れた選択。
