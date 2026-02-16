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
    role::Role,
    tenant::TenantId,
    user::{Email, User, UserId, UserStatus},
    value_objects::{DisplayId, DisplayNumber, UserName, display_prefix},
};
use ringiflow_infra::repository::{TenantRepository, UserRepository};
use ringiflow_shared::{ApiResponse, ErrorResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::CoreError,
    usecase::user::{CreateUserInput, UpdateUserInput, UpdateUserStatusInput, UserUseCaseImpl},
};

/// ユーザー API の共有状態
pub struct UserState {
    pub user_repository:   Arc<dyn UserRepository>,
    pub tenant_repository: Arc<dyn TenantRepository>,
    pub usecase:           UserUseCaseImpl,
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

/// User と Role リストから UserWithPermissionsData を構築する
fn build_user_with_permissions(
    user: &User,
    roles: &[Role],
    tenant_name: String,
) -> UserWithPermissionsData {
    let permissions: Vec<String> = roles
        .iter()
        .flat_map(|r| r.permissions().iter().map(|p| p.to_string()))
        .collect();

    UserWithPermissionsData {
        user: UserResponse::from(user),
        tenant_name,
        roles: roles.iter().map(|r| r.name().to_string()).collect(),
        permissions,
    }
}

/// ユーザー一覧の要素 DTO
#[derive(Debug, Serialize)]
pub struct UserItemDto {
    pub id: Uuid,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub roles: Vec<String>,
}

impl UserItemDto {
    fn from_user(user: &User) -> Self {
        Self {
            id: *user.id().as_uuid(),
            display_id: DisplayId::new(display_prefix::USER, user.display_number()).to_string(),
            display_number: user.display_number().as_i64(),
            name: user.name().to_string(),
            email: user.email().as_str().to_string(),
            status: user.status().to_string(),
            roles: Vec::new(),
        }
    }

    fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }
}

/// テナント ID クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
    pub status:    Option<String>,
}

/// ユーザー作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub tenant_id: Uuid,
    pub email:     String,
    pub name:      String,
    pub role_name: String,
}

/// ユーザー更新リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name:      Option<String>,
    pub role_name: Option<String>,
}

/// ユーザーステータス変更リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateUserStatusRequest {
    pub status:       String,
    pub tenant_id:    Uuid,
    pub requester_id: Uuid,
}

/// ユーザー作成レスポンス
#[derive(Debug, Serialize)]
pub struct CreateUserResponseDto {
    pub id: Uuid,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub role: String,
}

// --- ハンドラ ---

