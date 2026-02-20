//! # ワークフロー定義管理 API ハンドラ
//!
//! BFF のワークフロー定義管理エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `POST /api/v1/workflow-definitions` - 新規作成（Draft）
//! - `PUT /api/v1/workflow-definitions/{id}` - 更新（Draft のみ）
//! - `DELETE /api/v1/workflow-definitions/{id}` - 削除（Draft のみ）
//! - `POST /api/v1/workflow-definitions/{id}/publish` - 公開
//! - `POST /api/v1/workflow-definitions/{id}/archive` - アーカイブ
//! - `POST /api/v1/workflow-definitions/validate` - バリデーション
//!
//! GET（一覧・詳細）は認可不要のため `WorkflowState` に残す。

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_infra::SessionManager;
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    client::{
        CoreServiceWorkflowClient,
        CreateDefinitionCoreRequest,
        PublishArchiveCoreRequest,
        UpdateDefinitionCoreRequest,
        ValidateDefinitionCoreRequest,
    },
    error::{authenticate, log_and_convert_core_error},
    handler::workflow::WorkflowDefinitionData,
};

/// ワークフロー定義管理 API の共有状態
pub struct WorkflowDefinitionState {
    pub core_service_client: Arc<dyn CoreServiceWorkflowClient>,
    pub session_manager:     Arc<dyn SessionManager>,
}

// --- リクエスト型 ---

/// 定義作成リクエスト（BFF 公開 API）
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDefinitionRequest {
    /// ワークフロー定義名
    pub name:        String,
    /// 説明（任意）
    pub description: Option<String>,
    /// 定義 JSON
    pub definition:  serde_json::Value,
}

/// 定義更新リクエスト（BFF 公開 API）
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDefinitionRequest {
    /// ワークフロー定義名
    pub name:        String,
    /// 説明（任意）
    pub description: Option<String>,
    /// 定義 JSON
    pub definition:  serde_json::Value,
    /// 楽観的ロック用バージョン
    pub version:     i32,
}

/// 公開/アーカイブリクエスト（BFF 公開 API）
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishArchiveRequest {
    /// 楽観的ロック用バージョン
    pub version: i32,
}

/// バリデーションリクエスト（BFF 公開 API）
#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidateDefinitionRequest {
    /// 検証対象の定義 JSON
    pub definition: serde_json::Value,
}

// --- レスポンス型 ---

/// バリデーション結果データ
#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationResultData {
    pub valid:  bool,
    pub errors: Vec<ValidationErrorData>,
}

/// バリデーションエラーデータ
#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationErrorData {
    pub code:    String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
}

// --- ハンドラ ---

