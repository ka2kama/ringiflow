//! # 認証ハンドラ
//!
//! BFF の認証エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `POST /api/v1/auth/login` - ログイン
//! - `POST /api/v1/auth/logout` - ログアウト
//! - `GET /api/v1/auth/me` - 現在のユーザー情報を取得
//! - `GET /api/v1/auth/csrf` - CSRF トークン取得
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/08_AuthService設計.md)

mod login;
mod session;

use std::sync::Arc;

pub use login::*;
use ringiflow_infra::SessionManager;
use serde::{Deserialize, Serialize};
pub use session::*;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::client::{AuthServiceClient, CoreServiceUserClient, UserWithPermissionsData};

/// 認証ハンドラの共有状態
pub struct AuthState {
    pub core_service_client: Arc<dyn CoreServiceUserClient>,
    pub auth_service_client: Arc<dyn AuthServiceClient>,
    pub session_manager:     Arc<dyn SessionManager>,
}

// --- リクエスト/レスポンス型 ---

/// ログインリクエスト
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email:    String,
    pub password: String,
}

/// ログインレスポンスデータ
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponseData {
    pub user: LoginUserResponse,
}

/// ログインユーザー情報
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginUserResponse {
    pub id:        Uuid,
    pub email:     String,
    pub name:      String,
    pub tenant_id: Uuid,
    pub roles:     Vec<String>,
}

/// 現在のユーザー情報データ
#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponseData {
    pub id:          Uuid,
    pub email:       String,
    pub name:        String,
    pub tenant_id:   Uuid,
    pub tenant_name: String,
    pub roles:       Vec<String>,
    pub permissions: Vec<String>,
}

impl From<UserWithPermissionsData> for MeResponseData {
    fn from(res: UserWithPermissionsData) -> Self {
        Self {
            id:          res.user.id,
            email:       res.user.email,
            name:        res.user.name,
            tenant_id:   res.user.tenant_id,
            tenant_name: res.tenant_name,
            roles:       res.roles,
            permissions: res.permissions,
        }
    }
}

/// CSRF トークンデータ
#[derive(Debug, Serialize, ToSchema)]
pub struct CsrfResponseData {
    pub token: String,
}

// --- 共有定数 ---

/// Cookie 名
const SESSION_COOKIE_NAME: &str = "session_id";

/// セッション有効期限（秒）
const SESSION_MAX_AGE: i64 = 28800; // 8時間

// --- Cookie ヘルパー ---

/// セッション Cookie を構築する
fn build_session_cookie(session_id: &str) -> axum_extra::extract::cookie::Cookie<'static> {
    use axum_extra::extract::cookie::SameSite;

    // 本番環境では Secure フラグを有効にする
    // ENV=production の場合に HTTPS 必須となる
    let is_production = std::env::var("ENV").unwrap_or_default() == "production";

    let mut builder =
        axum_extra::extract::cookie::Cookie::build((SESSION_COOKIE_NAME, session_id.to_string()))
            .path("/")
            .max_age(time::Duration::seconds(SESSION_MAX_AGE))
            .http_only(true)
            .same_site(SameSite::Lax);

    if is_production {
        builder = builder.secure(true);
    }

    builder.build()
}

/// Cookie をクリアするための Cookie を構築する
fn build_clear_cookie() -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .max_age(time::Duration::seconds(0))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .build()
}

// --- テストユーティリティ ---

#[cfg(test)]
pub(super) mod test_utils {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{
        Router,
        routing::{get, post},
    };
    use ringiflow_domain::{tenant::TenantId, user::UserId};
    use ringiflow_infra::{InfraError, SessionData, SessionManager};
    use ringiflow_shared::ApiResponse;
    use uuid::Uuid;

    use super::{AuthState, csrf, login, logout, me};
    use crate::client::{
        AuthServiceClient,
        AuthServiceError,
        CoreServiceError,
        CoreServiceUserClient,
        UserResponse,
        UserWithPermissionsData,
        VerifyResponse,
    };

    pub struct StubCoreServiceClient {
        pub user_by_email_result: Result<ApiResponse<UserResponse>, CoreServiceError>,
        pub get_user_result:      Result<ApiResponse<UserWithPermissionsData>, CoreServiceError>,
    }

