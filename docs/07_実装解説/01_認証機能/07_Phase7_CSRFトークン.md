# Phase 7: CSRF トークン

## 概要

CSRF（Cross-Site Request Forgery）防御機能を実装した。
ログイン時に CSRF トークンを自動生成し、状態変更リクエスト（POST/PUT/PATCH/DELETE）で検証する。

### 対応 Issue

[#34 ユーザー認証（ログイン/ログアウト）](https://github.com/ka2kama/ringiflow/issues/34) - Phase 7

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [CSRF 防御](../../03_詳細設計書/07_認証機能設計.md#csrf-防御) | Double Submit Cookie パターン |
| [Redis キー設計](../../03_詳細設計書/07_認証機能設計.md#redis-キー設計) | `csrf:{tenant_id}:{session_id}` |
| [API 設計 > GET /auth/csrf](../../03_詳細設計書/07_認証機能設計.md#get-authcsrf) | トークン取得エンドポイント |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/crates/infra/src/session.rs`](../../../backend/crates/infra/src/session.rs) | CSRF トークン管理メソッド追加 |
| [`backend/apps/bff/src/middleware/csrf.rs`](../../../backend/apps/bff/src/middleware/csrf.rs) | CSRF 検証ミドルウェア（新規） |
| [`backend/apps/bff/src/middleware/mod.rs`](../../../backend/apps/bff/src/middleware/mod.rs) | ミドルウェアモジュール（新規） |
| [`backend/apps/bff/src/handler/auth.rs`](../../../backend/apps/bff/src/handler/auth.rs) | ログイン/ログアウト時の CSRF トークン処理 |
| [`backend/apps/bff/tests/auth_integration_test.rs`](../../../backend/apps/bff/tests/auth_integration_test.rs) | CSRF 統合テスト追加 |

---

## 実装内容

### 1. SessionManager トレイト拡張

CSRF トークン管理用のメソッドを追加。

```rust
#[async_trait]
pub trait SessionManager: Send + Sync {
    // ... 既存メソッド ...

    /// CSRF トークンを作成する
    async fn create_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<String, InfraError>;

    /// CSRF トークンを取得する
    async fn get_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<Option<String>, InfraError>;

    /// CSRF トークンを削除する
    async fn delete_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<(), InfraError>;

    /// テナントの全 CSRF トークンを削除する（テナント退会時）
    async fn delete_all_csrf_for_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError>;
}
```

### 2. CSRF トークン生成

64 文字の暗号論的ランダム文字列（hex エンコード）を生成。

```rust
fn generate_csrf_token() -> String {
    let uuid1 = Uuid::new_v4();
    let uuid2 = Uuid::new_v4();
    format!("{}{}", uuid1.simple(), uuid2.simple())
}
```

**Redis キー設計:**

| キー | 値 | TTL |
|-----|-----|-----|
| `csrf:{tenant_id}:{session_id}` | 64文字 hex 文字列 | 28800秒（8時間） |

### 3. CSRF 検証ミドルウェア

状態変更リクエストで CSRF トークンを検証。

```rust
pub async fn csrf_middleware<S>(
    State(state): State<CsrfState<S>>,
    jar: CookieJar,
    request: Request<Body>,
    next: Next,
) -> Response
where
    S: SessionManager + Clone + 'static,
{
    // POST/PUT/PATCH/DELETE のみ検証
    if !requires_csrf_validation(&method) || should_skip_csrf(&path) {
        return next.run(request).await;
    }

    // X-CSRF-Token ヘッダーと Redis 内のトークンを比較
    // ...
}
```

**スキップするパス:**
- `/auth/login` - ログイン前は CSRF トークンがない
- `/auth/csrf` - トークン取得用エンドポイント
- `/health` - ヘルスチェック

### 4. ログイン時の自動生成

セッション作成後に CSRF トークンを自動生成。

```rust
match state.session_manager.create(&session_data).await {
    Ok(session_id) => {
        // CSRF トークンを作成
        let tenant_id = TenantId::from_uuid(verified.user.tenant_id);
        if let Err(e) = state
            .session_manager
            .create_csrf_token(&tenant_id, &session_id)
            .await
        {
            tracing::error!("CSRF トークン作成に失敗: {}", e);
            return internal_error_response();
        }
        // ...
    }
}
```

### 5. ログアウト時の削除

セッション削除と同時に CSRF トークンも削除。

```rust
// CSRF トークンを削除（エラーは無視）
if let Err(e) = state
    .session_manager
    .delete_csrf_token(&tenant_id, session_id)
    .await
{
    tracing::warn!("CSRF トークン削除に失敗（無視）: {}", e);
}

// セッションを削除
if let Err(e) = state.session_manager.delete(&tenant_id, session_id).await {
    tracing::warn!("セッション削除に失敗（無視）: {}", e);
}
```

---

## テスト

### 単体テスト（session_test.rs）

| テスト | 検証内容 |
|-------|---------|
| `test_csrfトークンを作成できる` | 64 文字の hex 文字列が返る |
| `test_csrfトークンを取得できる` | 作成したトークンを取得 |
| `test_存在しないcsrfトークンはnoneを返す` | 存在しない場合は None |
| `test_csrfトークンを削除できる` | 削除後は None |
| `test_テナント単位で全csrfトークンを削除できる` | 一括削除 |

### 統合テスト（auth_integration_test.rs）

| テスト | 検証内容 |
|-------|---------|
| `test_csrfトークン_ログイン成功時に生成される` | ログイン後に Redis にトークン存在 |
| `test_csrfトークン_get_auth_csrfで取得できる` | エンドポイントからトークン取得 |
| `test_csrfトークン_正しいトークンでpostリクエストが成功する` | 正しいトークンで 204 |
| `test_csrfトークン_トークンなしでpostリクエストが403になる` | トークンなしで 403 |
| `test_csrfトークン_不正なトークンでpostリクエストが403になる` | 不正トークンで 403 |
| `test_csrfトークン_ログアウト時に削除される` | ログアウト後に Redis からトークン削除 |

### テスト実行

```bash
just dev-deps  # Redis 起動
cd backend && cargo test -p ringiflow-bff --test auth_integration_test
cd backend && cargo test -p ringiflow-infra --test session_test
```

---

## 関連ドキュメント

- 設計書: [07_認証機能設計.md](../../03_詳細設計書/07_認証機能設計.md#csrf-防御)

---

# 設計解説

以下は設計判断の詳細解説。レビュー・学習用。

---

## 1. UUID v4 × 2 による CSRF トークン生成

**場所:** [`session.rs:237-241`](../../../backend/crates/infra/src/session.rs#L237-L241)

```rust
fn generate_csrf_token() -> String {
    let uuid1 = Uuid::new_v4();
    let uuid2 = Uuid::new_v4();
    format!("{}{}", uuid1.simple(), uuid2.simple())
}
```

**なぜこの方式か:**

| 方式 | トークン長 | エントロピー |
|------|----------|------------|
| UUID v4 × 1 | 32 文字 | 122 ビット |
| UUID v4 × 2 | 64 文字 | 244 ビット |
| ランダムバイト | 可変 | 可変 |

設計書で「64 文字の暗号論的ランダム文字列」と定義されていたため、
UUID v4 を 2 つ連結して 64 文字の hex 文字列を生成。

**代替案:**

```rust
// rand crate を使う場合
use rand::{thread_rng, Rng};
let bytes: [u8; 32] = thread_rng().gen();
hex::encode(bytes)  // 64 文字
```

UUID v4 は `uuid` crate に含まれており、追加依存なしで使える。
また、UUID v4 自体が暗号論的に安全なランダム値を使用している。

---

## 2. ミドルウェアの適用順序

**場所:** [`main.rs:142-146`](../../../backend/apps/bff/src/main.rs#L142-L146)

```rust
let app = Router::new()
    // ... routes ...
    .with_state(auth_state)
    .layer(from_fn_with_state(csrf_state, csrf_middleware::<RedisSessionManager>))
    .layer(TraceLayer::new_for_http());
```

**layer の適用順序:**

axum の layer は「後に追加したものが先に実行される」。

```
リクエスト → TraceLayer → csrf_middleware → ハンドラ → csrf_middleware → TraceLayer → レスポンス
```

TraceLayer を最後に追加することで、すべてのリクエストがトレーシングされる。

**なぜ `from_fn_with_state` か:**

axum でミドルウェアに状態を渡す方法:

| 方式 | 用途 |
|------|------|
| `from_fn` | 状態なしミドルウェア |
| `from_fn_with_state` | 状態ありミドルウェア |
| `Layer` トレイト実装 | 再利用可能なミドルウェア |

今回は `CsrfState` を渡す必要があったため `from_fn_with_state` を使用。

---

## 3. CsrfState と AuthState の分離

**場所:** [`csrf.rs:30-37`](../../../backend/apps/bff/src/middleware/csrf.rs#L30-L37)

```rust
#[derive(Clone)]
pub struct CsrfState<S>
where
   S: SessionManager + Clone,
{
   pub session_manager: S,
}
```

**なぜ AuthState と別にしたか:**

| 観点 | CsrfState | AuthState |
|------|-----------|-----------|
| 依存 | SessionManager のみ | CoreApiClient + SessionManager |
| 用途 | CSRF 検証 | 認証ハンドラ |
| 型パラメータ | 1 つ | 2 つ |

CSRF ミドルウェアは CoreApiClient を必要としないため、
依存を最小限にするために別の状態型を定義。

**代替案:**

```rust
// AuthState を共有する場合
pub async fn csrf_middleware<C, S>(
    State(state): State<Arc<AuthState<C, S>>>,
    // ...
) where C: CoreApiClient, S: SessionManager
```

これでも動作するが、CSRF ミドルウェアが CoreApiClient に依存する形になり、
不要な結合が生まれる。

---

## 4. CSRF 検証をスキップするパス

**場所:** [`csrf.rs:30`](../../../backend/apps/bff/src/middleware/csrf.rs#L30)

```rust
const CSRF_SKIP_PATHS: &[&str] = &["/auth/login", "/auth/csrf", "/health"];
```

**なぜこれらをスキップするか:**

| パス | 理由 |
|------|------|
| `/auth/login` | ログイン前は CSRF トークンがない |
| `/auth/csrf` | トークン取得用エンドポイント（GET） |
| `/health` | ヘルスチェックは認証不要 |

**設計上の注意:**

`/auth/login` をスキップすることで、CSRF 攻撃でログインを強制される可能性がある。
しかし、ログイン自体はユーザー情報の変更ではないため、リスクは限定的。

より厳密な対策が必要な場合は、ログインフォームに埋め込む
一時的な CSRF トークン（セッションに紐づかない）を使う。

---

## 5. エラーハンドリング: ログイン vs ログアウト

**ログイン時の CSRF トークン作成失敗:**

```rust
if let Err(e) = state.session_manager.create_csrf_token(&tenant_id, &session_id).await {
    tracing::error!("CSRF トークン作成に失敗: {}", e);
    return internal_error_response();  // 500 エラー
}
```

**ログアウト時の CSRF トークン削除失敗:**

```rust
if let Err(e) = state.session_manager.delete_csrf_token(&tenant_id, session_id).await {
    tracing::warn!("CSRF トークン削除に失敗（無視）: {}", e);  // 処理続行
}
```

**なぜ違うのか:**

| 操作 | 失敗時の影響 | 対応 |
|------|------------|------|
| 作成 | CSRF 防御が機能しない | エラーを返す |
| 削除 | 孤児トークンが残る | TTL で自動削除されるため無視 |

ログイン時に CSRF トークンが作成できないと、以降の状態変更リクエストが
すべて 403 になってしまう。これはユーザー体験を損なうため、エラーを返す。

ログアウト時の削除失敗は、トークンが Redis に残るだけ。
TTL（8 時間）で自動削除されるため、ログアウト自体は成功させる。

---

## 6. RedisSessionManager への Clone 追加

**場所:** [`session.rs:197`](../../../backend/crates/infra/src/session.rs#L197)

```rust
#[derive(Clone)]
pub struct RedisSessionManager {
   conn: ConnectionManager,
}
```

**なぜ Clone が必要か:**

CSRF ミドルウェアと認証ハンドラで同じ `session_manager` を共有するため。

```rust
// main.rs
let session_manager = RedisSessionManager::new(&config.redis_url).await?;

let csrf_state = CsrfState {
    session_manager: session_manager.clone(),  // Clone
};

let auth_state = Arc::new(AuthState {
    core_api_client,
    session_manager,  // Move
});
```

**ConnectionManager は Clone が安価:**

`ConnectionManager` は内部で `Arc` を使用しており、
Clone は参照カウントのインクリメントのみ。
