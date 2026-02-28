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
//! 詳細: [08_AuthService設計.md](../../../../docs/40_詳細設計書/08_AuthService設計.md)

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use ringiflow_domain::{
    role::{Role, RoleId},
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
            name:      user.name().as_str().to_string(),
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
            name: user.name().as_str().to_string(),
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
    pub role_id:   Uuid,
}

/// ユーザー更新リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name:    Option<String>,
    pub role_id: Option<Uuid>,
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
#[tracing::instrument(skip_all)]
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
#[tracing::instrument(skip_all)]
pub async fn get_user_by_email(
    State(state): State<Arc<UserState>>,
    Query(query): Query<GetUserByEmailQuery>,
) -> impl IntoResponse {
    // メールアドレスを検証
    let Ok(email) = Email::new(&query.email) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::validation_error(
                "メールアドレスの形式が不正です",
            )),
        )
            .into_response();
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
#[tracing::instrument(skip_all, fields(%user_id))]
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
/// - `role_id`: 割り当てるロール ID
///
/// ## レスポンス
///
/// - `201 Created`: 作成されたユーザー情報
/// - `400 Bad Request`: バリデーションエラー（メール形式不正、ロール不存在）
/// - `409 Conflict`: メールアドレスが既に使用されている
#[tracing::instrument(skip_all)]
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
        role_id: RoleId::from_uuid(req.role_id),
    };

    let (user, role) = state.usecase.create_user(input).await?;

    let response = ApiResponse::new(CreateUserResponseDto {
        id: *user.id().as_uuid(),
        display_id: DisplayId::new(display_prefix::USER, user.display_number()).to_string(),
        display_number: user.display_number().as_i64(),
        name: user.name().as_str().to_string(),
        email: user.email().as_str().to_string(),
        role: role.name().to_string(),
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
#[tracing::instrument(skip_all, fields(display_number))]
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
/// - `role_id`: 新しいロール ID（省略可）
///
/// ## レスポンス
///
/// - `200 OK`: 更新後のユーザー情報
/// - `400 Bad Request`: バリデーションエラー
/// - `404 Not Found`: ユーザーが見つからない
#[tracing::instrument(skip_all, fields(%user_id))]
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
        role_id: req.role_id.map(RoleId::from_uuid),
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
#[tracing::instrument(skip_all, fields(%user_id))]
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
mod tests;
