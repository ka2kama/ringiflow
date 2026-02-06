//! # 認証ハンドラ
//!
//! BFF の認証エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `POST /api/v1/auth/login` - ログイン
//! - `POST /api/v1/auth/logout` - ログアウト
//! - `GET /api/v1/auth/me` - 現在のユーザー情報を取得
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/08_AuthService設計.md)

use std::sync::Arc;

use axum::{
   Json,
   extract::State,
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::{SessionData, SessionManager};
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
   client::{
      AuthServiceClient,
      AuthServiceError,
      CoreServiceClient,
      CoreServiceError,
      UserWithPermissionsData,
   },
   error::{
      authentication_failed_response,
      extract_tenant_id,
      internal_error_response,
      service_unavailable_response,
      unauthorized_response,
   },
};

/// Cookie 名
const SESSION_COOKIE_NAME: &str = "session_id";

/// セッション有効期限（秒）
const SESSION_MAX_AGE: i64 = 28800; // 8時間

/// 認証ハンドラの共有状態
pub struct AuthState<C, A, S>
where
   C: CoreServiceClient,
   A: AuthServiceClient,
   S: SessionManager,
{
   pub core_service_client: C,
   pub auth_service_client: A,
   pub session_manager:     S,
}

// --- リクエスト/レスポンス型 ---

/// ログインリクエスト
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
   pub email:    String,
   pub password: String,
}

/// ログインレスポンスデータ
#[derive(Debug, Serialize)]
pub struct LoginResponseData {
   pub user: LoginUserResponse,
}

/// ログインユーザー情報
#[derive(Debug, Serialize)]
pub struct LoginUserResponse {
   pub id:        Uuid,
   pub email:     String,
   pub name:      String,
   pub tenant_id: Uuid,
   pub roles:     Vec<String>,
}

/// 現在のユーザー情報データ
#[derive(Debug, Serialize)]
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
         // TODO(#34): Core API にテナント情報取得エンドポイントを追加して取得
         tenant_name: "Development Tenant".to_string(),
         roles:       res.roles,
         permissions: res.permissions,
      }
   }
}

/// CSRF トークンデータ
#[derive(Debug, Serialize)]
pub struct CsrfResponseData {
   pub token: String,
}

