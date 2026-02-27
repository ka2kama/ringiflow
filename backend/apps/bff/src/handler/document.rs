//! # ドキュメント管理 API ハンドラ
//!
//! BFF のドキュメント管理エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `POST /api/v1/documents/upload-url` - Upload URL 発行
//! - `POST /api/v1/documents/{document_id}/confirm` - アップロード完了確認
//! - `POST /api/v1/documents/{document_id}/download-url` - ダウンロード URL 発行
//! - `DELETE /api/v1/documents/{document_id}` - ドキュメント削除（ソフトデリート）
//! - `GET /api/v1/documents` - フォルダ内ドキュメント一覧
//! - `GET /api/v1/workflows/{workflow_instance_id}/attachments` - ワークフロー添付ファイル一覧

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_infra::SessionManager;
use ringiflow_shared::{ApiResponse, ErrorResponse};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    client::{CoreServiceDocumentClient, RequestUploadUrlCoreRequest},
    error::{authenticate, log_and_convert_core_error},
};

/// ドキュメント管理 API の共有状態
pub struct DocumentState {
    pub core_service_client: Arc<dyn CoreServiceDocumentClient>,
    pub session_manager:     Arc<dyn SessionManager>,
}

// --- リクエスト型 ---

/// Upload URL 発行リクエスト
///
/// `tenant_id` と `uploaded_by` はセッションから取得するため、
/// フロントエンドからは指定しない。
#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestUploadUrlRequest {
    pub filename: String,
    pub content_type: String,
    pub content_length: i64,
    pub folder_id: Option<Uuid>,
    pub workflow_instance_id: Option<Uuid>,
}

// --- レスポンス型 ---

/// Upload URL データ
#[derive(Debug, Serialize, ToSchema)]
pub struct UploadUrlData {
    pub document_id: String,
    pub upload_url:  String,
    pub expires_in:  u64,
}

/// ドキュメントデータ
#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentData {
    pub id:           String,
    pub filename:     String,
    pub content_type: String,
    pub size:         i64,
    pub status:       String,
    pub created_at:   String,
}

/// ダウンロード URL データ
#[derive(Debug, Serialize, ToSchema)]
pub struct DownloadUrlData {
    pub download_url: String,
    pub expires_in:   u64,
}

// --- クエリパラメータ型 ---

/// ドキュメント一覧クエリパラメータ
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListDocumentsQuery {
    pub folder_id: Uuid,
}

// --- ハンドラ ---

