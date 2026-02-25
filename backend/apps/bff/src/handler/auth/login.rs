//! ログイン・ログアウトハンドラ

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::SessionData;
use ringiflow_shared::{ApiResponse, ErrorResponse, event_log::event, log_business_event};
use uuid::Uuid;

use super::{
    AuthState,
    LoginRequest,
    LoginResponseData,
    LoginUserResponse,
    SESSION_COOKIE_NAME,
    build_clear_cookie,
    build_session_cookie,
};
use crate::{
    client::{AuthServiceError, CoreServiceError},
    error::{
        authentication_failed_response,
        extract_tenant_id,
        internal_error_response,
        service_unavailable_response,
    },
};

/// POST /api/v1/auth/login
///
/// メール/パスワードでログインし、セッションを確立する。
///
/// ## 認証フロー
///
/// 1. Core API でユーザーを検索（`GET /internal/users/by-email`）
/// 2. Auth Service でパスワードを検証（`POST /internal/auth/verify`）
/// 3. セッションを作成し Cookie を設定
///
/// ## タイミング攻撃対策
///
/// ユーザーが存在しない場合も Auth Service にダミーリクエストを送信し、
/// 処理時間を均一化してユーザー存在確認攻撃を防ぐ。
///
/// ## ヘッダー
///
/// - `X-Tenant-ID`: テナント ID（必須）
///
/// ## リクエストボディ
///
/// ```json
/// {
///   "email": "user@example.com",
///   "password": "password123"
/// }
/// ```
#[utoipa::path(
   post,
   path = "/api/v1/auth/login",
   tag = "auth",
   request_body = LoginRequest,
   responses(
      (status = 200, description = "ログイン成功", body = ApiResponse<LoginResponseData>),
      (status = 401, description = "認証失敗", body = ErrorResponse),
      (status = 503, description = "サービス利用不可", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn login(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // X-Tenant-ID ヘッダーからテナント ID を取得
    let tenant_id = match extract_tenant_id(&headers) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    // Step 1: Core API でユーザーを検索
    let user_response = match state
        .core_service_client
        .get_user_by_email(tenant_id, &req.email)
        .await
    {
        Ok(resp) => resp,
        Err(CoreServiceError::UserNotFound) => {
            // タイミング攻撃対策: ユーザーが存在しない場合もダミー検証を実行
            // Auth Service にダミーの user_id を送信して処理時間を均一化
            let dummy_user_id = Uuid::nil();
            let _ = state
                .auth_service_client
                .verify_password(tenant_id, dummy_user_id, &req.password)
                .await;

            log_business_event!(
                event.category = event::category::AUTH,
                event.action = event::action::LOGIN_FAILURE,
                event.entity_type = event::entity_type::USER,
                event.entity_id = ringiflow_domain::REDACTED,
                event.tenant_id = %tenant_id,
                event.result = event::result::FAILURE,
                event.reason = "user_not_found",
                "ログイン失敗: ユーザー不存在"
            );
            return authentication_failed_response();
        }
        Err(e) => {
            tracing::error!(
                error.category = "external_service",
                error.kind = "user_lookup",
                "ユーザー検索で内部エラー: {}",
                e
            );
            return internal_error_response();
        }
    };

    let user = &user_response.data;

    // Step 2: Auth Service でパスワードを検証
    if let Err(e) = state
        .auth_service_client
        .verify_password(user.tenant_id, user.id, &req.password)
        .await
    {
        return match e {
            AuthServiceError::AuthenticationFailed => {
                log_business_event!(
                    event.category = event::category::AUTH,
                    event.action = event::action::LOGIN_FAILURE,
                    event.entity_type = event::entity_type::USER,
                    event.entity_id = %user.id,
                    event.tenant_id = %user.tenant_id,
                    event.result = event::result::FAILURE,
                    event.reason = "password_mismatch",
                    "ログイン失敗: パスワード不一致"
                );
                authentication_failed_response()
            }
            AuthServiceError::ServiceUnavailable => service_unavailable_response(),
            e => {
                tracing::error!(
                    error.category = "external_service",
                    error.kind = "password_verification",
                    "パスワード検証で内部エラー: {}",
                    e
                );
                internal_error_response()
            }
        };
    }

    // Step 3: ロール情報を取得（get_user で権限付きで取得）
    let user_with_roles = match state.core_service_client.get_user(user.id).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(
                error.category = "external_service",
                error.kind = "user_lookup",
                "ユーザー情報取得で内部エラー: {}",
                e
            );
            return internal_error_response();
        }
    };

    // Step 4: セッションを作成（ロールと権限をキャッシュ）
    let session_data = SessionData::new(
        ringiflow_domain::user::UserId::from_uuid(user.id),
        TenantId::from_uuid(user.tenant_id),
        user.email.clone(),
        user.name.clone(),
        user_with_roles.data.roles.clone(),
        user_with_roles.data.permissions.clone(),
    );

    let session_id = match state.session_manager.create(&session_data).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(
                error.category = "infrastructure",
                error.kind = "session",
                "セッション作成に失敗: {}",
                e
            );
            return internal_error_response();
        }
    };

    // CSRF トークンを作成
    let tenant_id = TenantId::from_uuid(user.tenant_id);
    if let Err(e) = state
        .session_manager
        .create_csrf_token(&tenant_id, &session_id)
        .await
    {
        tracing::error!(
            error.category = "infrastructure",
            error.kind = "csrf_token",
            "CSRF トークン作成に失敗: {}",
            e
        );
        return internal_error_response();
    }

    // Cookie を設定
    let cookie = build_session_cookie(&session_id);
    let jar = jar.add(cookie);

    // レスポンスを返す
    let response = ApiResponse::new(LoginResponseData {
        user: LoginUserResponse {
            id:        user.id,
            email:     user.email.clone(),
            name:      user.name.clone(),
            tenant_id: user.tenant_id,
            roles:     user_with_roles.data.roles,
        },
    });

    log_business_event!(
        event.category = event::category::AUTH,
        event.action = event::action::LOGIN_SUCCESS,
        event.entity_type = event::entity_type::SESSION,
        event.entity_id = %session_id,
        event.actor_id = %user.id,
        event.tenant_id = %user.tenant_id,
        event.result = event::result::SUCCESS,
        "ログイン成功"
    );

    (jar, Json(response)).into_response()
}