    impl StubCoreServiceClient {
        pub fn success() -> Self {
            let user = UserResponse {
                id:        Uuid::now_v7(),
                tenant_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                email:     "user@example.com".to_string(),
                name:      "Test User".to_string(),
                status:    "active".to_string(),
            };
            Self {
                user_by_email_result: Ok(ApiResponse::new(user.clone())),
                get_user_result:      Ok(ApiResponse::new(UserWithPermissionsData {
                    user,
                    tenant_name: "Development Tenant".to_string(),
                    roles: vec!["user".to_string()],
                    permissions: vec!["workflow:read".to_string()],
                })),
            }
        }

        pub fn user_not_found() -> Self {
            Self {
                user_by_email_result: Err(CoreServiceError::UserNotFound),
                get_user_result:      Err(CoreServiceError::UserNotFound),
            }
        }
    }

    #[async_trait]
    impl CoreServiceUserClient for StubCoreServiceClient {
        async fn list_users(
            &self,
            _tenant_id: Uuid,
            _status: Option<&str>,
        ) -> Result<ApiResponse<Vec<crate::client::UserItemDto>>, CoreServiceError> {
            unimplemented!("list_users is not used in auth tests")
        }

        async fn get_user_by_email(
            &self,
            _tenant_id: Uuid,
            _email: &str,
        ) -> Result<ApiResponse<UserResponse>, CoreServiceError> {
            self.user_by_email_result.clone()
        }

        async fn get_user(
            &self,
            _user_id: Uuid,
        ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError> {
            self.get_user_result.clone()
        }

        async fn create_user(
            &self,
            _req: &crate::client::CreateUserCoreRequest,
        ) -> Result<ApiResponse<crate::client::CreateUserCoreResponse>, CoreServiceError> {
            unimplemented!("create_user is not used in auth tests")
        }

        async fn update_user(
            &self,
            _user_id: Uuid,
            _req: &crate::client::UpdateUserCoreRequest,
        ) -> Result<ApiResponse<UserResponse>, CoreServiceError> {
            unimplemented!("update_user is not used in auth tests")
        }

        async fn update_user_status(
            &self,
            _user_id: Uuid,
            _req: &crate::client::UpdateUserStatusCoreRequest,
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

    pub struct StubAuthServiceClient {
        pub verify_result: Result<VerifyResponse, AuthServiceError>,
    }

    impl StubAuthServiceClient {
        pub fn success() -> Self {
            Self {
                verify_result: Ok(VerifyResponse {
                    verified:      true,
                    credential_id: Some(Uuid::now_v7()),
                }),
            }
        }

        pub fn auth_failed() -> Self {
            Self {
                verify_result: Err(AuthServiceError::AuthenticationFailed),
            }
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
            self.verify_result.clone()
        }

        async fn create_credentials(
            &self,
            _tenant_id: Uuid,
            _user_id: Uuid,
            _credential_type: &str,
            _credential_data: &str,
        ) -> Result<crate::client::auth_service::CreateCredentialsResponse, AuthServiceError>
        {
            unimplemented!("create_credentials is not used in auth tests")
        }
    }

    pub struct StubSessionManager {
        pub session: Option<SessionData>,
    }

    impl StubSessionManager {
        pub fn new() -> Self {
            Self { session: None }
        }

        pub fn with_session(user_id: UserId, tenant_id: TenantId) -> Self {
            Self {
                session: Some(SessionData::new(
                    user_id,
                    tenant_id,
                    "user@example.com".to_string(),
                    "Test User".to_string(),
                    vec!["user".to_string()],
                    vec!["workflow:read".to_string()],
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
            if self.session.is_some() {
                Ok(Some("a".repeat(64)))
            } else {
                Ok(None)
            }
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

    pub fn create_test_app(
        core_client: StubCoreServiceClient,
        auth_client: StubAuthServiceClient,
        session_manager: StubSessionManager,
    ) -> Router {
        let state = Arc::new(AuthState {
            core_service_client: Arc::new(core_client),
            auth_service_client: Arc::new(auth_client),
            session_manager:     Arc::new(session_manager),
        });

        Router::new()
            .route("/api/v1/auth/login", post(login))
            .route("/api/v1/auth/logout", post(logout))
            .route("/api/v1/auth/me", get(me))
            .route("/api/v1/auth/csrf", get(csrf))
            .with_state(state)
    }

    pub const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";
}
