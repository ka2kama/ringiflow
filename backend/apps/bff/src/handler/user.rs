//! # ユーザー管理 API ハンドラ
//!
//! BFF のユーザー管理エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/users` - テナント内のユーザー一覧
//! - `POST /api/v1/users` - ユーザー作成
//! - `GET /api/v1/users/{display_number}` - ユーザー詳細
//! - `PATCH /api/v1/users/{display_number}` - ユーザー更新
//! - `PATCH /api/v1/users/{display_number}/status` - ステータス変更

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::audit_log::{AuditAction, AuditLog};
use ringiflow_infra::{SessionManager, repository::AuditLogRepository};
use ringiflow_shared::{ApiResponse, ErrorResponse};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    client::{
        AuthServiceClient,
        CoreServiceUserClient,
        CreateUserCoreRequest,
        UpdateUserCoreRequest,
        UpdateUserStatusCoreRequest,
    },
    error::{authenticate, log_and_convert_core_error},
};

/// ユーザー管理 API の共有状態
pub struct UserState {
    pub core_service_client:  Arc<dyn CoreServiceUserClient>,
    pub auth_service_client:  Arc<dyn AuthServiceClient>,
    pub session_manager:      Arc<dyn SessionManager>,
    pub audit_log_repository: Arc<dyn AuditLogRepository>,
}

// --- リクエスト型 ---

/// ユーザー一覧クエリパラメータ
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListUsersQuery {
    pub status: Option<String>,
}

/// ユーザー作成リクエスト
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email:   String,
    pub name:    String,
    #[schema(format = "uuid")]
    pub role_id: String,
}

/// ユーザー更新リクエスト
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub name:    Option<String>,
    #[schema(format = "uuid")]
    pub role_id: Option<String>,
}

/// ユーザーステータス変更リクエスト
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserStatusRequest {
    pub status: String,
}

// --- レスポンス型 ---

/// ユーザー一覧の要素データ
#[derive(Debug, Serialize, ToSchema)]
pub struct UserItemData {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub roles: Vec<String>,
}

impl From<crate::client::UserItemDto> for UserItemData {
    fn from(dto: crate::client::UserItemDto) -> Self {
        Self {
            id: dto.id.to_string(),
            display_id: dto.display_id,
            display_number: dto.display_number,
            name: dto.name,
            email: dto.email,
            status: dto.status,
            roles: dto.roles,
        }
    }
}

/// ユーザー作成レスポンス（初期パスワード付き）
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateUserResponseData {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub role: String,
    pub initial_password: String,
}

/// ユーザー詳細レスポンス
#[derive(Debug, Serialize, ToSchema)]
pub struct UserDetailData {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub tenant_name: String,
}

/// ユーザー簡易レスポンス（更新・ステータス変更用）
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponseData {
    pub id:     String,
    pub name:   String,
    pub email:  String,
    pub status: String,
}

// --- ヘルパー関数 ---

/// 初期パスワードを生成する（16文字、英数字 + 特殊文字）
fn generate_initial_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%&*";
    let mut rng = rand::rng();
    (0..16)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// --- ハンドラ ---

