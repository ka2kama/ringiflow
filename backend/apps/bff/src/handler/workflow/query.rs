//! ワークフローハンドラの読み取り操作

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
    StepPathParams,
    WorkflowCommentData,
    WorkflowData,
    WorkflowDefinitionData,
    WorkflowState,
};
use crate::{
    client::CoreServiceError,
    error::{
        authenticate,
        log_and_convert_core_error,
        not_found_response,
        validation_error_response,
    },
};

/// GET /api/v1/workflow-definitions
///
/// ワークフロー定義一覧を取得する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `GET /internal/workflow-definitions` を呼び出し
/// 3. レスポンスを返す
#[utoipa::path(
   get,
   path = "/api/v1/workflow-definitions",
   tag = "workflows",
   security(("session_auth" = [])),
   responses(
      (status = 200, description = "ワークフロー定義一覧", body = ApiResponse<Vec<WorkflowDefinitionData>>),
      (status = 401, description = "認証エラー", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn list_workflow_definitions(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_workflow_definitions(*session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義一覧取得", e))?;

    let response = ApiResponse::new(
        core_response
            .data
            .into_iter()
            .map(WorkflowDefinitionData::from)
            .collect::<Vec<_>>(),
    );
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// GET /api/v1/workflow-definitions/{id}
///
/// ワークフロー定義の詳細を取得する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `GET /internal/workflow-definitions/{id}` を呼び出し
/// 3. レスポンスを返す
#[utoipa::path(
   get,
   path = "/api/v1/workflow-definitions/{id}",
   tag = "workflows",
   security(("session_auth" = [])),
   params(("id" = uuid::Uuid, Path, description = "ワークフロー定義 ID")),
   responses(
      (status = 200, description = "ワークフロー定義詳細", body = ApiResponse<WorkflowDefinitionData>),
      (status = 404, description = "定義が見つからない", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(%definition_id))]
pub async fn get_workflow_definition(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(definition_id): Path<uuid::Uuid>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .get_workflow_definition(definition_id, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義取得", e))?;

    let response = ApiResponse::new(WorkflowDefinitionData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// GET /api/v1/workflows
///
/// 自分のワークフロー一覧を取得する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `GET /internal/workflows` を呼び出し
/// 3. レスポンスを返す
#[utoipa::path(
   get,
   path = "/api/v1/workflows",
   tag = "workflows",
   security(("session_auth" = [])),
   responses(
      (status = 200, description = "自分のワークフロー一覧", body = ApiResponse<Vec<WorkflowData>>),
      (status = 401, description = "認証エラー", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn list_my_workflows(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_my_workflows(
            *session_data.tenant_id().as_uuid(),
            *session_data.user_id().as_uuid(),
        )
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー一覧取得", e))?;

    let response = ApiResponse::new(
        core_response
            .data
            .into_iter()
            .map(WorkflowData::from)
            .collect::<Vec<_>>(),
    );
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// GET /api/v1/workflows/{display_number}
///
/// ワークフローの詳細を取得する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `GET /internal/workflows/by-display-number/{display_number}` を呼び出し
/// 3. レスポンスを返す
#[utoipa::path(
   get,
   path = "/api/v1/workflows/{display_number}",
   tag = "workflows",
   security(("session_auth" = [])),
   params(("display_number" = i64, Path, description = "ワークフロー表示番号")),
   responses(
      (status = 200, description = "ワークフロー詳細", body = ApiResponse<WorkflowData>),
      (status = 404, description = "ワークフローが見つからない", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn get_workflow(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(display_number): Path<i64>,
) -> Result<Response, Response> {
    if display_number <= 0 {
        return Err(validation_error_response(
            "display_number は 1 以上である必要があります",
        ));
    }

    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .get_workflow_by_display_number(display_number, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("ワークフロー取得", e))?;

    let response = ApiResponse::new(WorkflowData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}

// ===== タスクハンドラ =====

/// GET /api/v1/workflows/{display_number}/tasks/{step_display_number}
///
/// display_number でタスク詳細を取得する
#[utoipa::path(
   get,
   path = "/api/v1/workflows/{display_number}/tasks/{step_display_number}",
   tag = "tasks",
   security(("session_auth" = [])),
   params(StepPathParams),
   responses(
      (status = 200, description = "タスク詳細", body = ApiResponse<crate::handler::task::TaskDetailData>),
      (status = 404, description = "タスクが見つからない", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number = params.display_number, step_display_number = params.step_display_number))]
pub async fn get_task_by_display_numbers(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(params): Path<StepPathParams>,
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

    let core_response = state
        .core_service_client
        .get_task_by_display_numbers(
            params.display_number,
            params.step_display_number,
            *session_data.tenant_id().as_uuid(),
            *session_data.user_id().as_uuid(),
        )
        .await
        .map_err(|e| match e {
            // タスクコンテキストでは StepNotFound を task-not-found として返す
            CoreServiceError::StepNotFound => {
                not_found_response("task-not-found", "Task Not Found", "タスクが見つかりません")
            }
            e => log_and_convert_core_error("タスク詳細取得", e),
        })?;

    let response = ApiResponse::new(crate::handler::task::TaskDetailData::from(
        core_response.data,
    ));
    Ok((StatusCode::OK, Json(response)).into_response())
}

// ===== コメントハンドラ =====

/// GET /api/v1/workflows/{display_number}/comments
///
/// ワークフローのコメント一覧を取得する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `GET /internal/workflows/by-display-number/{display_number}/comments` を呼び出し
/// 3. 200 OK + コメント一覧を返す
#[utoipa::path(
   get,
   path = "/api/v1/workflows/{display_number}/comments",
   tag = "workflows",
   security(("session_auth" = [])),
   params(("display_number" = i64, Path, description = "ワークフロー表示番号")),
   responses(
      (status = 200, description = "コメント一覧", body = ApiResponse<Vec<WorkflowCommentData>>),
      (status = 404, description = "ワークフローが見つからない", body = ringiflow_shared::ErrorResponse)
   )
)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn list_comments(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Path(display_number): Path<i64>,
) -> Result<Response, Response> {
    if display_number <= 0 {
        return Err(validation_error_response(
            "display_number は 1 以上である必要があります",
        ));
    }

    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_comments(display_number, *session_data.tenant_id().as_uuid())
        .await
        .map_err(|e| log_and_convert_core_error("コメント一覧取得", e))?;

    let response = ApiResponse::new(
        core_response
            .data
            .into_iter()
            .map(WorkflowCommentData::from)
            .collect::<Vec<_>>(),
    );
    Ok((StatusCode::OK, Json(response)).into_response())
}