// --- ハンドラ ---

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
pub async fn login<C, A, S>(
   State(state): State<Arc<AuthState<C, A, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Json(req): Json<LoginRequest>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   A: AuthServiceClient,
   S: SessionManager,
{
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // Step 1: Core API でユーザーを検索
   let user_result = state
      .core_service_client
      .get_user_by_email(tenant_id, &req.email)
      .await;

   match user_result {
      Ok(user_response) => {
         let user = &user_response.data;

         // Step 2: Auth Service でパスワードを検証
         let verify_result = state
            .auth_service_client
            .verify_password(user.tenant_id, user.id, &req.password)
            .await;

         match verify_result {
            Ok(_) => {
               // Step 3: ロール情報を取得（get_user で権限付きで取得）
               let user_with_roles = match state.core_service_client.get_user(user.id).await {
                  Ok(u) => u,
                  Err(e) => {
                     tracing::error!("ユーザー情報取得で内部エラー: {}", e);
                     return internal_error_response();
                  }
               };

               // Step 4: セッションを作成
               let session_data = SessionData::new(
                  ringiflow_domain::user::UserId::from_uuid(user.id),
                  TenantId::from_uuid(user.tenant_id),
                  user.email.clone(),
                  user.name.clone(),
                  user_with_roles.data.roles.clone(),
               );

               match state.session_manager.create(&session_data).await {
                  Ok(session_id) => {
                     // CSRF トークンを作成
                     let tenant_id = TenantId::from_uuid(user.tenant_id);
                     if let Err(e) = state
                        .session_manager
                        .create_csrf_token(&tenant_id, &session_id)
                        .await
                     {
                        tracing::error!("CSRF トークン作成に失敗: {}", e);
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

                     (jar, Json(response)).into_response()
                  }
                  Err(e) => {
                     tracing::error!("セッション作成に失敗: {}", e);
                     internal_error_response()
                  }
               }
            }
            Err(AuthServiceError::AuthenticationFailed) => authentication_failed_response(),
            Err(AuthServiceError::ServiceUnavailable) => service_unavailable_response(),
            Err(e) => {
               tracing::error!("パスワード検証で内部エラー: {}", e);
               internal_error_response()
            }
         }
      }
      Err(CoreServiceError::UserNotFound) => {
         // タイミング攻撃対策: ユーザーが存在しない場合もダミー検証を実行
         // Auth Service にダミーの user_id を送信して処理時間を均一化
         let dummy_user_id = Uuid::nil();
         let _ = state
            .auth_service_client
            .verify_password(tenant_id, dummy_user_id, &req.password)
            .await;

         authentication_failed_response()
      }
      Err(e) => {
         tracing::error!("ユーザー検索で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// POST /api/v1/auth/logout
///
/// セッションを無効化してログアウトする。
pub async fn logout<C, A, S>(
   State(state): State<Arc<AuthState<C, A, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   A: AuthServiceClient,
   S: SessionManager,
{
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

   // Cookie をクリア
   let cookie = build_clear_cookie();
   let jar = jar.add(cookie);

   (jar, StatusCode::NO_CONTENT).into_response()
}

/// GET /api/v1/auth/me
///
/// 現在のユーザー情報と権限を取得する。
pub async fn me<C, A, S>(
   State(state): State<Arc<AuthState<C, A, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   A: AuthServiceClient,
   S: SessionManager,
{
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
pub async fn csrf<C, A, S>(
   State(state): State<Arc<AuthState<C, A, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   A: AuthServiceClient,
   S: SessionManager,
{
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

// --- ヘルパー関数 ---

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

#[cfg(test)]
mod tests {
   use std::sync::Arc;

   use async_trait::async_trait;
   use axum::{
      Router,
      body::Body,
      http::{Method, Request},
      routing::{get, post},
   };
   use ringiflow_domain::{tenant::TenantId, user::UserId};
   use ringiflow_infra::InfraError;
   use tower::ServiceExt;
   use uuid::Uuid;

   use super::*;
   use crate::client::{UserResponse, VerifyResponse};

   // テスト用スタブ

   struct StubCoreServiceClient {
      user_by_email_result: Result<ApiResponse<UserResponse>, CoreServiceError>,
      get_user_result:      Result<ApiResponse<UserWithPermissionsData>, CoreServiceError>,
   }

   impl StubCoreServiceClient {
      fn success() -> Self {
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
               roles: vec!["user".to_string()],
               permissions: vec!["workflow:read".to_string()],
            })),
         }
      }

      fn user_not_found() -> Self {
         Self {
            user_by_email_result: Err(CoreServiceError::UserNotFound),
            get_user_result:      Err(CoreServiceError::UserNotFound),
         }
      }
   }

   #[async_trait]
   impl CoreServiceClient for StubCoreServiceClient {
      async fn list_users(
         &self,
         _tenant_id: Uuid,
      ) -> Result<ApiResponse<Vec<crate::client::UserItemDto>>, CoreServiceError> {
         // テストスタブでは未使用
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

      async fn create_workflow(
         &self,
         _req: crate::client::CreateWorkflowRequest,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("create_workflow is not used in auth tests")
      }

      async fn submit_workflow(
         &self,
         _workflow_id: Uuid,
         _req: crate::client::SubmitWorkflowRequest,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("submit_workflow is not used in auth tests")
      }

      async fn list_workflow_definitions(
         &self,
         _tenant_id: Uuid,
      ) -> Result<ApiResponse<Vec<crate::client::WorkflowDefinitionDto>>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("list_workflow_definitions is not used in auth tests")
      }

      async fn get_workflow_definition(
         &self,
         _definition_id: Uuid,
         _tenant_id: Uuid,
      ) -> Result<ApiResponse<crate::client::WorkflowDefinitionDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("get_workflow_definition is not used in auth tests")
      }

      async fn list_my_workflows(
         &self,
         _tenant_id: Uuid,
         _user_id: Uuid,
      ) -> Result<ApiResponse<Vec<crate::client::WorkflowInstanceDto>>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("list_my_workflows is not used in auth tests")
      }

      async fn get_workflow(
         &self,
         _workflow_id: Uuid,
         _tenant_id: Uuid,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("get_workflow is not used in auth tests")
      }

      async fn approve_step(
         &self,
         _workflow_id: Uuid,
         _step_id: Uuid,
         _req: crate::client::ApproveRejectRequest,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("approve_step is not used in auth tests")
      }

      async fn reject_step(
         &self,
         _workflow_id: Uuid,
         _step_id: Uuid,
         _req: crate::client::ApproveRejectRequest,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("reject_step is not used in auth tests")
      }

      async fn list_my_tasks(
         &self,
         _tenant_id: Uuid,
         _user_id: Uuid,
      ) -> Result<ApiResponse<Vec<crate::client::TaskItemDto>>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("list_my_tasks is not used in auth tests")
      }

      async fn get_task(
         &self,
         _task_id: Uuid,
         _tenant_id: Uuid,
         _user_id: Uuid,
      ) -> Result<ApiResponse<crate::client::TaskDetailDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("get_task is not used in auth tests")
      }

      async fn get_dashboard_stats(
         &self,
         _tenant_id: Uuid,
         _user_id: Uuid,
      ) -> Result<ApiResponse<crate::client::DashboardStatsDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("get_dashboard_stats is not used in auth tests")
      }

      async fn get_workflow_by_display_number(
         &self,
         _display_number: i64,
         _tenant_id: Uuid,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("get_workflow_by_display_number is not used in auth tests")
      }

      async fn submit_workflow_by_display_number(
         &self,
         _display_number: i64,
         _req: crate::client::SubmitWorkflowRequest,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("submit_workflow_by_display_number is not used in auth tests")
      }

      async fn approve_step_by_display_number(
         &self,
         _workflow_display_number: i64,
         _step_display_number: i64,
         _req: crate::client::ApproveRejectRequest,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("approve_step_by_display_number is not used in auth tests")
      }

      async fn reject_step_by_display_number(
         &self,
         _workflow_display_number: i64,
         _step_display_number: i64,
         _req: crate::client::ApproveRejectRequest,
      ) -> Result<ApiResponse<crate::client::WorkflowInstanceDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("reject_step_by_display_number is not used in auth tests")
      }

      async fn get_task_by_display_numbers(
         &self,
         _workflow_display_number: i64,
         _step_display_number: i64,
         _tenant_id: Uuid,
         _user_id: Uuid,
      ) -> Result<ApiResponse<crate::client::TaskDetailDto>, CoreServiceError> {
         // テストスタブでは未使用
         unimplemented!("get_task_by_display_numbers is not used in auth tests")
      }
   }

   struct StubAuthServiceClient {
      verify_result: Result<VerifyResponse, AuthServiceError>,
   }

   impl StubAuthServiceClient {
      fn success() -> Self {
         Self {
            verify_result: Ok(VerifyResponse {
               verified:      true,
               credential_id: Some(Uuid::now_v7()),
            }),
         }
      }

      fn auth_failed() -> Self {
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
   }

   struct StubSessionManager {
      session: Option<SessionData>,
   }

   impl StubSessionManager {
      fn new() -> Self {
         Self { session: None }
      }

      fn with_session(user_id: UserId, tenant_id: TenantId) -> Self {
         Self {
            session: Some(SessionData::new(
               user_id,
               tenant_id,
               "user@example.com".to_string(),
               "Test User".to_string(),
               vec!["user".to_string()],
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

      async fn delete_all_csrf_for_tenant(&self, _tenant_id: &TenantId) -> Result<(), InfraError> {
         Ok(())
      }
   }

   fn create_test_app(
      core_client: StubCoreServiceClient,
      auth_client: StubAuthServiceClient,
      session_manager: StubSessionManager,
   ) -> Router {
      let state = Arc::new(AuthState {
         core_service_client: core_client,
         auth_service_client: auth_client,
         session_manager,
      });

      Router::new()
         .route(
            "/api/v1/auth/login",
            post(login::<StubCoreServiceClient, StubAuthServiceClient, StubSessionManager>),
         )
         .route(
            "/api/v1/auth/logout",
            post(logout::<StubCoreServiceClient, StubAuthServiceClient, StubSessionManager>),
         )
         .route(
            "/api/v1/auth/me",
            get(me::<StubCoreServiceClient, StubAuthServiceClient, StubSessionManager>),
         )
         .route(
            "/api/v1/auth/csrf",
            get(csrf::<StubCoreServiceClient, StubAuthServiceClient, StubSessionManager>),
         )
         .with_state(state)
   }

   const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";

   // テストケース

   #[tokio::test]
   async fn test_login_成功時にセッションcookieが設定される() {
      // Given
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

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
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

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
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
   }

   #[tokio::test]
   async fn test_login_ユーザー不存在で401() {
      // Given
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
   }

   #[tokio::test]
   async fn test_logout_セッションが削除されてcookieがクリアされる() {
      // Given
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

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
   async fn test_me_認証済みでユーザー情報が返る() {
      // Given
      let tenant_id = TenantId::from_uuid(Uuid::parse_str(TEST_TENANT_ID).unwrap());
      let user_id = UserId::new();
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::OK);

      let body = axum::body::to_bytes(response.into_body(), usize::MAX)
         .await
         .unwrap();
      let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

      assert!(json["data"]["id"].is_string());
      assert_eq!(json["data"]["email"], "user@example.com");
      assert!(json["data"]["roles"].is_array());
      assert!(json["data"]["permissions"].is_array());
   }

   #[tokio::test]
   async fn test_me_未認証で401() {
      // Given
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
   }

   #[tokio::test]
   #[allow(non_snake_case)]
   async fn test_login_テナントIDヘッダーなしで400() {
      // Given
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::BAD_REQUEST);
   }

   // --- CSRF トークンテスト ---

   #[tokio::test]
   async fn test_csrf_認証済みでトークンを取得できる() {
      // Given
      let tenant_id = TenantId::from_uuid(Uuid::parse_str(TEST_TENANT_ID).unwrap());
      let user_id = UserId::new();
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

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
      let app = create_test_app(
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
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
   }
}
