//! # ユーザーハンドラ
//!
//! Core API のユーザー関連内部 API を提供する。
//!
//! ## エンドポイント
//!
//! - `GET /internal/users` - テナント内のアクティブユーザー一覧
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
   tenant::TenantId,
   user::{Email, User, UserId},
   value_objects::{DisplayId, display_prefix},
};
use ringiflow_infra::repository::{
   tenant_repository::TenantRepository,
   user_repository::UserRepository,
};
use ringiflow_shared::{ApiResponse, ErrorResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ユーザー API の共有状態
pub struct UserState<R, T>
where
   R: UserRepository,
   T: TenantRepository,
{
   pub user_repository:   R,
   pub tenant_repository: T,
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
         status:    user.status().to_string(),
      }
   }
}

/// ユーザー詳細データ（権限付き）
#[derive(Debug, Serialize)]
pub struct UserWithPermissionsData {
   pub user:        UserResponse,
   pub tenant_name: String,
   pub roles:       Vec<String>,
   pub permissions: Vec<String>,
}

/// ユーザー一覧の要素 DTO
#[derive(Debug, Serialize)]
pub struct UserItemDto {
   pub id: Uuid,
   pub display_id: String,
   pub display_number: i64,
   pub name: String,
   pub email: String,
}

impl UserItemDto {
   fn from_user(user: &User) -> Self {
      Self {
         id: *user.id().as_uuid(),
         display_id: DisplayId::new(display_prefix::USER, user.display_number()).to_string(),
         display_number: user.display_number().as_i64(),
         name: user.name().to_string(),
         email: user.email().as_str().to_string(),
      }
   }
}

/// テナント ID クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct TenantQuery {
   pub tenant_id: Uuid,
}

// --- ハンドラ ---

/// GET /internal/users
///
/// テナント内のアクティブユーザー一覧を取得する。
///
/// ## クエリパラメータ
///
/// - `tenant_id`: テナント ID
///
/// ## レスポンス
///
/// - `200 OK`: ユーザー一覧
pub async fn list_users<R, T>(
   State(state): State<Arc<UserState<R, T>>>,
   Query(query): Query<TenantQuery>,
) -> impl IntoResponse
where
   R: UserRepository,
   T: TenantRepository,
{
   let tenant_id = TenantId::from_uuid(query.tenant_id);

   match state
      .user_repository
      .find_all_active_by_tenant(&tenant_id)
      .await
   {
      Ok(users) => {
         let response =
            ApiResponse::new(users.iter().map(UserItemDto::from_user).collect::<Vec<_>>());
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(e) => {
         tracing::error!("ユーザー一覧取得で内部エラー: {}", e);
         (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error()),
         )
            .into_response()
      }
   }
}

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
pub async fn get_user_by_email<R, T>(
   State(state): State<Arc<UserState<R, T>>>,
   Query(query): Query<GetUserByEmailQuery>,
) -> impl IntoResponse
where
   R: UserRepository,
   T: TenantRepository,
{
   // メールアドレスを検証
   let email = match Email::new(&query.email) {
      Ok(e) => e,
      Err(_) => {
         return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::validation_error(
               "メールアドレスの形式が不正です",
            )),
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
         let response = ApiResponse::new(UserResponse::from(&user));
         (StatusCode::OK, Json(response)).into_response()
      }
      Ok(None) => (
         StatusCode::NOT_FOUND,
         Json(ErrorResponse::not_found("ユーザーが見つかりません")),
      )
         .into_response(),
      Err(e) => {
         tracing::error!("ユーザー検索で内部エラー: {}", e);
         (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error()),
         )
            .into_response()
      }
   }
}

