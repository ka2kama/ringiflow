//! ワークフローハンドラの状態変更操作

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_shared::ApiResponse;

use super::{
    ApproveRejectRequest,
    CreateWorkflowRequest,
    PostCommentRequest,
    ResubmitWorkflowRequest,
    StepPathParams,
    SubmitWorkflowRequest,
    WorkflowCommentData,
    WorkflowData,
    WorkflowState,
};
use crate::error::{authenticate, log_and_convert_core_error, validation_error_response};

/// POST /api/v1/workflows
///
/// ワークフローを作成する（下書き）
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows` を呼び出し
/// 3. レスポンスを返す
#[utoipa::path(
   post,
   path = "/api/v1/workflows",
   tag = "workflows",
   security(("session_auth" = [])),
   request_body = CreateWorkflowRequest,
   responses(
      (status = 201, description = "ワークフロー作成", body = ApiResponse<WorkflowData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "定義が見つからない", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn create_workflow(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_req = crate::client::CreateWorkflowRequest {
        definition_id: req.definition_id,
        title:         req.title,
        form_data:     req.form_data,
        tenant_id:     *session_data.tenant_id().as_uuid(),
        user_id:       *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .create_workflow(core_req)
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー作成", e))?;

    let response = ApiResponse::new(WorkflowData::from(core_response.data));
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// POST /api/v1/workflows/{display_number}/submit
///
/// ワークフローを申請する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{display_number}/submit` を呼び出し
/// 3. レスポンスを返す
#[utoipa::path(
   post,
   path = "/api/v1/workflows/{display_number}/submit",
   tag = "workflows",
   security(("session_auth" = [])),
   params(("display_number" = i64, Path, description = "ワークフロー表示番号")),
   request_body = SubmitWorkflowRequest,
   responses(
      (status = 200, description = "申請成功", body = ApiResponse<WorkflowData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "ワークフローが見つからない", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn submit_workflow(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(display_number): Path<i64>,
    Json(req): Json<SubmitWorkflowRequest>,
) -> Result<Response, Response> {
    if display_number <= 0 {
        return Err(validation_error_response(
            "display_number は 1 以上である必要があります",
        ));
    }

    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_req = crate::client::SubmitWorkflowRequest {
        approvers: req
            .approvers
            .into_iter()
            .map(|a| crate::client::StepApproverRequest {
                step_id:     a.step_id,
                assigned_to: a.assigned_to,
            })
            .collect(),
        tenant_id: *session_data.tenant_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .submit_workflow_by_display_number(display_number, core_req)
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー申請", e))?;

    let response = ApiResponse::new(WorkflowData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

// ===== 承認/却下ハンドラ =====

/// POST /api/v1/workflows/{display_number}/steps/{step_display_number}/approve
///
/// ワークフローステップを承認する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/approve` を呼び出し
/// 3. 200 OK + 更新されたワークフローを返す
#[utoipa::path(
   post,
   path = "/api/v1/workflows/{display_number}/steps/{step_display_number}/approve",
   tag = "workflows",
   security(("session_auth" = [])),
   params(StepPathParams),
   request_body = ApproveRejectRequest,
   responses(
      (status = 200, description = "承認成功", body = ApiResponse<WorkflowData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse),
      (status = 403, description = "権限なし", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "ステップが見つからない", body = ringiflow_shared::ErrorResponse),
      (status = 409, description = "競合", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number = params.display_number, step_display_number = params.step_display_number))]
pub async fn approve_step(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(params): Path<StepPathParams>,
    Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, Response> {
    if params.display_number <= 0 {
        return Err(validation_error_response(
            "display_number は 1 以上である必要があります",
        ));
    }
    if params.step_display_number <= 0 {
        return Err(validation_error_response(
            "step_display_number は 1 以上である必要があります",
        ));
    }

    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_req = crate::client::ApproveRejectRequest {
        version:   req.version,
        comment:   req.comment,
        tenant_id: *session_data.tenant_id().as_uuid(),
        user_id:   *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .approve_step_by_display_number(params.display_number, params.step_display_number, core_req)
        .await
        .map_err(|e| log_and_convert_core_error("ステップ承認", e))?;

    let response = ApiResponse::new(WorkflowData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/workflows/{display_number}/steps/{step_display_number}/reject
///
/// ワークフローステップを却下する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/reject` を呼び出し
/// 3. 200 OK + 更新されたワークフローを返す
#[utoipa::path(
   post,
   path = "/api/v1/workflows/{display_number}/steps/{step_display_number}/reject",
   tag = "workflows",
   security(("session_auth" = [])),
   params(StepPathParams),
   request_body = ApproveRejectRequest,
   responses(
      (status = 200, description = "却下成功", body = ApiResponse<WorkflowData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse),
      (status = 403, description = "権限なし", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "ステップが見つからない", body = ringiflow_shared::ErrorResponse),
      (status = 409, description = "競合", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number = params.display_number, step_display_number = params.step_display_number))]
pub async fn reject_step(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(params): Path<StepPathParams>,
    Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, Response> {
    if params.display_number <= 0 {
        return Err(validation_error_response(
            "display_number は 1 以上である必要があります",
        ));
    }
    if params.step_display_number <= 0 {
        return Err(validation_error_response(
            "step_display_number は 1 以上である必要があります",
        ));
    }

    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_req = crate::client::ApproveRejectRequest {
        version:   req.version,
        comment:   req.comment,
        tenant_id: *session_data.tenant_id().as_uuid(),
        user_id:   *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .reject_step_by_display_number(params.display_number, params.step_display_number, core_req)
        .await
        .map_err(|e| log_and_convert_core_error("ステップ却下", e))?;

    let response = ApiResponse::new(WorkflowData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/workflows/{display_number}/steps/{step_display_number}/request-changes
///
/// ワークフローステップを差し戻す
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/request-changes` を呼び出し
/// 3. 200 OK + 更新されたワークフローを返す
#[utoipa::path(
   post,
   path = "/api/v1/workflows/{display_number}/steps/{step_display_number}/request-changes",
   tag = "workflows",
   security(("session_auth" = [])),
   params(StepPathParams),
   request_body = ApproveRejectRequest,
   responses(
      (status = 200, description = "差し戻し成功", body = ApiResponse<WorkflowData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse),
      (status = 403, description = "権限なし", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "ステップが見つからない", body = ringiflow_shared::ErrorResponse),
      (status = 409, description = "競合", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number = params.display_number, step_display_number = params.step_display_number))]
pub async fn request_changes_step(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(params): Path<StepPathParams>,
    Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, Response> {
    if params.display_number <= 0 {
        return Err(validation_error_response(
            "display_number は 1 以上である必要があります",
        ));
    }
    if params.step_display_number <= 0 {
        return Err(validation_error_response(
            "step_display_number は 1 以上である必要があります",
        ));
    }

    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_req = crate::client::ApproveRejectRequest {
        version:   req.version,
        comment:   req.comment,
        tenant_id: *session_data.tenant_id().as_uuid(),
        user_id:   *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .request_changes_step_by_display_number(
            params.display_number,
            params.step_display_number,
            core_req,
        )
        .await
        .map_err(|e| log_and_convert_core_error("ステップ差し戻し", e))?;

    let response = ApiResponse::new(WorkflowData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/workflows/{display_number}/resubmit
///
/// ワークフローを再申請する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{dn}/resubmit` を呼び出し
/// 3. 200 OK + 更新されたワークフローを返す
#[utoipa::path(
   post,
   path = "/api/v1/workflows/{display_number}/resubmit",
   tag = "workflows",
   security(("session_auth" = [])),
   params(("display_number" = i64, Path, description = "ワークフロー表示番号")),
   request_body = ResubmitWorkflowRequest,
   responses(
      (status = 200, description = "再申請成功", body = ApiResponse<WorkflowData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse),
      (status = 403, description = "権限なし", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "ワークフローが見つからない", body = ringiflow_shared::ErrorResponse),
      (status = 409, description = "競合", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn resubmit_workflow(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(display_number): Path<i64>,
    Json(req): Json<ResubmitWorkflowRequest>,
) -> Result<Response, Response> {
    if display_number <= 0 {
        return Err(validation_error_response(
            "display_number は 1 以上である必要があります",
        ));
    }

    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_req = crate::client::ResubmitWorkflowRequest {
        form_data: req.form_data,
        approvers: req
            .approvers
            .into_iter()
            .map(|a| crate::client::StepApproverRequest {
                step_id:     a.step_id,
                assigned_to: a.assigned_to,
            })
            .collect(),
        version:   req.version,
        tenant_id: *session_data.tenant_id().as_uuid(),
        user_id:   *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .resubmit_workflow_by_display_number(display_number, core_req)
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー再申請", e))?;

    let response = ApiResponse::new(WorkflowData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

// ===== コメントハンドラ =====

/// POST /api/v1/workflows/{display_number}/comments
///
/// ワークフローにコメントを投稿する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{display_number}/comments` を呼び出し
/// 3. 201 Created + コメントを返す
#[utoipa::path(
   post,
   path = "/api/v1/workflows/{display_number}/comments",
   tag = "workflows",
   security(("session_auth" = [])),
   params(("display_number" = i64, Path, description = "ワークフロー表示番号")),
   request_body = PostCommentRequest,
   responses(
      (status = 201, description = "コメント投稿成功", body = ApiResponse<WorkflowCommentData>),
      (status = 400, description = "バリデーションエラー", body = ringiflow_shared::ErrorResponse),
      (status = 403, description = "権限なし", body = ringiflow_shared::ErrorResponse),
      (status = 404, description = "ワークフローが見つからない", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn post_comment(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(display_number): Path<i64>,
    Json(req): Json<PostCommentRequest>,
) -> Result<Response, Response> {
    if display_number <= 0 {
        return Err(validation_error_response(
            "display_number は 1 以上である必要があります",
        ));
    }

    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_req = crate::client::PostCommentCoreRequest {
        body:      req.body,
        tenant_id: *session_data.tenant_id().as_uuid(),
        user_id:   *session_data.user_id().as_uuid(),
    };

    let core_response = state
        .core_service_client
        .post_comment(display_number, core_req)
        .await
        .map_err(|e| log_and_convert_core_error("コメント投稿", e))?;

    let response = ApiResponse::new(WorkflowCommentData::from(core_response.data));
    Ok((StatusCode::CREATED, Json(response)).into_response())
}
