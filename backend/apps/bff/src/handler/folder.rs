//! # フォルダ管理 API ハンドラ
//!
//! BFF のフォルダ管理エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/folders` - テナント内のフォルダ一覧（path 順）
//! - `POST /api/v1/folders` - フォルダ作成
//! - `PUT /api/v1/folders/{folder_id}` - フォルダ更新（名前変更・移動）
//! - `DELETE /api/v1/folders/{folder_id}` - フォルダ削除

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_infra::SessionManager;
use ringiflow_shared::{ApiResponse, ErrorResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    client::{CoreServiceFolderClient, CreateFolderCoreRequest, UpdateFolderCoreRequest},
    error::{authenticate, log_and_convert_core_error},
};

/// フォルダ管理 API の共有状態
pub struct FolderState {
    pub core_service_client: Arc<dyn CoreServiceFolderClient>,
    pub session_manager:     Arc<dyn SessionManager>,
}

// --- リクエスト型 ---

/// フォルダ作成リクエスト
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFolderRequest {
    pub name:      String,
    pub parent_id: Option<Uuid>,
}

/// フォルダ更新リクエスト
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFolderRequest {
    pub name:      Option<String>,
    pub parent_id: Option<Option<Uuid>>,
}

// --- レスポンス型 ---

/// フォルダデータ
#[derive(Debug, Serialize, ToSchema)]
pub struct FolderData {
    pub id:         String,
    pub name:       String,
    pub parent_id:  Option<String>,
    pub path:       String,
    pub depth:      i32,
    pub created_at: String,
    pub updated_at: String,
}

// --- ハンドラ ---

/// GET /api/v1/folders
///
/// テナント内のフォルダ一覧を path 順で取得する。
#[utoipa::path(
   get,
   path = "/api/v1/folders",
   tag = "folders",
   security(("session_auth" = [])),
   responses(
      (status = 200, description = "フォルダ一覧", body = ApiResponse<Vec<FolderData>>),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn list_folders(
    State(state): State<Arc<FolderState>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_folders(*session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("フォルダ一覧取得", e))?;

    let items: Vec<FolderData> = core_response
        .data
        .into_iter()
        .map(|dto| FolderData {
            id:         dto.id.to_string(),
            name:       dto.name,
            parent_id:  dto.parent_id.map(|p| p.to_string()),
            path:       dto.path,
            depth:      dto.depth,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        })
        .collect();
    let response = ApiResponse::new(items);
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/folders
///
/// フォルダを作成する。
#[utoipa::path(
   post,
   path = "/api/v1/folders",
   tag = "folders",
   security(("session_auth" = [])),
   request_body = CreateFolderRequest,
   responses(
      (status = 201, description = "フォルダ作成成功", body = ApiResponse<FolderData>),
      (status = 400, description = "バリデーションエラー", body = ErrorResponse),
      (status = 409, description = "フォルダ名重複", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn create_folder(
    State(state): State<Arc<FolderState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<CreateFolderRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = CreateFolderCoreRequest {
        tenant_id:  *session_data.tenant_id().as_uuid(),
        name:       req.name,
        parent_id:  req.parent_id,
        created_by: *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .create_folder(&core_request)
        .await
        .map_err(|e| log_and_convert_core_error("フォルダ作成", e))?;

    let dto = core_response.data;
    let response = ApiResponse::new(FolderData {
        id:         dto.id.to_string(),
        name:       dto.name,
        parent_id:  dto.parent_id.map(|p| p.to_string()),
        path:       dto.path,
        depth:      dto.depth,
        created_at: dto.created_at,
        updated_at: dto.updated_at,
    });
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// PUT /api/v1/folders/{folder_id}
///
/// フォルダを更新する（名前変更・移動）。
#[utoipa::path(
   put,
   path = "/api/v1/folders/{folder_id}",
   tag = "folders",
   security(("session_auth" = [])),
   params(("folder_id" = Uuid, Path, description = "フォルダID")),
   request_body = UpdateFolderRequest,
   responses(
      (status = 200, description = "フォルダ更新成功", body = ApiResponse<FolderData>),
      (status = 400, description = "バリデーションエラー", body = ErrorResponse),
      (status = 404, description = "フォルダが見つからない", body = ErrorResponse),
      (status = 409, description = "フォルダ名重複", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%folder_id))]
pub async fn update_folder(
    State(state): State<Arc<FolderState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(folder_id): Path<Uuid>,
    Json(req): Json<UpdateFolderRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = UpdateFolderCoreRequest {
        tenant_id: *session_data.tenant_id().as_uuid(),
        name:      req.name,
        parent_id: req.parent_id,
    };

    let core_response = state
        .core_service_client
        .update_folder(folder_id, &core_request)
        .await
        .map_err(|e| log_and_convert_core_error("フォルダ更新", e))?;

    let dto = core_response.data;
    let response = ApiResponse::new(FolderData {
        id:         dto.id.to_string(),
        name:       dto.name,
        parent_id:  dto.parent_id.map(|p| p.to_string()),
        path:       dto.path,
        depth:      dto.depth,
        created_at: dto.created_at,
        updated_at: dto.updated_at,
    });
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// DELETE /api/v1/folders/{folder_id}
///
/// フォルダを削除する。
#[utoipa::path(
   delete,
   path = "/api/v1/folders/{folder_id}",
   tag = "folders",
   security(("session_auth" = [])),
   params(("folder_id" = Uuid, Path, description = "フォルダID")),
   responses(
      (status = 204, description = "削除成功"),
      (status = 400, description = "子フォルダが存在する", body = ErrorResponse),
      (status = 404, description = "フォルダが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%folder_id))]
pub async fn delete_folder(
    State(state): State<Arc<FolderState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(folder_id): Path<Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    state
        .core_service_client
        .delete_folder(folder_id, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("フォルダ削除", e))?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
