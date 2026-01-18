//! # 認証ハンドラ
//!
//! Core API の内部認証 API を提供する。
//!
//! ## エンドポイント
//!
//! - `POST /internal/auth/verify` - 認証情報を検証
//! - `GET /internal/users/{user_id}` - ユーザー情報を取得
//!
//! 詳細: [認証機能設計](../../../../docs/03_詳細設計書/07_認証機能設計.md)

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, State},
   http::StatusCode,
   response::IntoResponse,
};
use ringiflow_domain::{
   password::PlainPassword,
   role::Role,
   tenant::TenantId,
   user::{Email, User, UserId},
};
use ringiflow_infra::{PasswordChecker, repository::user_repository::UserRepository};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::usecase::{AuthError, AuthUseCase};

/// 認証 API の共有状態
pub struct AuthState<R, P>
where
   R: UserRepository,
   P: PasswordChecker,
{
   pub usecase: AuthUseCase<R, P>,
}

// --- リクエスト/レスポンス型 ---

/// 認証検証リクエスト
#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
   pub tenant_id: Uuid,
   pub email:     String,
   pub password:  String,
}

/// 認証検証レスポンス
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
   pub user:  UserResponse,
   pub roles: Vec<RoleResponse>,
}

/// ユーザー情報レスポンス
#[derive(Debug, Serialize)]
pub struct UserResponse {
   pub id:        Uuid,
   pub tenant_id: Uuid,
   pub email:     String,
   pub name:      String,
   pub status:    String,
}

impl From<&User> for UserResponse {
   fn from(user: &User) -> Self {
      Self {
         id:        *user.id().as_uuid(),
         tenant_id: *user.tenant_id().as_uuid(),
         email:     user.email().as_str().to_string(),
         name:      user.name().to_string(),
         status:    user.status().as_str().to_string(),
      }
   }
}

/// ロール情報レスポンス
#[derive(Debug, Serialize)]
pub struct RoleResponse {
   pub id:          Uuid,
   pub name:        String,
   pub permissions: Vec<String>,
}

impl From<&Role> for RoleResponse {
   fn from(role: &Role) -> Self {
      Self {
         id:          *role.id().as_uuid(),
         name:        role.name().to_string(),
         permissions: role
            .permissions()
            .iter()
            .map(|p| p.as_str().to_string())
            .collect(),
      }
   }
}