/// POST /api/v1/workflow-definitions
///
/// ワークフロー定義を新規作成する（Draft 状態）。
#[utoipa::path(
   post,
   path = "/api/v1/workflow-definitions",
   tag = "workflow-definitions",
   security(("session_auth" = [])),
   request_body = CreateDefinitionRequest,
   responses(
      (status = 201, description = "定義作成成功", body = ApiResponse<WorkflowDefinitionData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn create_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<CreateDefinitionRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = CreateDefinitionCoreRequest {
        name:        req.name,
        description: req.description,
        definition:  req.definition,
        tenant_id:   *session_data.tenant_id().as_uuid(),
        user_id:     *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .create_workflow_definition(&core_request)
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義作成", e))?;

    let response = ApiResponse::new(WorkflowDefinitionData::from(core_response.data));
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// PUT /api/v1/workflow-definitions/{id}
///
/// ワークフロー定義を更新する（Draft のみ）。
#[utoipa::path(
   put,
   path = "/api/v1/workflow-definitions/{id}",
   tag = "workflow-definitions",
   security(("session_auth" = [])),
   params(("id" = Uuid, Path, description = "ワークフロー定義 ID")),
   request_body = UpdateDefinitionRequest,
   responses(
      (status = 200, description = "定義更新成功", body = ApiResponse<WorkflowDefinitionData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "定義が見つからない", body = ringiflow_shared::ErrorResponse),
      (status = 409, description = "バージョン競合", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%definition_id))]
pub async fn update_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(definition_id): Path<Uuid>,
    Json(req): Json<UpdateDefinitionRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = UpdateDefinitionCoreRequest {
        name:        req.name,
        description: req.description,
        definition:  req.definition,
        version:     req.version,
        tenant_id:   *session_data.tenant_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .update_workflow_definition(definition_id, &core_request)
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義更新", e))?;

    let response = ApiResponse::new(WorkflowDefinitionData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// DELETE /api/v1/workflow-definitions/{id}
///
/// ワークフロー定義を削除する（Draft のみ）。
#[utoipa::path(
   delete,
   path = "/api/v1/workflow-definitions/{id}",
   tag = "workflow-definitions",
   security(("session_auth" = [])),
   params(("id" = Uuid, Path, description = "ワークフロー定義 ID")),
   responses(
      (status = 204, description = "削除成功"),
      (status = 400, description = "Draft 以外の削除", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "定義が見つからない", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%definition_id))]
pub async fn delete_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(definition_id): Path<Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    state
        .core_service_client
        .delete_workflow_definition(definition_id, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義削除", e))?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// POST /api/v1/workflow-definitions/{id}/publish
///
/// ワークフロー定義を公開する（Draft → Published）。
#[utoipa::path(
   post,
   path = "/api/v1/workflow-definitions/{id}/publish",
   tag = "workflow-definitions",
   security(("session_auth" = [])),
   params(("id" = Uuid, Path, description = "ワークフロー定義 ID")),
   request_body = PublishArchiveRequest,
   responses(
      (status = 200, description = "公開成功", body = ApiResponse<WorkflowDefinitionData>),
      (status = 400, description = "バリデーション失敗 or Draft 以外", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "定義が見つからない", body = ringiflow_shared::ErrorResponse),
      (status = 409, description = "バージョン競合", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%definition_id))]
pub async fn publish_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(definition_id): Path<Uuid>,
    Json(req): Json<PublishArchiveRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = PublishArchiveCoreRequest {
        version:   req.version,
        tenant_id: *session_data.tenant_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .publish_workflow_definition(definition_id, &core_request)
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義公開", e))?;

    let response = ApiResponse::new(WorkflowDefinitionData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/workflow-definitions/{id}/archive
///
/// ワークフロー定義をアーカイブする（Published → Archived）。
#[utoipa::path(
   post,
   path = "/api/v1/workflow-definitions/{id}/archive",
   tag = "workflow-definitions",
   security(("session_auth" = [])),
   params(("id" = Uuid, Path, description = "ワークフロー定義 ID")),
   request_body = PublishArchiveRequest,
   responses(
      (status = 200, description = "アーカイブ成功", body = ApiResponse<WorkflowDefinitionData>),
      (status = 400, description = "Published 以外", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "定義が見つからない", body = ringiflow_shared::ErrorResponse),
      (status = 409, description = "バージョン競合", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%definition_id))]
pub async fn archive_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(definition_id): Path<Uuid>,
    Json(req): Json<PublishArchiveRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = PublishArchiveCoreRequest {
        version:   req.version,
        tenant_id: *session_data.tenant_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .archive_workflow_definition(definition_id, &core_request)
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義アーカイブ", e))?;

    let response = ApiResponse::new(WorkflowDefinitionData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/workflow-definitions/validate
///
/// ワークフロー定義 JSON のバリデーションのみ実行する。保存は行わない。
#[utoipa::path(
   post,
   path = "/api/v1/workflow-definitions/validate",
   tag = "workflow-definitions",
   security(("session_auth" = [])),
   request_body = ValidateDefinitionRequest,
   responses(
      (status = 200, description = "バリデーション結果", body = ApiResponse<ValidationResultData>),
      (status = 400, description = "リクエスト不正", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn validate_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<ValidateDefinitionRequest>,
) -> Result<Response, Response> {
    // バリデーションにも認証は必要（ログインユーザーのみ使用可能）
    let _session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_request = ValidateDefinitionCoreRequest {
        definition: req.definition,
    };

    let core_response = state
        .core_service_client
        .validate_workflow_definition(&core_request)
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義バリデーション", e))?;

    let result = core_response.data;
    let response = ApiResponse::new(ValidationResultData {
        valid:  result.valid,
        errors: result
            .errors
            .into_iter()
            .map(|e| ValidationErrorData {
                code:    e.code,
                message: e.message,
                step_id: e.step_id,
            })
            .collect(),
    });
    Ok((StatusCode::OK, Json(response)).into_response())
}
