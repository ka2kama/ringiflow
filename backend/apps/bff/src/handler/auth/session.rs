//! セッション参照ハンドラ（me, csrf）

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::tenant::TenantId;
use ringiflow_shared::{ApiResponse, ErrorResponse};

use super::{AuthState, CsrfResponseData, MeResponseData, SESSION_COOKIE_NAME};
use crate::{
    client::CoreServiceError,
    error::{extract_tenant_id, internal_error_response, unauthorized_response},
};

/// GET /api/v1/auth/me
///
/// 現在のユーザー情報と権限を取得する。
#[utoipa::path(
   get,
   path = "/api/v1/auth/me",
   tag = "auth",
   security(("session_auth" = [])),
   responses(
      (status = 200, description = "ユーザー情報", body = ApiResponse<MeResponseData>),
      (status = 401, description = "未認証", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn me(
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
    let session_id = match jar.get(SESSION_COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        None => return unauthorized_response(),
    };

    let tenant_id = TenantId::from_uuid(tenant_id);

    // セッションを取得
    match state.session_manager.get(&tenant_id, &session_id).await {
        Ok(Some(session_data)) => {
            // Core API からユーザー情報を取得
            let user_id = *session_data.user_id().as_uuid();
            match state.core_service_client.get_user(user_id).await {
                Ok(user_info) => {
                    let response = ApiResponse::new(MeResponseData::from(user_info.data));
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(CoreServiceError::UserNotFound) => {
                    // ユーザーが削除された場合
                    unauthorized_response()
                }
                Err(e) => {
                    tracing::error!("ユーザー情報取得で内部エラー: {}", e);
                    internal_error_response()
                }
            }
        }
        Ok(None) => unauthorized_response(),
        Err(e) => {
            tracing::error!("セッション取得で内部エラー: {}", e);
            internal_error_response()
        }
    }
}

/// GET /api/v1/auth/csrf
///
/// CSRF トークンを取得する。
/// セッションが存在しない場合は新規作成し、存在する場合は既存のトークンを返す。
#[utoipa::path(
   get,
   path = "/api/v1/auth/csrf",
   tag = "auth",
   security(("session_auth" = [])),
   responses(
      (status = 200, description = "CSRF トークン", body = ApiResponse<CsrfResponseData>),
      (status = 401, description = "未認証", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn csrf(
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
    let session_id = match jar.get(SESSION_COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        None => return unauthorized_response(),
    };

    let tenant_id = TenantId::from_uuid(tenant_id);

    // セッションが存在するか確認
    match state.session_manager.get(&tenant_id, &session_id).await {
        Ok(Some(_)) => {
            // 既存の CSRF トークンを取得、なければ新規作成
            let token = match state
                .session_manager
                .get_csrf_token(&tenant_id, &session_id)
                .await
            {
                Ok(Some(token)) => token,
                Ok(None) => {
                    // トークンが存在しない場合は新規作成
                    match state
                        .session_manager
                        .create_csrf_token(&tenant_id, &session_id)
                        .await
                    {
                        Ok(token) => token,
                        Err(e) => {
                            tracing::error!("CSRF トークン作成で内部エラー: {}", e);
                            return internal_error_response();
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("CSRF トークン取得で内部エラー: {}", e);
                    return internal_error_response();
                }
            };

            let response = ApiResponse::new(CsrfResponseData { token });
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => unauthorized_response(),
        Err(e) => {
            tracing::error!("セッション取得で内部エラー: {}", e);
            internal_error_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use ringiflow_domain::{tenant::TenantId, user::UserId};
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::super::test_utils::*;

    #[tokio::test]
    async fn test_me_認証済みでユーザー情報が返る() {
        // Given
        let tenant_id = TenantId::from_uuid(Uuid::parse_str(TEST_TENANT_ID).unwrap());
        let user_id = UserId::new();
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::success(),
            StubSessionManager::with_session(user_id, tenant_id),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("Cookie", "session_id=test-session-id")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json["data"]["id"].is_string());
        assert_eq!(json["data"]["email"], "user@example.com");
        assert_eq!(json["data"]["tenant_name"], "Development Tenant");
        assert!(json["data"]["roles"].is_array());
        assert!(json["data"]["permissions"].is_array());
    }

    #[tokio::test]
    async fn test_me_未認証で401() {
        // Given
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::success(),
            StubSessionManager::new(),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/me")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            // Cookie なし
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // --- CSRF トークンテスト ---

    #[tokio::test]
    async fn test_csrf_認証済みでトークンを取得できる() {
        // Given
        let tenant_id = TenantId::from_uuid(Uuid::parse_str(TEST_TENANT_ID).unwrap());
        let user_id = UserId::new();
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::success(),
            StubSessionManager::with_session(user_id, tenant_id),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/csrf")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("Cookie", "session_id=test-session-id")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json["data"]["token"].is_string());
        let token = json["data"]["token"].as_str().unwrap();
        assert_eq!(token.len(), 64);
    }

    #[tokio::test]
    async fn test_csrf_未認証で401() {
        // Given
        let sut = create_test_app(
            StubCoreServiceClient::success(),
            StubAuthServiceClient::success(),
            StubSessionManager::new(),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/csrf")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            // Cookie なし
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