/// ユーザー詳細レスポンス（権限付き）
#[derive(Debug, Serialize)]
pub struct UserWithPermissionsResponse {
   pub user:        UserResponse,
   pub roles:       Vec<String>,
   pub permissions: Vec<String>,
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

/// POST /internal/auth/verify
///
/// 認証情報を検証し、ユーザー情報を返す。
pub async fn verify<R, P>(
   State(state): State<Arc<AuthState<R, P>>>,
   Json(req): Json<VerifyRequest>,
) -> impl IntoResponse
where
   R: UserRepository,
   P: PasswordChecker,
{
   // リクエストをドメイン型に変換
   let tenant_id = TenantId::from_uuid(req.tenant_id);
   let email = match Email::new(&req.email) {
      Ok(e) => e,
      Err(_) => {
         return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
               error_type: "https://ringiflow.example.com/errors/validation-error".to_string(),
               title:      "Validation Error".to_string(),
               status:     400,
               detail:     "メールアドレスの形式が不正です".to_string(),
            }),
         )
            .into_response();
      }
   };
   let password = PlainPassword::new(&req.password);

   // 認証を実行
   match state
      .usecase
      .verify_credentials(&tenant_id, &email, &password)
      .await
   {
      Ok((user, roles)) => {
         let response = VerifyResponse {
            user:  UserResponse::from(&user),
            roles: roles.iter().map(RoleResponse::from).collect(),
         };
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(AuthError::AuthenticationFailed) => (
         StatusCode::UNAUTHORIZED,
         Json(ErrorResponse {
            error_type: "https://ringiflow.example.com/errors/authentication-failed".to_string(),
            title:      "Authentication Failed".to_string(),
            status:     401,
            detail:     "メールアドレスまたはパスワードが正しくありません".to_string(),
         }),
      )
         .into_response(),
      Err(AuthError::Internal(e)) => {
         tracing::error!("認証処理で内部エラー: {}", e);
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
   }
}

/// GET /internal/users/{user_id}
///
/// ユーザー情報をロール・権限付きで取得する。
pub async fn get_user<R, P>(
   State(state): State<Arc<AuthState<R, P>>>,
   Path(user_id): Path<Uuid>,
) -> impl IntoResponse
where
   R: UserRepository,
   P: PasswordChecker,
{
   let user_id = UserId::from_uuid(user_id);

   // ユーザーをロール付きで取得
   match state
      .usecase
      .user_repository()
      .find_with_roles(&user_id)
      .await
   {
      Ok(Some((user, roles))) => {
         // 権限を集約
         let permissions: Vec<String> = roles
            .iter()
            .flat_map(|r| r.permissions().iter().map(|p| p.as_str().to_string()))
            .collect();

         let response = UserWithPermissionsResponse {
            user: UserResponse::from(&user),
            roles: roles.iter().map(|r| r.name().to_string()).collect(),
            permissions,
         };
         (StatusCode::OK, Json(response)).into_response()
      }
      Ok(None) => (
         StatusCode::NOT_FOUND,
         Json(ErrorResponse {
            error_type: "https://ringiflow.example.com/errors/not-found".to_string(),
            title:      "Not Found".to_string(),
            status:     404,
            detail:     "ユーザーが見つかりません".to_string(),
         }),
      )
         .into_response(),
      Err(e) => {
         tracing::error!("ユーザー取得で内部エラー: {}", e);
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
   }
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
   use ringiflow_domain::{
      password::{PasswordHash, PasswordVerifyResult},
      role::{Permission, Role},
      tenant::TenantId,
      user::{Email, User, UserId, UserStatus},
      value_objects::UserName,
   };
   use ringiflow_infra::InfraError;
   use tower::ServiceExt;

   use super::*;

   // テスト用のスタブ実装

   struct StubUserRepository {
      user: Option<User>,
      user_with_roles: Option<(User, Vec<Role>)>,
   }

   impl StubUserRepository {
      fn with_user(user: User, roles: Vec<Role>) -> Self {
         Self {
            user: Some(user.clone()),
            user_with_roles: Some((user, roles)),
         }
      }

      fn empty() -> Self {
         Self {
            user: None,
            user_with_roles: None,
         }
      }
   }

   #[async_trait]
   impl UserRepository for StubUserRepository {
      async fn find_by_email(
         &self,
         _tenant_id: &TenantId,
         _email: &Email,
      ) -> Result<Option<User>, InfraError> {
         Ok(self.user.clone())
      }

      async fn find_by_id(&self, _id: &UserId) -> Result<Option<User>, InfraError> {
         Ok(self.user.clone())
      }

      async fn find_with_roles(
         &self,
         _id: &UserId,
      ) -> Result<Option<(User, Vec<Role>)>, InfraError> {
         Ok(self.user_with_roles.clone())
      }

      async fn update_last_login(&self, _id: &UserId) -> Result<(), InfraError> {
         Ok(())
      }
   }

   struct StubPasswordChecker {
      result: PasswordVerifyResult,
   }

   impl StubPasswordChecker {
      fn matching() -> Self {
         Self {
            result: PasswordVerifyResult::Match,
         }
      }

      fn mismatching() -> Self {
         Self {
            result: PasswordVerifyResult::Mismatch,
         }
      }
   }

   impl PasswordChecker for StubPasswordChecker {
      fn verify(
         &self,
         _password: &PlainPassword,
         _hash: &PasswordHash,
      ) -> Result<PasswordVerifyResult, InfraError> {
         Ok(self.result)
      }
   }

   // テストデータ生成

   fn create_active_user(tenant_id: &TenantId) -> User {
      User::from_db(
         UserId::new(),
         tenant_id.clone(),
         Email::new("user@example.com").unwrap(),
         UserName::new("Test User").unwrap(),
         Some("$argon2id$v=19$m=65536,t=1,p=1$...".to_string()),
         UserStatus::Active,
         None,
         chrono::Utc::now(),
         chrono::Utc::now(),
      )
   }

   fn create_user_role() -> Role {
      Role::new_system(
         "user".to_string(),
         Some("一般ユーザー".to_string()),
         vec![
            Permission::new("workflow:read"),
            Permission::new("task:read"),
         ],
      )
   }

   fn create_test_app(repo: StubUserRepository, checker: StubPasswordChecker) -> Router {
      let usecase = AuthUseCase::new(repo, checker);
      let state = Arc::new(AuthState { usecase });

      Router::new()
         .route(
            "/internal/auth/verify",
            post(verify::<StubUserRepository, StubPasswordChecker>),
         )
         .route(
            "/internal/users/{user_id}",
            get(get_user::<StubUserRepository, StubPasswordChecker>),
         )
         .with_state(state)
   }

   // テストケース

   #[tokio::test]
   async fn test_verify_正しい認証情報で認証できる() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let roles = vec![create_user_role()];
      let app = create_test_app(
         StubUserRepository::with_user(user, roles),
         StubPasswordChecker::matching(),
      );

      let request_body = serde_json::json!({
          "tenant_id": tenant_id.as_uuid(),
          "email": "user@example.com",
          "password": "password123"
      });

      let request = Request::builder()
         .method(Method::POST)
         .uri("/internal/auth/verify")
         .header("content-type", "application/json")
         .body(Body::from(serde_json::to_string(&request_body).unwrap()))
         .unwrap();

      // When
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::OK);
   }

   #[tokio::test]
   async fn test_verify_不正なパスワードで401() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let roles = vec![create_user_role()];
      let app = create_test_app(
         StubUserRepository::with_user(user, roles),
         StubPasswordChecker::mismatching(),
      );

      let request_body = serde_json::json!({
          "tenant_id": tenant_id.as_uuid(),
          "email": "user@example.com",
          "password": "wrongpassword"
      });

      let request = Request::builder()
         .method(Method::POST)
         .uri("/internal/auth/verify")
         .header("content-type", "application/json")
         .body(Body::from(serde_json::to_string(&request_body).unwrap()))
         .unwrap();

      // When
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
   }

   #[tokio::test]
   async fn test_get_user_ユーザー情報を取得できる() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let user_id = *user.id().as_uuid();
      let roles = vec![create_user_role()];
      let app = create_test_app(
         StubUserRepository::with_user(user, roles),
         StubPasswordChecker::matching(),
      );

      let request = Request::builder()
         .method(Method::GET)
         .uri(format!("/internal/users/{}", user_id))
         .body(Body::empty())
         .unwrap();

      // When
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::OK);
   }

   #[tokio::test]
   async fn test_get_user_存在しないユーザーで404() {
      // Given
      let app = create_test_app(StubUserRepository::empty(), StubPasswordChecker::matching());

      let user_id = Uuid::now_v7();
      let request = Request::builder()
         .method(Method::GET)
         .uri(format!("/internal/users/{}", user_id))
         .body(Body::empty())
         .unwrap();

      // When
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::NOT_FOUND);
   }
}
