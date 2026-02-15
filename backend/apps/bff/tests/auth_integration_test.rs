//! BFF 認証統合テスト
//!
//! 実際の Redis を使用してセッション管理の一連のフローをテストする。
//! Core API と Auth Service の呼び出しはスタブを使用する。
//!
//! ## 実行方法
//!
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-bff --test auth_integration_test
//! ```
//!
//! ## テストケース
//!
//! - ログイン → /api/v1/auth/me → ログアウトの一連フロー
//! - ログアウト後に /api/v1/auth/me で 401
//! - 不正なパスワードでログインできない
//! - 存在しないメールでログインできない
//! - 非アクティブユーザーはログインできない
//! - CSRF トークン: ログイン成功時に生成される
//! - CSRF トークン: GET /api/v1/auth/csrf で取得できる
//! - CSRF トークン: 正しいトークンで POST リクエストが成功する
//! - CSRF トークン: トークンなしで POST リクエストが 403 になる
//! - CSRF トークン: 不正なトークンで POST リクエストが 403 になる
//! - CSRF トークン: ログアウト時に削除される

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    middleware::from_fn_with_state,
    routing::{get, post},
};
use ringiflow_bff::{
    client::{
        AuthServiceClient,
        AuthServiceError,
        CoreServiceError,
        CoreServiceRoleClient,
        CoreServiceUserClient,
        UserResponse,
        UserWithPermissionsData,
        VerifyResponse,
    },
    handler::{AuthState, csrf, login, logout, me},
    middleware::{CsrfState, csrf_middleware},
};
use ringiflow_infra::{RedisSessionManager, SessionManager};
use ringiflow_shared::ApiResponse;
use tower::ServiceExt;
use uuid::Uuid;

/// テスト用の Redis URL
///
/// 優先順位:
/// 1. `REDIS_URL`（CI で明示的に設定）
/// 2. `REDIS_PORT` から構築（justfile が root `.env` から渡す）
/// 3. フォールバック: `redis://localhost:16379`
fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| {
        let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "16379".to_string());
        format!("redis://localhost:{port}")
    })
}

/// テスト用のテナント ID
fn test_tenant_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
}

/// テスト用のユーザー ID
fn test_user_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
}

// --- Core Service スタブ ---

/// Core Service スタブの設定
#[derive(Clone)]
struct CoreServiceStubConfig {
    /// ユーザーが存在するか
    user_exists: bool,
}

impl CoreServiceStubConfig {
    fn success() -> Self {
        Self { user_exists: true }
    }

    fn user_not_found() -> Self {
        Self { user_exists: false }
    }

    /// 非アクティブユーザー用
    ///
    /// 現在の設計では、ユーザーのアクティブ状態は Auth Service 側で
    /// 認証情報の有効性としてチェックされる。ここでは Core Service が
    /// ユーザーを返すが、Auth Service で認証が失敗するシナリオを
    /// テストするために使用する。
    fn user_inactive() -> Self {
        // 非アクティブユーザーもユーザー自体は存在する
        Self { user_exists: true }
    }
}

/// テスト用 Core Service クライアント
struct StubCoreServiceClient {
    config: CoreServiceStubConfig,
}

impl StubCoreServiceClient {
    fn new(config: CoreServiceStubConfig) -> Self {
        Self { config }
    }

    fn create_user_response() -> UserResponse {
        UserResponse {
            id:        test_user_id(),
            tenant_id: test_tenant_id(),
            email:     "user@example.com".to_string(),
            name:      "Test User".to_string(),
            status:    "active".to_string(),
        }
    }
}

#[async_trait]
impl CoreServiceUserClient for StubCoreServiceClient {
    async fn get_user_by_email(
        &self,
        _tenant_id: Uuid,
        _email: &str,
    ) -> Result<ApiResponse<UserResponse>, CoreServiceError> {
        if !self.config.user_exists {
            return Err(CoreServiceError::UserNotFound);
        }

        Ok(ApiResponse::new(Self::create_user_response()))
    }

