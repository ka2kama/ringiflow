//! # 認証ハンドラ
//!
//! BFF の認証エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `POST /auth/login` - ログイン
//! - `POST /auth/logout` - ログアウト
//! - `GET /auth/me` - 現在のユーザー情報を取得
//!
//! 詳細: [07_認証機能設計.md](../../../../docs/03_詳細設計書/07_認証機能設計.md)

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
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::client::{CoreApiClient, CoreApiError, UserWithPermissionsResponse};

/// Cookie 名
const SESSION_COOKIE_NAME: &str = "session_id";

// --- エラー型 ---

/// テナント ID 抽出エラー
#[derive(Debug)]
pub enum TenantIdError {
   /// ヘッダーが存在しない
   Missing,
   /// UUID の形式が不正
   InvalidFormat,
}

impl IntoResponse for TenantIdError {
   fn into_response(self) -> axum::response::Response {
      let (status, detail) = match self {
         TenantIdError::Missing => (StatusCode::BAD_REQUEST, "X-Tenant-ID ヘッダーが必要です"),
         TenantIdError::InvalidFormat => (StatusCode::BAD_REQUEST, "X-Tenant-ID の形式が不正です"),
      };
      (
         status,
         Json(ErrorResponse {
            error_type: "https://ringiflow.example.com/errors/validation-error".to_string(),
            title:      "Validation Error".to_string(),
            status:     status.as_u16(),
            detail:     detail.to_string(),
         }),
      )
         .into_response()
   }
}

/// セッション有効期限（秒）
const SESSION_MAX_AGE: i64 = 28800; // 8時間

/// 認証ハンドラの共有状態
pub struct AuthState<C, S>
where
   C: CoreApiClient,
   S: SessionManager,
{
   pub core_api_client: C,
   pub session_manager: S,
}

// --- リクエスト/レスポンス型 ---

/// ログインリクエスト
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
   pub email:    String,
   pub password: String,
}