/// GET /internal/users/{user_id}
///
/// ユーザー情報をロール・権限付きで取得する。
///
/// テナント名も含めたレスポンスを返す。
/// 認証フローで BFF が呼び出し、フロントエンドに必要な情報を一括取得する。
pub async fn get_user<R, T>(
   State(state): State<Arc<UserState<R, T>>>,
   Path(user_id): Path<Uuid>,
) -> impl IntoResponse
where
   R: UserRepository,
   T: TenantRepository,
{
   let user_id = UserId::from_uuid(user_id);

   // ユーザーをロール付きで取得
   let (user, roles) = match state.user_repository.find_with_roles(&user_id).await {
      Ok(Some(result)) => result,
      Ok(None) => {
         return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("ユーザーが見つかりません")),
         )
            .into_response();
      }
      Err(e) => {
         tracing::error!("ユーザー取得で内部エラー: {}", e);
         return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error()),
         )
            .into_response();
      }
   };

   // テナント名を取得
   let tenant_name = match state.tenant_repository.find_by_id(user.tenant_id()).await {
      Ok(Some(tenant)) => tenant.name().to_string(),
      Ok(None) => {
         tracing::error!(
            "テナントが見つかりません: user_id={}, tenant_id={}",
            user.id(),
            user.tenant_id()
         );
         return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error()),
         )
            .into_response();
      }
      Err(e) => {
         tracing::error!("テナント取得で内部エラー: {}", e);
         return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error()),
         )
            .into_response();
      }
   };

   // 権限を集約
   let permissions: Vec<String> = roles
      .iter()
      .flat_map(|r| r.permissions().iter().map(|p| p.to_string()))
      .collect();

   let response = ApiResponse::new(UserWithPermissionsData {
      user: UserResponse::from(&user),
      tenant_name,
      roles: roles.iter().map(|r| r.name().to_string()).collect(),
      permissions,
   });
   (StatusCode::OK, Json(response)).into_response()
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
      role::{Permission, Role, RoleId},
      tenant::{Tenant, TenantId, TenantName},
      user::{Email, User, UserId, UserStatus},
      value_objects::{DisplayNumber, UserName},
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

      async fn find_by_ids(&self, _ids: &[UserId]) -> Result<Vec<User>, InfraError> {
         Ok(self.user.clone().into_iter().collect())
      }

      async fn find_all_active_by_tenant(
         &self,
         _tenant_id: &TenantId,
      ) -> Result<Vec<User>, InfraError> {
         Ok(self.user.clone().into_iter().collect())
      }

      async fn update_last_login(&self, _id: &UserId) -> Result<(), InfraError> {
         Ok(())
      }
   }

   struct StubTenantRepository {
      tenant: Option<Tenant>,
   }

   impl StubTenantRepository {
      fn with_tenant(tenant: Tenant) -> Self {
         Self {
            tenant: Some(tenant),
         }
      }

      fn empty() -> Self {
         Self { tenant: None }
      }
   }

   #[async_trait]
   impl TenantRepository for StubTenantRepository {
      async fn find_by_id(&self, _id: &TenantId) -> Result<Option<Tenant>, InfraError> {
         Ok(self.tenant.clone())
      }
   }

   // テストデータ生成

   fn create_active_user(tenant_id: &TenantId) -> User {
      User::from_db(
         UserId::new(),
         tenant_id.clone(),
         DisplayNumber::new(1).unwrap(),
         Email::new("user@example.com").unwrap(),
         UserName::new("Test User").unwrap(),
         UserStatus::Active,
         None,
         chrono::Utc::now(),
         chrono::Utc::now(),
      )
   }

   fn create_user_role() -> Role {
      Role::new_system(
         RoleId::new(),
         "user".to_string(),
         Some("一般ユーザー".to_string()),
         vec![
            Permission::new("workflow:read"),
            Permission::new("task:read"),
         ],
         chrono::Utc::now(),
      )
   }

   fn create_tenant(tenant_id: &TenantId) -> Tenant {
      Tenant::from_db(tenant_id.clone(), TenantName::new("Test Tenant").unwrap())
   }

   fn create_test_app(user_repo: StubUserRepository, tenant_repo: StubTenantRepository) -> Router {
      let state = Arc::new(UserState {
         user_repository:   user_repo,
         tenant_repository: tenant_repo,
      });

      Router::new()
         .route(
            "/internal/users/by-email",
            get(get_user_by_email::<StubUserRepository, StubTenantRepository>),
         )
         .route(
            "/internal/users/{user_id}",
            get(get_user::<StubUserRepository, StubTenantRepository>),
         )
         .with_state(state)
   }

   // テストケース

   #[tokio::test]
   async fn test_get_user_by_email_ユーザーが見つかる() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let tenant = create_tenant(&tenant_id);
      let roles = vec![create_user_role()];
      let app = create_test_app(
         StubUserRepository::with_user(user, roles),
         StubTenantRepository::with_tenant(tenant),
      );

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

      assert_eq!(json["data"]["email"], "user@example.com");
      assert_eq!(json["data"]["status"], "active");
   }

   #[tokio::test]
   async fn test_get_user_by_email_ユーザーが見つからない() {
      // Given
      let app = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());
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
      let app = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());
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
      let tenant = create_tenant(&tenant_id);
      let roles = vec![create_user_role()];
      let app = create_test_app(
         StubUserRepository::with_user(user, roles),
         StubTenantRepository::with_tenant(tenant),
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

      let body = axum::body::to_bytes(response.into_body(), usize::MAX)
         .await
         .unwrap();
      let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

      assert_eq!(json["data"]["tenant_name"], "Test Tenant");
   }

   #[tokio::test]
   async fn test_get_user_存在しないユーザーで404() {
      // Given
      let app = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());

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

   #[tokio::test]
   async fn test_get_user_テナントが見つからない場合に500() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let user_id = *user.id().as_uuid();
      let roles = vec![create_user_role()];
      // テナントリポジトリは空（テナントが見つからないケース）
      let app = create_test_app(
         StubUserRepository::with_user(user, roles),
         StubTenantRepository::empty(),
      );

      let request = Request::builder()
         .method(Method::GET)
         .uri(format!("/internal/users/{}", user_id))
         .body(Body::empty())
         .unwrap();

      // When
      let response = app.oneshot(request).await.unwrap();

      // Then
      assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
   }
}