    async fn get_user(
        &self,
        _user_id: Uuid,
    ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError> {
        if !self.config.user_exists {
            return Err(CoreServiceError::UserNotFound);
        }

        Ok(ApiResponse::new(UserWithPermissionsData {
            user:        Self::create_user_response(),
            tenant_name: "Development Tenant".to_string(),
            roles:       vec!["user".to_string()],
            permissions: vec!["workflow:read".to_string()],
        }))
    }

    async fn list_users(
        &self,
        _tenant_id: Uuid,
        _status: Option<&str>,
    ) -> Result<ApiResponse<Vec<ringiflow_bff::client::UserItemDto>>, CoreServiceError> {
        unimplemented!("list_users is not used in auth tests")
    }

    async fn create_user(
        &self,
        _req: &ringiflow_bff::client::CreateUserCoreRequest,
    ) -> Result<ApiResponse<ringiflow_bff::client::CreateUserCoreResponse>, CoreServiceError> {
        unimplemented!("create_user is not used in auth tests")
    }

    async fn update_user(
        &self,
        _user_id: Uuid,
        _req: &ringiflow_bff::client::UpdateUserCoreRequest,
    ) -> Result<ApiResponse<UserResponse>, CoreServiceError> {
        unimplemented!("update_user is not used in auth tests")
    }

    async fn update_user_status(
        &self,
        _user_id: Uuid,
        _req: &ringiflow_bff::client::UpdateUserStatusCoreRequest,
    ) -> Result<ApiResponse<UserResponse>, CoreServiceError> {
        unimplemented!("update_user_status is not used in auth tests")
    }

    async fn get_user_by_display_number(
        &self,
        _tenant_id: Uuid,
        _display_number: i64,
    ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError> {
        unimplemented!("get_user_by_display_number is not used in auth tests")
    }
}

#[async_trait]
impl CoreServiceRoleClient for StubCoreServiceClient {
    async fn list_roles(
        &self,
        _tenant_id: Uuid,
    ) -> Result<
        ringiflow_shared::ApiResponse<Vec<ringiflow_bff::client::RoleItemDto>>,
        CoreServiceError,
    > {
        unimplemented!("list_roles is not used in auth tests")
    }

    async fn get_role(
        &self,
        _role_id: Uuid,
        _tenant_id: Uuid,
    ) -> Result<ringiflow_shared::ApiResponse<ringiflow_bff::client::RoleDetailDto>, CoreServiceError>
    {
        unimplemented!("get_role is not used in auth tests")
    }

    async fn create_role(
        &self,
        _req: &ringiflow_bff::client::CreateRoleCoreRequest,
    ) -> Result<ringiflow_shared::ApiResponse<ringiflow_bff::client::RoleDetailDto>, CoreServiceError>
    {
        unimplemented!("create_role is not used in auth tests")
    }

    async fn update_role(
        &self,
        _role_id: Uuid,
        _req: &ringiflow_bff::client::UpdateRoleCoreRequest,
    ) -> Result<ringiflow_shared::ApiResponse<ringiflow_bff::client::RoleDetailDto>, CoreServiceError>
    {
        unimplemented!("update_role is not used in auth tests")
    }

    async fn delete_role(&self, _role_id: Uuid) -> Result<(), CoreServiceError> {
        unimplemented!("delete_role is not used in auth tests")
    }
}

// --- Auth Service スタブ ---

/// Auth Service スタブの設定
#[derive(Clone)]
struct AuthServiceStubConfig {
    /// 認証が成功するか
    auth_success: bool,
}

impl AuthServiceStubConfig {
    fn success() -> Self {
        Self { auth_success: true }
    }

    fn auth_failed() -> Self {
        Self {
            auth_success: false,
        }
    }
}

/// テスト用 Auth Service クライアント
struct StubAuthServiceClient {
    config: AuthServiceStubConfig,
}

impl StubAuthServiceClient {
    fn new(config: AuthServiceStubConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AuthServiceClient for StubAuthServiceClient {
    async fn verify_password(
        &self,
        _tenant_id: Uuid,
        _user_id: Uuid,
        _password: &str,
    ) -> Result<VerifyResponse, AuthServiceError> {
        if !self.config.auth_success {
            return Err(AuthServiceError::AuthenticationFailed);
        }

        Ok(VerifyResponse {
            verified:      true,
            credential_id: Some(Uuid::now_v7()),
        })
    }

    async fn create_credentials(
        &self,
        _tenant_id: Uuid,
        _user_id: Uuid,
        _credential_type: &str,
        _credential_data: &str,
    ) -> Result<ringiflow_bff::client::CreateCredentialsResponse, AuthServiceError> {
        unimplemented!("create_credentials is not used in auth tests")
    }
}

// --- テストヘルパー ---

/// テスト用アプリケーションを作成
async fn create_test_app(
    core_service_config: CoreServiceStubConfig,
    auth_service_config: AuthServiceStubConfig,
) -> (Router, Arc<AuthState>) {
    let session_manager: Arc<dyn SessionManager> = Arc::new(
        RedisSessionManager::new(&redis_url())
            .await
            .expect("Redis への接続に失敗"),
    );

    // CSRF ミドルウェア用の状態
    let csrf_state = CsrfState {
        session_manager: session_manager.clone(),
    };

    let state = Arc::new(AuthState {
        core_service_client: Arc::new(StubCoreServiceClient::new(core_service_config)),
        auth_service_client: Arc::new(StubAuthServiceClient::new(auth_service_config)),
        session_manager,
    });

    let sut = Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/auth/csrf", get(csrf))
        .with_state(state.clone())
        .layer(from_fn_with_state(csrf_state, csrf_middleware));

    (sut, state)
}

/// ログインリクエストを作成
fn login_request(email: &str, password: &str) -> Request<Body> {
    let body = serde_json::json!({
        "email": email,
        "password": password
    });

    Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .header("X-Tenant-ID", test_tenant_id().to_string())
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

/// ログアウトリクエストを作成
fn logout_request(session_cookie: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/logout")
        .header("X-Tenant-ID", test_tenant_id().to_string())
        .header("Cookie", format!("session_id={}", session_cookie))
        .body(Body::empty())
        .unwrap()
}

/// /api/v1/auth/me リクエストを作成
fn me_request(session_cookie: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/me")
        .header("X-Tenant-ID", test_tenant_id().to_string())
        .header("Cookie", format!("session_id={}", session_cookie))
        .body(Body::empty())
        .unwrap()
}

/// /api/v1/auth/me リクエストを作成（Cookie なし）
fn me_request_without_cookie() -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/me")
        .header("X-Tenant-ID", test_tenant_id().to_string())
        .body(Body::empty())
        .unwrap()
}

/// Set-Cookie ヘッダーからセッション ID を抽出
fn extract_session_id(set_cookie: &str) -> Option<String> {
    // "session_id=xxx; Path=/; ..." の形式からセッション ID を抽出
    set_cookie
        .split(';')
        .next()
        .and_then(|s| s.strip_prefix("session_id="))
        .map(|s| s.to_string())
}

// --- テストケース ---

#[tokio::test]
async fn test_ログインからログアウトまでの一連フロー() {
    // Given
    let (sut, state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // When: ログイン
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();

    // Then: ログイン成功
    assert_eq!(login_response.status(), StatusCode::OK);

    // セッション ID を取得
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id(set_cookie).expect("セッション ID が設定されていない");
    assert!(!session_id.is_empty());

    // When: /api/v1/auth/me でユーザー情報を取得
    let me_response = sut.clone().oneshot(me_request(&session_id)).await.unwrap();

    // Then: ユーザー情報が返る
    assert_eq!(me_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(me_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["data"]["email"], "user@example.com");
    assert_eq!(json["data"]["name"], "Test User");

    // CSRF トークンを取得
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let csrf_token = state
        .session_manager
        .get_csrf_token(&tenant_id, &session_id)
        .await
        .unwrap()
        .expect("CSRF トークンが存在しない");

    // When: ログアウト（CSRF トークン付き）
    let logout_response = sut
        .clone()
        .oneshot(logout_request_with_csrf(&session_id, &csrf_token))
        .await
        .unwrap();

    // Then: ログアウト成功
    assert_eq!(logout_response.status(), StatusCode::NO_CONTENT);

    // Cookie がクリアされていることを確認
    let clear_cookie = logout_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(clear_cookie.contains("Max-Age=0"));

    // クリーンアップ: セッションが削除されていることを確認
    let session = state
        .session_manager
        .get(&tenant_id, &session_id)
        .await
        .unwrap();
    assert!(session.is_none(), "セッションが削除されていない");
}

#[tokio::test]
async fn test_ログアウト後にauthmeで401() {
    // Given
    let (sut, state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // ログイン
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id(set_cookie).unwrap();

    // CSRF トークンを取得
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let csrf_token = state
        .session_manager
        .get_csrf_token(&tenant_id, &session_id)
        .await
        .unwrap()
        .expect("CSRF トークンが存在しない");

    // ログアウト（CSRF トークン付き）
    sut.clone()
        .oneshot(logout_request_with_csrf(&session_id, &csrf_token))
        .await
        .unwrap();

    // When: ログアウト後に /api/v1/auth/me にアクセス
    let me_response = sut.clone().oneshot(me_request(&session_id)).await.unwrap();

    // Then: 401 Unauthorized
    assert_eq!(me_response.status(), StatusCode::UNAUTHORIZED);

    // クリーンアップ
    let _ = state.session_manager.delete(&tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_不正なパスワードでログインできない() {
    // Given
    let (sut, _state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::auth_failed(),
    )
    .await;

    // When
    let response = sut
        .oneshot(login_request("user@example.com", "wrongpassword"))
        .await
        .unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Cookie が設定されていないことを確認
    assert!(response.headers().get("set-cookie").is_none());
}

#[tokio::test]
async fn test_存在しないメールでログインできない() {
    // Given
    let (sut, _state) = create_test_app(
        CoreServiceStubConfig::user_not_found(),
        AuthServiceStubConfig::auth_failed(),
    )
    .await;

    // When
    let response = sut
        .oneshot(login_request("nonexistent@example.com", "password123"))
        .await
        .unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_非アクティブユーザーはログインできない() {
    // Given
    let (sut, _state) = create_test_app(
        CoreServiceStubConfig::user_inactive(),
        AuthServiceStubConfig::auth_failed(),
    )
    .await;

    // When
    let response = sut
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_未認証状態でauthmeにアクセスすると401() {
    // Given
    let (sut, _state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // When: Cookie なしで /api/v1/auth/me にアクセス
    let response = sut.oneshot(me_request_without_cookie()).await.unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- CSRF トークン統合テスト ---

/// /api/v1/auth/csrf リクエストを作成
fn csrf_request(session_cookie: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/csrf")
        .header("X-Tenant-ID", test_tenant_id().to_string())
        .header("Cookie", format!("session_id={}", session_cookie))
        .body(Body::empty())
        .unwrap()
}

/// CSRF トークン付きログアウトリクエストを作成
fn logout_request_with_csrf(session_cookie: &str, csrf_token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/logout")
        .header("X-Tenant-ID", test_tenant_id().to_string())
        .header("Cookie", format!("session_id={}", session_cookie))
        .header("X-CSRF-Token", csrf_token)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn test_csrfトークン_ログイン成功時に生成される() {
    // Given
    let (sut, state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // When: ログイン
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();

    // Then: ログイン成功
    assert_eq!(login_response.status(), StatusCode::OK);

    // セッション ID を取得
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id(set_cookie).expect("セッション ID が設定されていない");

    // CSRF トークンが Redis に存在することを確認
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let csrf_token = state
        .session_manager
        .get_csrf_token(&tenant_id, &session_id)
        .await
        .unwrap();
    assert!(csrf_token.is_some(), "CSRF トークンが生成されていない");
    assert_eq!(csrf_token.unwrap().len(), 64);

    // クリーンアップ
    let _ = state.session_manager.delete(&tenant_id, &session_id).await;
    let _ = state
        .session_manager
        .delete_csrf_token(&tenant_id, &session_id)
        .await;
}

#[tokio::test]
async fn test_csrfトークン_get_auth_csrfで取得できる() {
    // Given
    let (sut, state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // ログイン
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id(set_cookie).unwrap();

    // When: GET /api/v1/auth/csrf でトークンを取得
    let csrf_response = sut
        .clone()
        .oneshot(csrf_request(&session_id))
        .await
        .unwrap();

    // Then: トークンが返される
    assert_eq!(csrf_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(csrf_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let token = json["data"]["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);

    // クリーンアップ
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let _ = state.session_manager.delete(&tenant_id, &session_id).await;
    let _ = state
        .session_manager
        .delete_csrf_token(&tenant_id, &session_id)
        .await;
}

#[tokio::test]
async fn test_csrfトークン_正しいトークンでpostリクエストが成功する() {
    // Given
    let (sut, state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // ログイン
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id(set_cookie).unwrap();

    // CSRF トークンを取得
    let csrf_response = sut
        .clone()
        .oneshot(csrf_request(&session_id))
        .await
        .unwrap();
    let body = axum::body::to_bytes(csrf_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let csrf_token = json["data"]["token"].as_str().unwrap();

    // When: 正しい CSRF トークンでログアウト
    let logout_response = sut
        .clone()
        .oneshot(logout_request_with_csrf(&session_id, csrf_token))
        .await
        .unwrap();

    // Then: ログアウト成功
    assert_eq!(logout_response.status(), StatusCode::NO_CONTENT);

    // クリーンアップ
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let _ = state.session_manager.delete(&tenant_id, &session_id).await;
    let _ = state
        .session_manager
        .delete_csrf_token(&tenant_id, &session_id)
        .await;
}

#[tokio::test]
async fn test_csrfトークン_トークンなしでpostリクエストが403になる() {
    // Given
    let (sut, state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // ログイン
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id(set_cookie).unwrap();

    // When: CSRF トークンなしでログアウト
    let logout_response = sut
        .clone()
        .oneshot(logout_request(&session_id))
        .await
        .unwrap();

    // Then: 403 Forbidden
    assert_eq!(logout_response.status(), StatusCode::FORBIDDEN);

    // エラーメッセージを確認
    let body = axum::body::to_bytes(logout_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["detail"].as_str().unwrap().contains("CSRF"));

    // クリーンアップ
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let _ = state.session_manager.delete(&tenant_id, &session_id).await;
    let _ = state
        .session_manager
        .delete_csrf_token(&tenant_id, &session_id)
        .await;
}

#[tokio::test]
async fn test_csrfトークン_不正なトークンでpostリクエストが403になる() {
    // Given
    let (sut, state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // ログイン
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id(set_cookie).unwrap();

    // When: 不正な CSRF トークンでログアウト
    let logout_response = sut
        .clone()
        .oneshot(logout_request_with_csrf(&session_id, "invalid_csrf_token"))
        .await
        .unwrap();

    // Then: 403 Forbidden
    assert_eq!(logout_response.status(), StatusCode::FORBIDDEN);

    // エラーメッセージを確認
    let body = axum::body::to_bytes(logout_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["detail"].as_str().unwrap().contains("CSRF"));

    // クリーンアップ
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let _ = state.session_manager.delete(&tenant_id, &session_id).await;
    let _ = state
        .session_manager
        .delete_csrf_token(&tenant_id, &session_id)
        .await;
}

#[tokio::test]
async fn test_csrfトークン_ログアウト時に削除される() {
    // Given
    let (sut, state) = create_test_app(
        CoreServiceStubConfig::success(),
        AuthServiceStubConfig::success(),
    )
    .await;

    // ログイン
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let session_id = extract_session_id(set_cookie).unwrap();

    // CSRF トークンを取得
    let csrf_response = sut
        .clone()
        .oneshot(csrf_request(&session_id))
        .await
        .unwrap();
    let body = axum::body::to_bytes(csrf_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let csrf_token = json["data"]["token"].as_str().unwrap();

    // CSRF トークンが存在することを確認
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let token_before = state
        .session_manager
        .get_csrf_token(&tenant_id, &session_id)
        .await
        .unwrap();
    assert!(token_before.is_some());

    // When: ログアウト
    let logout_response = sut
        .clone()
        .oneshot(logout_request_with_csrf(&session_id, csrf_token))
        .await
        .unwrap();
    assert_eq!(logout_response.status(), StatusCode::NO_CONTENT);

    // Then: CSRF トークンが削除されている
    let token_after = state
        .session_manager
        .get_csrf_token(&tenant_id, &session_id)
        .await
        .unwrap();
    assert!(token_after.is_none(), "CSRF トークンが削除されていない");
}