/// ログインレスポンス
#[derive(Debug, Serialize)]
pub struct LoginResponse {
   pub data: LoginResponseData,
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

/// 現在のユーザー情報レスポンス
#[derive(Debug, Serialize)]
pub struct MeResponse {
   pub data: MeResponseData,
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

impl From<UserWithPermissionsResponse> for MeResponseData {
   fn from(res: UserWithPermissionsResponse) -> Self {
      Self {
         id:          res.user.id,
         email:       res.user.email,
         name:        res.user.name,
         tenant_id:   res.user.tenant_id,
         tenant_name: "Development Tenant".to_string(), // TODO: Core API から取得
         roles:       res.roles,
         permissions: res.permissions,
      }
   }
}

/// エラーレスポンス（RFC 7807 Problem Details）
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
   #[serde(rename = "type")]
   pub error_type: String,
   pub title:      String,
   pub status:     u16,
   pub detail:     String,
}

// --- ハンドラ ---

/// POST /auth/login
///
/// メール/パスワードでログインし、セッションを確立する。
///
/// # ヘッダー
///
/// - `X-Tenant-ID`: テナント ID（必須）
///
/// # リクエストボディ
///
/// ```json
/// {
///   "email": "user@example.com",
///   "password": "password123"
/// }
/// ```
pub async fn login<C, S>(
   State(state): State<Arc<AuthState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Json(req): Json<LoginRequest>,
) -> impl IntoResponse
where
   C: CoreApiClient,
   S: SessionManager,
{
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // Core API で認証
   let verify_result = state
      .core_api_client
      .verify_credentials(tenant_id, &req.email, &req.password)
      .await;

   match verify_result {
      Ok(verified) => {
         // セッションを作成
         let session_data = SessionData::new(
            ringiflow_domain::user::UserId::from_uuid(verified.user.id),
            TenantId::from_uuid(verified.user.tenant_id),
            verified.user.email.clone(),
            verified.user.name.clone(),
            verified.roles.iter().map(|r| r.name.clone()).collect(),
         );

         match state.session_manager.create(&session_data).await {
            Ok(session_id) => {
               // Cookie を設定
               let cookie = build_session_cookie(&session_id);
               let jar = jar.add(cookie);

               // レスポンスを返す
               let response = LoginResponse {
                  data: LoginResponseData {
                     user: LoginUserResponse {
                        id:        verified.user.id,
                        email:     verified.user.email,
                        name:      verified.user.name,
                        tenant_id: verified.user.tenant_id,
                        roles:     verified.roles.iter().map(|r| r.name.clone()).collect(),
                     },
                  },
               };

               (jar, Json(response)).into_response()
            }
            Err(e) => {
               tracing::error!("セッション作成に失敗: {}", e);
               internal_error_response()
            }
         }
      }
      Err(CoreApiError::AuthenticationFailed) => authentication_failed_response(),
      Err(e) => {
         tracing::error!("認証処理で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// POST /auth/logout
///
/// セッションを無効化してログアウトする。
pub async fn logout<C, S>(
   State(state): State<Arc<AuthState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse
where
   C: CoreApiClient,
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

/// GET /auth/me
///
/// 現在のユーザー情報と権限を取得する。
pub async fn me<C, S>(
   State(state): State<Arc<AuthState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse
where
   C: CoreApiClient,
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
         match state.core_api_client.get_user(user_id).await {
            Ok(user_info) => {
               let response = MeResponse {
                  data: MeResponseData::from(user_info),
               };
               (StatusCode::OK, Json(response)).into_response()
            }
            Err(CoreApiError::UserNotFound) => {
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

// --- ヘルパー関数 ---

/// X-Tenant-ID ヘッダーからテナント ID を抽出する
fn extract_tenant_id(headers: &HeaderMap) -> Result<Uuid, TenantIdError> {
   let tenant_id_str = headers
      .get("X-Tenant-ID")
      .and_then(|v| v.to_str().ok())
      .ok_or(TenantIdError::Missing)?;

   Uuid::parse_str(tenant_id_str).map_err(|_| TenantIdError::InvalidFormat)
}

/// セッション Cookie を構築する
fn build_session_cookie(session_id: &str) -> axum_extra::extract::cookie::Cookie<'static> {
   axum_extra::extract::cookie::Cookie::build((SESSION_COOKIE_NAME, session_id.to_string()))
      .path("/")
      .max_age(time::Duration::seconds(SESSION_MAX_AGE))
      .http_only(true)
      .same_site(axum_extra::extract::cookie::SameSite::Lax)
      // TODO: 本番環境では Secure を有効にする
      // .secure(true)
      .build()
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

/// 認証失敗レスポンス
fn authentication_failed_response() -> axum::response::Response {
   (
      StatusCode::UNAUTHORIZED,
      Json(ErrorResponse {
         error_type: "https://ringiflow.example.com/errors/authentication-failed".to_string(),
         title:      "Authentication Failed".to_string(),
         status:     401,
         detail:     "メールアドレスまたはパスワードが正しくありません".to_string(),
      }),
   )
      .into_response()
}

/// 未認証レスポンス
fn unauthorized_response() -> axum::response::Response {
   (
      StatusCode::UNAUTHORIZED,
      Json(ErrorResponse {
         error_type: "https://ringiflow.example.com/errors/unauthorized".to_string(),
         title:      "Unauthorized".to_string(),
         status:     401,
         detail:     "認証が必要です".to_string(),
      }),
   )
      .into_response()
}

/// 内部エラーレスポンス
fn internal_error_response() -> axum::response::Response {
   (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(ErrorResponse {
         error_type: "https://ringiflow.example.com/errors/internal-error".to_string(),
         title:      "Internal Server Error".to_string(),
         status:     500,
         detail:     "内部エラーが発生しました".to_string(),
      }),
   )
      .into_response()
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
   use crate::client::{RoleResponse, UserResponse, VerifyResponse};

   // テスト用スタブ

   struct StubCoreApiClient {
      verify_result:   Result<VerifyResponse, CoreApiError>,
      get_user_result: Result<UserWithPermissionsResponse, CoreApiError>,
   }

   impl StubCoreApiClient {
      fn success() -> Self {
         let user = UserResponse {
            id:        Uuid::now_v7(),
            tenant_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            email:     "user@example.com".to_string(),
            name:      "Test User".to_string(),
            status:    "active".to_string(),
         };
         let roles = vec![RoleResponse {
            id:          Uuid::now_v7(),
            name:        "user".to_string(),
            permissions: vec!["workflow:read".to_string()],
         }];
         Self {
            verify_result:   Ok(VerifyResponse {
               user:  user.clone(),
               roles: roles.clone(),
            }),
            get_user_result: Ok(UserWithPermissionsResponse {
               user,
               roles: vec!["user".to_string()],
               permissions: vec!["workflow:read".to_string()],
            }),
         }
      }

      fn auth_failed() -> Self {
         Self {
            verify_result:   Err(CoreApiError::AuthenticationFailed),
            get_user_result: Err(CoreApiError::UserNotFound),
         }
      }
   }

   #[async_trait]
   impl CoreApiClient for StubCoreApiClient {
      async fn verify_credentials(
         &self,
         _tenant_id: Uuid,
         _email: &str,
         _password: &str,
      ) -> Result<VerifyResponse, CoreApiError> {
         self.verify_result.clone()
      }

      async fn get_user(
         &self,
         _user_id: Uuid,
      ) -> Result<UserWithPermissionsResponse, CoreApiError> {
         self.get_user_result.clone()
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
   }

   fn create_test_app(client: StubCoreApiClient, session_manager: StubSessionManager) -> Router {
      let state = Arc::new(AuthState {
         core_api_client: client,
         session_manager,
      });

      Router::new()
         .route(
            "/auth/login",
            post(login::<StubCoreApiClient, StubSessionManager>),
         )
         .route(
            "/auth/logout",
            post(logout::<StubCoreApiClient, StubSessionManager>),
         )
         .route("/auth/me", get(me::<StubCoreApiClient, StubSessionManager>))
         .with_state(state)
   }

   const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";

   // テストケース

   #[tokio::test]
   async fn test_login_成功時にセッションcookieが設定される() {
      // Given
      let app = create_test_app(StubCoreApiClient::success(), StubSessionManager::new());

      let body = serde_json::json!({
          "email": "user@example.com",
          "password": "password123"
      });

      let request = Request::builder()
         .method(Method::POST)
         .uri("/auth/login")
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
      let app = create_test_app(StubCoreApiClient::success(), StubSessionManager::new());

      let body = serde_json::json!({
          "email": "user@example.com",
          "password": "password123"
      });

      let request = Request::builder()
         .method(Method::POST)
         .uri("/auth/login")
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
   async fn test_login_認証失敗で401() {
      // Given
      let app = create_test_app(StubCoreApiClient::auth_failed(), StubSessionManager::new());

      let body = serde_json::json!({
          "email": "user@example.com",
          "password": "wrongpassword"
      });

      let request = Request::builder()
         .method(Method::POST)
         .uri("/auth/login")
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
      let app = create_test_app(StubCoreApiClient::success(), StubSessionManager::new());

      let request = Request::builder()
         .method(Method::POST)
         .uri("/auth/logout")
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
         StubCoreApiClient::success(),
         StubSessionManager::with_session(user_id, tenant_id),
      );

      let request = Request::builder()
         .method(Method::GET)
         .uri("/auth/me")
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
      let app = create_test_app(StubCoreApiClient::success(), StubSessionManager::new());

      let request = Request::builder()
         .method(Method::GET)
         .uri("/auth/me")
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
      let app = create_test_app(StubCoreApiClient::success(), StubSessionManager::new());

      let body = serde_json::json!({
          "email": "user@example.com",
          "password": "password123"
      });

      let request = Request::builder()
         .method(Method::POST)
         .uri("/auth/login")
         .header("content-type", "application/json")
         // X-Tenant-ID ヘッダーなし
         .body(Body::from(serde_json::to_string(&body).unwrap()))
         .unwrap();

      // When
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::BAD_REQUEST);
   }
}