/// POST /api/v1/documents/upload-url
///
/// Presigned PUT URL を発行する。
#[utoipa::path(
   post,
   path = "/api/v1/documents/upload-url",
   tag = "documents",
   security(("session_auth" = [])),
   request_body = RequestUploadUrlRequest,
   responses(
      (status = 200, description = "Upload URL 発行成功", body = ApiResponse<UploadUrlData>),
      (status = 400, description = "バリデーションエラー", body = ErrorResponse),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn request_upload_url(
    State(state): State<Arc<DocumentState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<RequestUploadUrlRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = RequestUploadUrlCoreRequest {
        tenant_id: *session_data.tenant_id().as_uuid(),
        filename: req.filename,
        content_type: req.content_type,
        content_length: req.content_length,
        folder_id: req.folder_id,
        workflow_instance_id: req.workflow_instance_id,
        uploaded_by: *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .request_upload_url(&core_request)
        .await
        .map_err(|e| log_and_convert_core_error("Upload URL 発行", e))?;

    let dto = core_response.data;
    let response = ApiResponse::new(UploadUrlData {
        document_id: dto.document_id.to_string(),
        upload_url:  dto.upload_url,
        expires_in:  dto.expires_in,
    });
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/documents/{document_id}/confirm
///
/// アップロード完了を確認し、ドキュメントを active にする。
#[utoipa::path(
   post,
   path = "/api/v1/documents/{document_id}/confirm",
   tag = "documents",
   security(("session_auth" = [])),
   params(("document_id" = Uuid, Path, description = "ドキュメントID")),
   responses(
      (status = 200, description = "アップロード確認成功", body = ApiResponse<DocumentData>),
      (status = 400, description = "ステータスエラー / S3 にファイルが存在しない", body = ErrorResponse),
      (status = 401, description = "認証エラー", body = ErrorResponse),
      (status = 404, description = "ドキュメントが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%document_id))]
pub async fn confirm_upload(
    State(state): State<Arc<DocumentState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(document_id): Path<Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .confirm_upload(document_id, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("アップロード確認", e))?;

    let dto = core_response.data;
    let response = ApiResponse::new(DocumentData {
        id:           dto.id.to_string(),
        filename:     dto.filename,
        content_type: dto.content_type,
        size:         dto.size,
        status:       dto.status,
        created_at:   dto.created_at,
    });
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/documents/{document_id}/download-url
///
/// Presigned GET URL を発行する。
#[utoipa::path(
   post,
   path = "/api/v1/documents/{document_id}/download-url",
   tag = "documents",
   security(("session_auth" = [])),
   params(("document_id" = Uuid, Path, description = "ドキュメントID")),
   responses(
      (status = 200, description = "ダウンロード URL 発行成功", body = ApiResponse<DownloadUrlData>),
      (status = 400, description = "ステータスエラー", body = ErrorResponse),
      (status = 401, description = "認証エラー", body = ErrorResponse),
      (status = 404, description = "ドキュメントが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%document_id))]
pub async fn generate_download_url(
    State(state): State<Arc<DocumentState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(document_id): Path<Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .generate_download_url(document_id, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("ダウンロード URL 発行", e))?;

    let dto = core_response.data;
    let response = ApiResponse::new(DownloadUrlData {
        download_url: dto.download_url,
        expires_in:   dto.expires_in,
    });
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// DELETE /api/v1/documents/{document_id}
///
/// ドキュメントをソフトデリートする。
/// 管理者またはアップロード者本人のみ削除可能。
#[utoipa::path(
   delete,
   path = "/api/v1/documents/{document_id}",
   tag = "documents",
   security(("session_auth" = [])),
   params(("document_id" = Uuid, Path, description = "ドキュメントID")),
   responses(
      (status = 204, description = "削除成功"),
      (status = 400, description = "ワークフロー状態エラー", body = ErrorResponse),
      (status = 401, description = "認証エラー", body = ErrorResponse),
      (status = 403, description = "権限不足", body = ErrorResponse),
      (status = 404, description = "ドキュメントが見つからない", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%document_id))]
pub async fn delete_document(
    State(state): State<Arc<DocumentState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(document_id): Path<Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let is_tenant_admin = session_data.roles().iter().any(|r| r == "tenant_admin");

    state
        .core_service_client
        .delete_document(
            document_id,
            *session_data.tenant_id().as_uuid(),
            *session_data.user_id().as_uuid(),
            is_tenant_admin,
        )
        .await
        .map_err(|e| log_and_convert_core_error("ドキュメント削除", e))?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// GET /api/v1/documents
///
/// フォルダ内のドキュメント一覧を取得する。
#[utoipa::path(
   get,
   path = "/api/v1/documents",
   tag = "documents",
   security(("session_auth" = [])),
   params(ListDocumentsQuery),
   responses(
      (status = 200, description = "ドキュメント一覧", body = ApiResponse<Vec<DocumentData>>),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn list_documents(
    State(state): State<Arc<DocumentState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_documents(*session_data.tenant_id().as_uuid(), query.folder_id)
        .await
        .map_err(|e| log_and_convert_core_error("ドキュメント一覧取得", e))?;

    let documents: Vec<DocumentData> = core_response
        .data
        .into_iter()
        .map(|dto| DocumentData {
            id:           dto.id.to_string(),
            filename:     dto.filename,
            content_type: dto.content_type,
            size:         dto.size,
            status:       dto.status,
            created_at:   dto.created_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(ApiResponse::new(documents))).into_response())
}

/// GET /api/v1/workflows/{workflow_instance_id}/attachments
///
/// ワークフロー添付ファイル一覧を取得する。
#[utoipa::path(
   get,
   path = "/api/v1/workflows/{workflow_instance_id}/attachments",
   tag = "documents",
   security(("session_auth" = [])),
   params(("workflow_instance_id" = Uuid, Path, description = "ワークフローインスタンスID")),
   responses(
      (status = 200, description = "添付ファイル一覧", body = ApiResponse<Vec<DocumentData>>),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%workflow_instance_id))]
pub async fn list_workflow_attachments(
    State(state): State<Arc<DocumentState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(workflow_instance_id): Path<Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_workflow_attachments(workflow_instance_id, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー添付ファイル一覧取得", e))?;

    let documents: Vec<DocumentData> = core_response
        .data
        .into_iter()
        .map(|dto| DocumentData {
            id:           dto.id.to_string(),
            filename:     dto.filename,
            content_type: dto.content_type,
            size:         dto.size,
            status:       dto.status,
            created_at:   dto.created_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(ApiResponse::new(documents))).into_response())
}
