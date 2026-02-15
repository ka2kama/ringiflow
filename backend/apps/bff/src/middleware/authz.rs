//! # 認可ミドルウェア
//!
//! セッションの権限を検証し、RBAC ベースのアクセス制御を実現する。
//!
//! ## 使い方
//!
//! ```rust,ignore
//! use axum::middleware::from_fn_with_state;
//!
//! let authz_state = AuthzState {
//!     session_manager: session_manager.clone(),
//!     required_permission: "user:read".to_string(),
//! };
//!
//! Router::new()
//!     .route("/api/v1/users", get(list_users))
//!     .layer(from_fn_with_state(authz_state, require_permission))
//! ```

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::role::Permission;
use ringiflow_infra::SessionManager;

use crate::error::{extract_tenant_id, forbidden_response, get_session};

/// 認可ミドルウェアの状態
#[derive(Clone)]
pub struct AuthzState {
    pub session_manager:     Arc<dyn SessionManager>,
    pub required_permission: String,
}

/// 認可ミドルウェア
///
/// セッションから権限を取得し、要求された権限を満たすか検証する。
/// 権限が不足している場合は 403 Forbidden を返す。
/// セッションが存在しない場合は 401 Unauthorized を返す。
pub async fn require_permission(
    State(state): State<AuthzState>,
    jar: CookieJar,
    request: Request<Body>,
    next: Next,
) -> Response {
    // テナント ID を取得
    let tenant_id = match extract_tenant_id(request.headers()) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    // セッションを取得
    let session = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
        Ok(s) => s,
        Err(response) => return response,
    };

    // 権限チェック
    let required = Permission::new(&state.required_permission);
    let has_permission = session
        .permissions()
        .iter()
        .any(|p| Permission::new(p).satisfies(&required));

    if !has_permission {
        return forbidden_response("この操作を実行する権限がありません");
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Method, Request, StatusCode},
        middleware::from_fn_with_state,
        response::IntoResponse,
        routing::get,
    };
    use ringiflow_domain::{tenant::TenantId, user::UserId};
    use ringiflow_infra::{InfraError, SessionData, SessionManager};
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;

    const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";

    /// テスト用のダミーハンドラ
    async fn dummy_handler() -> impl IntoResponse {
        StatusCode::OK
    }

    /// テスト用スタブ SessionManager
    struct StubSessionManager {
        session: Option<SessionData>,
    }

    impl StubSessionManager {
        fn new() -> Self {
            Self { session: None }
        }

        fn with_permissions(permissions: Vec<String>) -> Self {
            let tenant_id = TenantId::from_uuid(Uuid::parse_str(TEST_TENANT_ID).unwrap());
            Self {
                session: Some(SessionData::new(
                    UserId::new(),
                    tenant_id,
                    "user@example.com".to_string(),
                    "Test User".to_string(),
                    vec!["user".to_string()],
                    permissions,
                )),
            }
        }
    }

    #[async_trait]
    impl SessionManager for StubSessionManager {
        async fn create(&self, _data: &SessionData) -> Result<String, InfraError> {
            Ok(Uuid::now_v7().to_string())
        }

        async fn create_with_id(
            &self,
            _session_id: &str,
            _data: &SessionData,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn get(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<Option<SessionData>, InfraError> {
            Ok(self.session.clone())
        }

        async fn delete(&self, _tenant_id: &TenantId, _session_id: &str) -> Result<(), InfraError> {
            Ok(())
        }

        async fn delete_all_for_tenant(&self, _tenant_id: &TenantId) -> Result<(), InfraError> {
            Ok(())
        }

        async fn get_ttl(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<Option<i64>, InfraError> {
            Ok(Some(28800))
        }

        async fn create_csrf_token(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<String, InfraError> {
            Ok("a".repeat(64))
        }

        async fn get_csrf_token(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<Option<String>, InfraError> {
            Ok(None)
        }

        async fn delete_csrf_token(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn delete_all_csrf_for_tenant(
            &self,
            _tenant_id: &TenantId,
        ) -> Result<(), InfraError> {
            Ok(())
        }
    }

    fn create_test_app(session_manager: StubSessionManager, required_permission: &str) -> Router {
        let authz_state = AuthzState {
            session_manager:     Arc::new(session_manager),
            required_permission: required_permission.to_string(),
        };

        Router::new()
            .route("/test", get(dummy_handler))
            .layer(from_fn_with_state(authz_state, require_permission))
    }

    #[tokio::test]
    async fn test_権限を持つユーザーはリクエストが通過する() {
        // Given
        let sut = create_test_app(
            StubSessionManager::with_permissions(vec!["user:read".to_string()]),
            "user:read",
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("Cookie", "session_id=test-session-id")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_権限を持たないユーザーは403を返す() {
        // Given
        let sut = create_test_app(
            StubSessionManager::with_permissions(vec!["workflow:read".to_string()]),
            "user:read",
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("Cookie", "session_id=test-session-id")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_ワイルドカード権限は任意のリクエストを通過させる() {
        // Given
        let sut = create_test_app(
            StubSessionManager::with_permissions(vec!["*".to_string()]),
            "user:read",
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("Cookie", "session_id=test-session-id")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_セッションなしは401を返す() {
        // Given
        let sut = create_test_app(StubSessionManager::new(), "user:read");

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("Cookie", "session_id=nonexistent")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