/// GET /api/v1/users
///
/// テナント内のユーザー一覧を取得する。
/// ステータスフィルタに対応（省略時は deleted 以外すべて）。
#[utoipa::path(
   get,
   path = "/api/v1/users",
   tag = "users",
   security(("session_auth" = [])),
   params(ListUsersQuery),
   responses(
      (status = 200, description = "ユーザー一覧", body = ApiResponse<Vec<UserItemData>>),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn list_users(
    State(state): State<Arc<UserState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Query(query): Query<ListUsersQuery>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_users(*session_data.tenant_id().as_uuid(), query.status.as_deref())
        .await
        .map_err(|e| log_and_convert_core_error("ユーザー一覧取得", e))?;

    let response = ApiResponse::new(
        core_response
            .data
            .into_iter()
            .map(UserItemData::from)
            .collect::<Vec<_>>(),
    );
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/users
///
/// ユーザーを作成する。
///
/// ## フロー
///
/// 1. 入力バリデーション
/// 2. 初期パスワード生成
/// 3. Core Service でユーザー作成
/// 4. Auth Service で認証情報作成
/// 5. 初期パスワード付きレスポンス返却
#[utoipa::path(
   post,
   path = "/api/v1/users",
   tag = "users",
   security(("session_auth" = [])),
   request_body = CreateUserRequest,
   responses(
      (status = 201, description = "ユーザー作成", body = ApiResponse<CreateUserResponseData>),
      (status = 400, description = "バリデーションエラー", body = ErrorResponse),
      (status = 409, description = "メールアドレス重複", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn create_user(
    State(state): State<Arc<UserState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<CreateUserRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    // 初期パスワード生成
    let initial_password = generate_initial_password();

    // role_id の UUID パース
    let role_id = uuid::Uuid::parse_str(&req.role_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::validation_error("role_id の形式が不正です")),
        )
            .into_response()
    })?;

    // Core Service でユーザー作成
    let core_request = CreateUserCoreRequest {
        tenant_id: *session_data.tenant_id().as_uuid(),
        email: req.email,
        name: req.name,
        role_id,
    };

    let core_response = state
        .core_service_client
        .create_user(&core_request)
        .await
        .map_err(|e| log_and_convert_core_error("ユーザー作成", e))?;

    let user_data = core_response.data;

    // Auth Service で認証情報作成
    if let Err(e) = state
        .auth_service_client
        .create_credentials(
            *session_data.tenant_id().as_uuid(),
            user_data.id,
            "password",
            &initial_password,
        )
        .await
    {
        tracing::error!("認証情報の作成に失敗しました（ユーザーは作成済み）: {}", e);
        return Err(crate::error::internal_error_response());
    }

    // 監査ログ記録
    let audit_log = AuditLog::new_success(
        session_data.tenant_id().clone(),
        session_data.user_id().clone(),
        session_data.name().to_string(),
        AuditAction::UserCreate,
        "user",
        user_data.id.to_string(),
        Some(serde_json::json!({
           "email": &user_data.email,
           "name": &user_data.name,
           "role": &user_data.role,
        })),
        None,
    );
    if let Err(e) = state.audit_log_repository.record(&audit_log).await {
        tracing::error!("監査ログ記録に失敗: {}", e);
    }

    let response = ApiResponse::new(CreateUserResponseData {
        id: user_data.id.to_string(),
        display_id: user_data.display_id,
        display_number: user_data.display_number,
        name: user_data.name,
        email: user_data.email,
        role: user_data.role,
        initial_password,
    });

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// GET /api/v1/users/{display_number}
///
/// 表示用連番でユーザー詳細を取得する。
#[utoipa::path(
   get,
   path = "/api/v1/users/{display_number}",
   tag = "users",
   security(("session_auth" = [])),
   params(("display_number" = i64, Path, description = "ユーザー表示番号")),
   responses(
      (status = 200, description = "ユーザー詳細", body = ApiResponse<UserDetailData>),
      (status = 404, description = "ユーザーが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn get_user_detail(
    State(state): State<Arc<UserState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(display_number): Path<i64>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .get_user_by_display_number(*session_data.tenant_id().as_uuid(), display_number)
        .await
        .map_err(|e| log_and_convert_core_error("ユーザー詳細取得", e))?;

    let data = core_response.data;
    let response = ApiResponse::new(UserDetailData {
        id: data.user.id.to_string(),
        display_id: format!("USR-{:06}", display_number),
        display_number,
        name: data.user.name,
        email: data.user.email,
        status: data.user.status,
        roles: data.roles,
        permissions: data.permissions,
        tenant_name: data.tenant_name,
    });
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// PATCH /api/v1/users/{display_number}
///
/// ユーザー情報を更新する（名前、ロール）。
#[utoipa::path(
   patch,
   path = "/api/v1/users/{display_number}",
   tag = "users",
   security(("session_auth" = [])),
   params(("display_number" = i64, Path, description = "ユーザー表示番号")),
   request_body = UpdateUserRequest,
   responses(
      (status = 200, description = "更新成功", body = ApiResponse<UserResponseData>),
      (status = 400, description = "バリデーションエラー", body = ErrorResponse),
      (status = 404, description = "ユーザーが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn update_user(
    State(state): State<Arc<UserState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(display_number): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    // display_number でユーザーを取得して UUID を解決する
    let user_data = state
        .core_service_client
        .get_user_by_display_number(*session_data.tenant_id().as_uuid(), display_number)
        .await
        .map_err(|e| log_and_convert_core_error("ユーザー取得", e))?
        .data;

    // role_id の UUID パース
    let role_id = req
        .role_id
        .map(|id| uuid::Uuid::parse_str(&id))
        .transpose()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::validation_error("role_id の形式が不正です")),
            )
                .into_response()
        })?;

    let core_request = UpdateUserCoreRequest {
        name: req.name,
        role_id,
    };

    match state
        .core_service_client
        .update_user(user_data.user.id, &core_request)
        .await
    {
        Ok(core_response) => {
            let user = core_response.data;

            // 監査ログ記録
            let audit_log = AuditLog::new_success(
                session_data.tenant_id().clone(),
                session_data.user_id().clone(),
                session_data.name().to_string(),
                AuditAction::UserUpdate,
                "user",
                user.id.to_string(),
                Some(serde_json::json!({
                   "name": &user.name,
                })),
                None,
            );
            if let Err(e) = state.audit_log_repository.record(&audit_log).await {
                tracing::error!("監査ログ記録に失敗: {}", e);
            }

            let response = ApiResponse::new(UserResponseData {
                id:     user.id.to_string(),
                name:   user.name,
                email:  user.email,
                status: user.status,
            });
            Ok((StatusCode::OK, Json(response)).into_response())
        }
        Err(e) => Err(log_and_convert_core_error("ユーザー更新", e)),
    }
}

/// PATCH /api/v1/users/{display_number}/status
///
/// ユーザーステータスを変更する。
/// セッションから自身の user_id を取得し、Core Service に渡す。
#[utoipa::path(
   patch,
   path = "/api/v1/users/{display_number}/status",
   tag = "users",
   security(("session_auth" = [])),
   params(("display_number" = i64, Path, description = "ユーザー表示番号")),
   request_body = UpdateUserStatusRequest,
   responses(
      (status = 200, description = "ステータス変更成功", body = ApiResponse<UserResponseData>),
      (status = 400, description = "バリデーションエラー", body = ErrorResponse),
      (status = 404, description = "ユーザーが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn update_user_status(
    State(state): State<Arc<UserState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(display_number): Path<i64>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    // display_number でユーザーを取得して UUID を解決する
    let user_data = state
        .core_service_client
        .get_user_by_display_number(*session_data.tenant_id().as_uuid(), display_number)
        .await
        .map_err(|e| log_and_convert_core_error("ユーザー取得", e))?
        .data;

    let core_request = UpdateUserStatusCoreRequest {
        status:       req.status,
        tenant_id:    *session_data.tenant_id().as_uuid(),
        requester_id: *session_data.user_id().as_uuid(),
    };

    match state
        .core_service_client
        .update_user_status(user_data.user.id, &core_request)
        .await
    {
        Ok(core_response) => {
            let user = core_response.data;

            // 監査ログ記録: ステータスに応じて Deactivate / Activate を使い分ける
            let action = if user.status == "inactive" {
                AuditAction::UserDeactivate
            } else {
                AuditAction::UserActivate
            };
            let audit_log = AuditLog::new_success(
                session_data.tenant_id().clone(),
                session_data.user_id().clone(),
                session_data.name().to_string(),
                action,
                "user",
                user.id.to_string(),
                Some(serde_json::json!({
                   "user_name": &user.name,
                })),
                None,
            );
            if let Err(e) = state.audit_log_repository.record(&audit_log).await {
                tracing::error!("監査ログ記録に失敗: {}", e);
            }

            let response = ApiResponse::new(UserResponseData {
                id:     user.id.to_string(),
                name:   user.name,
                email:  user.email,
                status: user.status,
            });
            Ok((StatusCode::OK, Json(response)).into_response())
        }
        Err(e) => Err(log_and_convert_core_error("ユーザーステータス変更", e)),
    }
}
