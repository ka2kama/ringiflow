//! # ロール管理 API ハンドラ
//!
//! BFF のロール管理エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/roles` - テナント内のロール一覧（ユーザー数付き）
//! - `POST /api/v1/roles` - カスタムロール作成
//! - `GET /api/v1/roles/{role_id}` - ロール詳細
//! - `PATCH /api/v1/roles/{role_id}` - カスタムロール更新
//! - `DELETE /api/v1/roles/{role_id}` - カスタムロール削除

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::audit_log::{AuditAction, AuditLog};
use ringiflow_infra::{SessionManager, repository::AuditLogRepository};
use ringiflow_shared::{ApiResponse, ErrorResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    client::{CoreServiceRoleClient, CreateRoleCoreRequest, UpdateRoleCoreRequest},
    error::{authenticate, log_and_convert_core_error},
};

/// ロール管理 API の共有状態
pub struct RoleState {
    pub core_service_client:  Arc<dyn CoreServiceRoleClient>,
    pub session_manager:      Arc<dyn SessionManager>,
    pub audit_log_repository: Arc<dyn AuditLogRepository>,
}

// --- リクエスト型 ---

/// ロール作成リクエスト
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name:        String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

/// ロール更新リクエスト
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub name:        Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

// --- レスポンス型 ---

/// ロール一覧の要素データ
#[derive(Debug, Serialize, ToSchema)]
pub struct RoleItemData {
    pub id:          String,
    pub name:        String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system:   bool,
    pub user_count:  i64,
}

/// ロール詳細データ
#[derive(Debug, Serialize, ToSchema)]
pub struct RoleDetailData {
    pub id:          String,
    pub name:        String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system:   bool,
    pub created_at:  String,
    pub updated_at:  String,
}

// --- ハンドラ ---