/// POST /api/v1/auth/logout
///
/// セッションを無効化してログアウトする。
#[utoipa::path(
   post,
   path = "/api/v1/auth/logout",
   tag = "auth",
   security(("session_auth" = [])),
   responses(
      (status = 204, description = "ログアウト成功"),
      (status = 401, description = "未認証", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn logout(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> impl IntoResponse {
    // X-Tenant-ID ヘッダーからテナント ID を取得
    let tenant_id = match extract_tenant_id(&headers) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    // Cookie からセッション ID を取得
    if let Some(session_cookie) = jar.get(SESSION_COOKIE_NAME) {
        let session_id = session_cookie.value();
        let tenant_id = TenantId::from_uuid(tenant_id);

        // CSRF トークンを削除（エラーは無視）
        if let Err(e) = state
            .session_manager
            .delete_csrf_token(&tenant_id, session_id)
            .await
        {
            tracing::warn!("CSRF トークン削除に失敗（無視）: {}", e);
        }

        // セッションを削除（エラーは無視）
        if let Err(e) = state.session_manager.delete(&tenant_id, session_id).await {
            tracing::warn!("セッション削除に失敗（無視）: {}", e);
        }
    }

    log_business_event!(
        event.category = event::category::AUTH,
        event.action = event::action::LOGOUT,
        event.entity_type = event::entity_type::SESSION,
        event.tenant_id = %tenant_id,
        event.result = event::result::SUCCESS,
        "ログアウト"
    );

    // Cookie をクリア
    let cookie = build_clear_cookie();
    let jar = jar.add(cookie);

    (jar, StatusCode::NO_CONTENT).into_response()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::super::test_utils::*;

    #[tokio::test]
    async fn test_login_成功時にセッションcookieが設定される() {
        // Given
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::success(),
            StubSessionManager::new(),
        );

        let body = serde_json::json!({
            "email": "user@example.com",
            "password": "password123"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);

        // Set-Cookie ヘッダーを確認
        let set_cookie = response.headers().get("set-cookie");
        assert!(set_cookie.is_some());
        let cookie_value = set_cookie.unwrap().to_str().unwrap();
        assert!(cookie_value.contains("session_id="));
        assert!(cookie_value.contains("HttpOnly"));
    }

    #[tokio::test]
    async fn test_login_成功時にユーザー情報が返る() {
        // Given
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::success(),
            StubSessionManager::new(),
        );

        let body = serde_json::json!({
            "email": "user@example.com",
            "password": "password123"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json["data"]["user"]["id"].is_string());
        assert_eq!(json["data"]["user"]["email"], "user@example.com");
        assert_eq!(json["data"]["user"]["name"], "Test User");
    }

    #[tokio::test]
    async fn test_login_パスワード不一致で401() {
        // Given
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::auth_failed(),
            StubSessionManager::new(),
        );

        let body = serde_json::json!({
            "email": "user@example.com",
            "password": "wrongpassword"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_ユーザー不存在で401() {
        // Given
        let sut = create_test_app(
            StubCoreServiceClient::user_not_found(),
            StubAuthServiceClient::auth_failed(),
            StubSessionManager::new(),
        );

        let body = serde_json::json!({
            "email": "notfound@example.com",
            "password": "password123"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_logout_セッションが削除されてcookieがクリアされる() {
        // Given
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::success(),
            StubSessionManager::new(),
        );

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/logout")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("Cookie", "session_id=test-session-id")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Cookie がクリアされていることを確認
        let set_cookie = response.headers().get("set-cookie");
        assert!(set_cookie.is_some());
        let cookie_value = set_cookie.unwrap().to_str().unwrap();
        assert!(cookie_value.contains("session_id="));
        assert!(cookie_value.contains("Max-Age=0"));
    }

    #[tokio::test]
    #[allow(non_snake_case)]
    async fn test_login_テナントIDヘッダーなしで400() {
        // Given
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::success(),
            StubSessionManager::new(),
        );

        let body = serde_json::json!({
            "email": "user@example.com",
            "password": "password123"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json")
            // X-Tenant-ID ヘッダーなし
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