/// GET /internal/users
///
/// テナント内のユーザー一覧を取得する。
///
/// ## クエリパラメータ
///
/// - `tenant_id`: テナント ID
/// - `status`: ステータスフィルタ（省略時は deleted 以外すべて）
///
/// ## レスポンス
///
/// - `200 OK`: ユーザー一覧（ロール情報付き）
pub async fn list_users(
    State(state): State<Arc<UserState>>,
    Query(query): Query<TenantQuery>,
) -> impl IntoResponse {
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    // ステータスフィルタのパース
    let status_filter = match query.status.as_deref() {
        Some(s) => match s.parse::<UserStatus>() {
            Ok(status) => Some(status),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::bad_request("不正なステータス値です")),
                )
                    .into_response();
            }
        },
        None => None,
    };

    let users = match state
        .user_repository
        .find_all_by_tenant(&tenant_id, status_filter)
        .await
    {
        Ok(users) => users,
        Err(e) => {
            tracing::error!("ユーザー一覧取得で内部エラー: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error()),
            )
                .into_response();
        }
    };

    // ロール情報を一括取得
    let user_ids: Vec<UserId> = users.iter().map(|u| u.id().clone()).collect();
    let roles_map = match state
        .user_repository
        .find_roles_for_users(&user_ids, &tenant_id)
        .await
    {
        Ok(map) => map,
        Err(e) => {
            tracing::error!("ロール情報取得で内部エラー: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error()),
            )
                .into_response();
        }
    };

    let items: Vec<UserItemDto> = users
        .iter()
        .map(|user| {
            let roles = roles_map.get(user.id()).cloned().unwrap_or_default();
            UserItemDto::from_user(user).with_roles(roles)
        })
        .collect();

    let response = ApiResponse::new(items);
    (StatusCode::OK, Json(response)).into_response()
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
pub async fn get_user_by_email(
    State(state): State<Arc<UserState>>,
    Query(query): Query<GetUserByEmailQuery>,
) -> impl IntoResponse {
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
pub async fn get_user(
    State(state): State<Arc<UserState>>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
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

    let response = ApiResponse::new(build_user_with_permissions(&user, &roles, tenant_name));
    (StatusCode::OK, Json(response)).into_response()
}

/// POST /internal/users
///
/// ユーザーを作成する。
///
/// ## リクエストボディ
///
/// - `tenant_id`: テナント ID
/// - `email`: メールアドレス
/// - `name`: ユーザー名
/// - `role_name`: 割り当てるロール名
///
/// ## レスポンス
///
/// - `201 Created`: 作成されたユーザー情報
/// - `400 Bad Request`: バリデーションエラー（メール形式不正、ロール不存在）
/// - `409 Conflict`: メールアドレスが既に使用されている
pub async fn create_user(
    State(state): State<Arc<UserState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let email = Email::new(&req.email).map_err(|e| CoreError::BadRequest(e.to_string()))?;
    let name = UserName::new(&req.name).map_err(|e| CoreError::BadRequest(e.to_string()))?;

    let input = CreateUserInput {
        tenant_id: TenantId::from_uuid(req.tenant_id),
        email,
        name,
        role_name: req.role_name.clone(),
    };

    let user = state.usecase.create_user(input).await?;

    let response = ApiResponse::new(CreateUserResponseDto {
        id: *user.id().as_uuid(),
        display_id: DisplayId::new(display_prefix::USER, user.display_number()).to_string(),
        display_number: user.display_number().as_i64(),
        name: user.name().to_string(),
        email: user.email().as_str().to_string(),
        role: req.role_name,
    });

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /internal/users/by-display-number/{display_number}
///
/// 表示用連番でユーザーを取得する。
///
/// ## パスパラメータ
///
/// - `display_number`: 表示用連番
///
/// ## クエリパラメータ
///
/// - `tenant_id`: テナント ID
///
/// ## レスポンス
///
/// - `200 OK`: ユーザー詳細（ロール・権限付き）
/// - `404 Not Found`: ユーザーが見つからない
pub async fn get_user_by_display_number(
    State(state): State<Arc<UserState>>,
    Path(display_number): Path<i64>,
    Query(query): Query<TenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let tenant_id = TenantId::from_uuid(query.tenant_id);
    let display_number =
        DisplayNumber::new(display_number).map_err(|e| CoreError::BadRequest(e.to_string()))?;

    let user = state
        .user_repository
        .find_by_display_number(&tenant_id, display_number)
        .await?
        .ok_or_else(|| CoreError::NotFound("ユーザーが見つかりません".to_string()))?;

    // ロール・権限情報を取得
    let (_, roles) = state
        .user_repository
        .find_with_roles(user.id())
        .await?
        .ok_or_else(|| CoreError::NotFound("ユーザーが見つかりません".to_string()))?;

    // テナント名を取得
    let tenant_name = state
        .tenant_repository
        .find_by_id(user.tenant_id())
        .await?
        .map(|t| t.name().to_string())
        .unwrap_or_default();

    let response = ApiResponse::new(build_user_with_permissions(&user, &roles, tenant_name));

    Ok((StatusCode::OK, Json(response)))
}

/// PATCH /internal/users/{user_id}
///
/// ユーザー情報を更新する（名前、ロール）。
///
/// ## パスパラメータ
///
/// - `user_id`: ユーザー ID
///
/// ## リクエストボディ
///
/// - `name`: 新しいユーザー名（省略可）
/// - `role_name`: 新しいロール名（省略可）
///
/// ## レスポンス
///
/// - `200 OK`: 更新後のユーザー情報
/// - `400 Bad Request`: バリデーションエラー
/// - `404 Not Found`: ユーザーが見つからない
pub async fn update_user(
    State(state): State<Arc<UserState>>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let name = req
        .name
        .map(|n| UserName::new(&n))
        .transpose()
        .map_err(|e| CoreError::BadRequest(e.to_string()))?;

    let input = UpdateUserInput {
        user_id: UserId::from_uuid(user_id),
        name,
        role_name: req.role_name,
    };

    let user = state.usecase.update_user(input).await?;

    let response = ApiResponse::new(UserResponse::from(&user));
    Ok((StatusCode::OK, Json(response)))
}

/// PATCH /internal/users/{user_id}/status
///
/// ユーザーステータスを変更する。
///
/// ## パスパラメータ
///
/// - `user_id`: ユーザー ID
///
/// ## リクエストボディ
///
/// - `status`: 新しいステータス（"active", "inactive"）
/// - `tenant_id`: テナント ID
/// - `requester_id`: リクエスト元のユーザー ID
///
/// ## レスポンス
///
/// - `200 OK`: 更新後のユーザー情報
/// - `400 Bad Request`: 自己無効化、最後の管理者無効化、不正なステータス
/// - `404 Not Found`: ユーザーが見つからない
pub async fn update_user_status(
    State(state): State<Arc<UserState>>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let status: UserStatus = req
        .status
        .parse()
        .map_err(|_| CoreError::BadRequest("不正なステータス値です".to_string()))?;

    let input = UpdateUserStatusInput {
        user_id: UserId::from_uuid(user_id),
        tenant_id: TenantId::from_uuid(req.tenant_id),
        status,
        requester_id: UserId::from_uuid(req.requester_id),
    };

    let user = state.usecase.update_user_status(input).await?;

    let response = ApiResponse::new(UserResponse::from(&user));
    Ok((StatusCode::OK, Json(response)))
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
        clock::Clock,
        role::{Permission, Role, RoleId},
        tenant::{Tenant, TenantId, TenantName},
        user::{Email, User, UserId, UserStatus},
        value_objects::{DisplayIdEntityType, DisplayNumber, UserName},
    };
    use ringiflow_infra::{InfraError, repository::DisplayIdCounterRepository};
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

        async fn insert(&self, _user: &User) -> Result<(), InfraError> {
            todo!()
        }

        async fn update(&self, _user: &User) -> Result<(), InfraError> {
            todo!()
        }

        async fn update_status(&self, _user: &User) -> Result<(), InfraError> {
            todo!()
        }

        async fn find_by_display_number(
            &self,
            _tenant_id: &TenantId,
            _display_number: DisplayNumber,
        ) -> Result<Option<User>, InfraError> {
            todo!()
        }

        async fn find_all_by_tenant(
            &self,
            _tenant_id: &TenantId,
            _status_filter: Option<UserStatus>,
        ) -> Result<Vec<User>, InfraError> {
            todo!()
        }

        async fn insert_user_role(
            &self,
            _user_id: &UserId,
            _role_id: &ringiflow_domain::role::RoleId,
            _tenant_id: &TenantId,
        ) -> Result<(), InfraError> {
            todo!()
        }

        async fn replace_user_roles(
            &self,
            _user_id: &UserId,
            _role_id: &ringiflow_domain::role::RoleId,
            _tenant_id: &TenantId,
        ) -> Result<(), InfraError> {
            todo!()
        }

        async fn find_role_by_name(
            &self,
            _name: &str,
        ) -> Result<Option<ringiflow_domain::role::Role>, InfraError> {
            todo!()
        }

        async fn count_active_users_with_role(
            &self,
            _tenant_id: &TenantId,
            _role_name: &str,
            _excluding_user_id: Option<&UserId>,
        ) -> Result<i64, InfraError> {
            todo!()
        }

        async fn find_roles_for_users(
            &self,
            _user_ids: &[UserId],
            _tenant_id: &TenantId,
        ) -> Result<std::collections::HashMap<UserId, Vec<String>>, InfraError> {
            todo!()
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

    struct StubDisplayIdCounterRepository;

    #[async_trait]
    impl DisplayIdCounterRepository for StubDisplayIdCounterRepository {
        async fn next_display_number(
            &self,
            _tenant_id: &TenantId,
            _entity_type: DisplayIdEntityType,
        ) -> Result<DisplayNumber, InfraError> {
            todo!()
        }
    }

    struct StubClock;

    impl Clock for StubClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
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
        let user_repo_arc = Arc::new(user_repo) as Arc<dyn UserRepository>;
        let usecase = crate::usecase::UserUseCaseImpl::new(
            user_repo_arc.clone(),
            Arc::new(StubDisplayIdCounterRepository) as Arc<dyn DisplayIdCounterRepository>,
            Arc::new(StubClock) as Arc<dyn Clock>,
        );
        let state = Arc::new(UserState {
            user_repository: user_repo_arc,
            tenant_repository: Arc::new(tenant_repo) as Arc<dyn TenantRepository>,
            usecase,
        });

        Router::new()
            .route("/internal/users/by-email", get(get_user_by_email))
            .route("/internal/users/{user_id}", get(get_user))
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
        let sut = create_test_app(
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
        let response = sut.oneshot(request).await.unwrap();

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
        let sut = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());
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
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_user_by_email_不正なメールアドレス() {
        // Given
        let sut = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());
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
        let response = sut.oneshot(request).await.unwrap();

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
        let sut = create_test_app(
            StubUserRepository::with_user(user, roles),
            StubTenantRepository::with_tenant(tenant),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("/internal/users/{}", user_id))
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

        assert_eq!(json["data"]["tenant_name"], "Test Tenant");
    }

    #[tokio::test]
    async fn test_get_user_存在しないユーザーで404() {
        // Given
        let sut = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());

        let user_id = Uuid::now_v7();
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("/internal/users/{}", user_id))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

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
        let sut = create_test_app(
            StubUserRepository::with_user(user, roles),
            StubTenantRepository::empty(),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("/internal/users/{}", user_id))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // --- build_user_with_permissions テスト ---

    #[test]
    fn build_user_with_permissions_ロールありで権限が集約される() {
        let tenant_id = TenantId::new();
        let user = create_active_user(&tenant_id);
        let roles = vec![create_user_role()];

        let result = build_user_with_permissions(&user, &roles, "Test Tenant".to_string());

        assert_eq!(result.tenant_name, "Test Tenant");
        assert_eq!(result.roles, vec!["user"]);
        assert_eq!(result.permissions.len(), 2);
        assert!(result.permissions.contains(&"workflow:read".to_string()));
        assert!(result.permissions.contains(&"task:read".to_string()));
    }

    #[test]
    fn build_user_with_permissions_ロール空で空リスト() {
        let tenant_id = TenantId::new();
        let user = create_active_user(&tenant_id);

        let result = build_user_with_permissions(&user, &[], "Test Tenant".to_string());

        assert!(result.roles.is_empty());
        assert!(result.permissions.is_empty());
    }
}