/// GET /api/v1/roles
///
/// テナント内のロール一覧をユーザー数付きで取得する。
/// system_admin は除外される。
#[utoipa::path(
   get,
   path = "/api/v1/roles",
   tag = "roles",
   security(("session_auth" = [])),
   responses(
      (status = 200, description = "ロール一覧", body = ApiResponse<Vec<RoleItemData>>),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn list_roles(
    State(state): State<Arc<RoleState>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_roles(*session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("ロール一覧取得", e))?;

    let items: Vec<RoleItemData> = core_response
        .data
        .into_iter()
        .map(|dto| RoleItemData {
            id:          dto.id.to_string(),
            name:        dto.name,
            description: dto.description,
            permissions: dto.permissions,
            is_system:   dto.is_system,
            user_count:  dto.user_count,
        })
        .collect();
    let response = ApiResponse::new(items);
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// GET /api/v1/roles/{role_id}
///
/// ロール詳細を取得する。
#[utoipa::path(
   get,
   path = "/api/v1/roles/{role_id}",
   tag = "roles",
   security(("session_auth" = [])),
   params(("role_id" = Uuid, Path, description = "ロールID")),
   responses(
      (status = 200, description = "ロール詳細", body = ApiResponse<RoleDetailData>),
      (status = 404, description = "ロールが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%role_id))]
pub async fn get_role(
    State(state): State<Arc<RoleState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(role_id): Path<Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .get_role(role_id, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("ロール詳細取得", e))?;

    let dto = core_response.data;
    let response = ApiResponse::new(RoleDetailData {
        id:          dto.id.to_string(),
        name:        dto.name,
        description: dto.description,
        permissions: dto.permissions,
        is_system:   dto.is_system,
        created_at:  dto.created_at,
        updated_at:  dto.updated_at,
    });
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/roles
///
/// カスタムロールを作成する。
#[utoipa::path(
   post,
   path = "/api/v1/roles",
   tag = "roles",
   security(("session_auth" = [])),
   request_body = CreateRoleRequest,
   responses(
      (status = 201, description = "ロール作成成功", body = ApiResponse<RoleDetailData>),
      (status = 400, description = "バリデーションエラー", body = ErrorResponse),
      (status = 409, description = "ロール名重複", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn create_role(
    State(state): State<Arc<RoleState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = CreateRoleCoreRequest {
        tenant_id:   *session_data.tenant_id().as_uuid(),
        name:        req.name,
        description: req.description,
        permissions: req.permissions,
    };

    match state.core_service_client.create_role(&core_request).await {
        Ok(core_response) => {
            let dto = core_response.data;

            // 監査ログ記録
            let audit_log = AuditLog::new_success(
                session_data.tenant_id().clone(),
                session_data.user_id().clone(),
                session_data.name().to_string(),
                AuditAction::RoleCreate,
                "role",
                dto.id.to_string(),
                Some(serde_json::json!({
                   "name": &dto.name,
                   "permissions": &dto.permissions,
                })),
                None,
            );
            if let Err(e) = state.audit_log_repository.record(&audit_log).await {
                tracing::error!("監査ログ記録に失敗: {}", e);
            }

            let response = ApiResponse::new(RoleDetailData {
                id:          dto.id.to_string(),
                name:        dto.name,
                description: dto.description,
                permissions: dto.permissions,
                is_system:   dto.is_system,
                created_at:  dto.created_at,
                updated_at:  dto.updated_at,
            });
            Ok((StatusCode::CREATED, Json(response)).into_response())
        }
        Err(e) => Err(log_and_convert_core_error("ロール作成", e)),
    }
}

/// PATCH /api/v1/roles/{role_id}
///
/// カスタムロールを更新する。
#[utoipa::path(
   patch,
   path = "/api/v1/roles/{role_id}",
   tag = "roles",
   security(("session_auth" = [])),
   params(("role_id" = Uuid, Path, description = "ロールID")),
   request_body = UpdateRoleRequest,
   responses(
      (status = 200, description = "ロール更新成功", body = ApiResponse<RoleDetailData>),
      (status = 400, description = "バリデーションエラー", body = ErrorResponse),
      (status = 404, description = "ロールが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%role_id))]
pub async fn update_role(
    State(state): State<Arc<RoleState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(role_id): Path<Uuid>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = UpdateRoleCoreRequest {
        name:        req.name,
        description: req.description,
        permissions: req.permissions,
    };

    match state
        .core_service_client
        .update_role(role_id, &core_request)
        .await
    {
        Ok(core_response) => {
            let dto = core_response.data;

            // 監査ログ記録
            let audit_log = AuditLog::new_success(
                session_data.tenant_id().clone(),
                session_data.user_id().clone(),
                session_data.name().to_string(),
                AuditAction::RoleUpdate,
                "role",
                dto.id.to_string(),
                Some(serde_json::json!({
                   "name": &dto.name,
                   "permissions": &dto.permissions,
                })),
                None,
            );
            if let Err(e) = state.audit_log_repository.record(&audit_log).await {
                tracing::error!("監査ログ記録に失敗: {}", e);
            }

            let response = ApiResponse::new(RoleDetailData {
                id:          dto.id.to_string(),
                name:        dto.name,
                description: dto.description,
                permissions: dto.permissions,
                is_system:   dto.is_system,
                created_at:  dto.created_at,
                updated_at:  dto.updated_at,
            });
            Ok((StatusCode::OK, Json(response)).into_response())
        }
        Err(e) => Err(log_and_convert_core_error("ロール更新", e)),
    }
}

/// DELETE /api/v1/roles/{role_id}
///
/// カスタムロールを削除する。
#[utoipa::path(
   delete,
   path = "/api/v1/roles/{role_id}",
   tag = "roles",
   security(("session_auth" = [])),
   params(("role_id" = Uuid, Path, description = "ロールID")),
   responses(
      (status = 204, description = "削除成功"),
      (status = 404, description = "ロールが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%role_id))]
pub async fn delete_role(
    State(state): State<Arc<RoleState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(role_id): Path<Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    match state.core_service_client.delete_role(role_id).await {
        Ok(()) => {
            // 監査ログ記録
            let audit_log = AuditLog::new_success(
                session_data.tenant_id().clone(),
                session_data.user_id().clone(),
                session_data.name().to_string(),
                AuditAction::RoleDelete,
                "role",
                role_id.to_string(),
                Some(serde_json::json!({
                   "role_id": role_id.to_string(),
                })),
                None,
            );
            if let Err(e) = state.audit_log_repository.record(&audit_log).await {
                tracing::error!("監査ログ記録に失敗: {}", e);
            }

            Ok(StatusCode::NO_CONTENT.into_response())
        }
        Err(e) => Err(log_and_convert_core_error("ロール削除", e)),
    }
}
