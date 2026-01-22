//! # ユーザーハンドラ
//!
//! Core API のユーザー関連内部 API を提供する。
//!
//! ## エンドポイント
//!
//! - `GET /internal/users/by-email` - メールアドレスでユーザーを検索
//! - `GET /internal/users/{user_id}` - ユーザー情報を取得
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/08_AuthService設計.md)

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, Query, State},
   http::StatusCode,
   response::IntoResponse,
};
use ringiflow_domain::{
   role::Role,
   tenant::TenantId,
   user::{Email, User, UserId},
};
use ringiflow_infra::repository::user_repository::UserRepository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ユーザー API の共有状態
pub struct UserState<R>
where
   R: UserRepository,
{
   pub user_repository: R,
}

// --- リクエスト/レスポンス型 ---

/// メールアドレス検索クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct GetUserByEmailQuery {
   pub email:     String,
   pub tenant_id: Uuid,
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

/// メールアドレス検索レスポンス
#[derive(Debug, Serialize)]
pub struct GetUserByEmailResponse {
   pub user: UserResponse,
}

/// ロール情報レスポンス
///
/// FIXME: `#[allow(dead_code)]` を解消する
///        （ユーザー取得 API でロール詳細を返すか、構造体ごと削除する）
#[allow(dead_code)]
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

/// GET /internal/users/by-email
///
/// メールアドレスでユーザーを検索する。
///
/// ## クエリパラメータ
///
/// - `email`: メールアドレス
/// - `tenant_id`: テナント ID
///
/// ## レスポンス
///
/// - `200 OK`: ユーザー情報
/// - `400 Bad Request`: メールアドレスの形式が不正
/// - `404 Not Found`: ユーザーが見つからない
pub async fn get_user_by_email<R>(
   State(state): State<Arc<UserState<R>>>,
   Query(query): Query<GetUserByEmailQuery>,
) -> impl IntoResponse
where
   R: UserRepository,
{
   // メールアドレスを検証
   let email = match Email::new(&query.email) {
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

   let tenant_id = TenantId::from_uuid(query.tenant_id);

   // ユーザーを検索
   match state
      .user_repository
      .find_by_email(&tenant_id, &email)
      .await
   {
      Ok(Some(user)) => {
         let response = GetUserByEmailResponse {
            user: UserResponse::from(&user),
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
         tracing::error!("ユーザー検索で内部エラー: {}", e);
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
pub async fn get_user<R>(
   State(state): State<Arc<UserState<R>>>,
   Path(user_id): Path<Uuid>,
) -> impl IntoResponse
where
   R: UserRepository,
{
   let user_id = UserId::from_uuid(user_id);

   // ユーザーをロール付きで取得
   match state.user_repository.find_with_roles(&user_id).await {
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
      routing::get,
   };
   use ringiflow_domain::{
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

   fn create_test_app(repo: StubUserRepository) -> Router {
      let state = Arc::new(UserState {
         user_repository: repo,
      });

      Router::new()
         .route(
            "/internal/users/by-email",
            get(get_user_by_email::<StubUserRepository>),
         )
         .route(
            "/internal/users/{user_id}",
            get(get_user::<StubUserRepository>),
         )
         .with_state(state)
   }

   // テストケース

   #[tokio::test]
   async fn test_get_user_by_email_ユーザーが見つかる() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let roles = vec![create_user_role()];
      let app = create_test_app(StubUserRepository::with_user(user, roles));

      let request = Request::builder()
         .method(Method::GET)
         .uri(format!(
            "/internal/users/by-email?email=user@example.com&tenant_id={}",
            tenant_id.as_uuid()
         ))
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

      assert_eq!(json["user"]["email"], "user@example.com");
      assert_eq!(json["user"]["status"], "active");
   }

   #[tokio::test]
   async fn test_get_user_by_email_ユーザーが見つからない() {
      // Given
      let app = create_test_app(StubUserRepository::empty());
      let tenant_id = TenantId::new();

      let request = Request::builder()
         .method(Method::GET)
         .uri(format!(
            "/internal/users/by-email?email=notfound@example.com&tenant_id={}",
            tenant_id.as_uuid()
         ))
         .body(Body::empty())
         .unwrap();

      // When
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::NOT_FOUND);
   }

   #[tokio::test]
   async fn test_get_user_by_email_不正なメールアドレス() {
      // Given
      let app = create_test_app(StubUserRepository::empty());
      let tenant_id = TenantId::new();

      let request = Request::builder()
         .method(Method::GET)
         .uri(format!(
            "/internal/users/by-email?email=invalid-email&tenant_id={}",
            tenant_id.as_uuid()
         ))
         .body(Body::empty())
         .unwrap();

      // When
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::BAD_REQUEST);
   }

   #[tokio::test]
   async fn test_get_user_ユーザー情報を取得できる() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let user_id = *user.id().as_uuid();
      let roles = vec![create_user_role()];
      let app = create_test_app(StubUserRepository::with_user(user, roles));

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
      let app = create_test_app(StubUserRepository::empty());

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
