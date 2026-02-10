//! ワークフローハンドラの読み取り操作

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, State},
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{StepPathParams, WorkflowData, WorkflowDefinitionData, WorkflowState};
use crate::{
   client::CoreServiceError,
   error::{
      extract_tenant_id,
      forbidden_response,
      get_session,
      internal_error_response,
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
pub async fn list_workflow_definitions(
   State(state): State<Arc<WorkflowState>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse {
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // Core Service を呼び出し
   match state
      .core_service_client
      .list_workflow_definitions(*session_data.tenant_id().as_uuid())
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(
            core_response
               .data
               .into_iter()
               .map(WorkflowDefinitionData::from)
               .collect::<Vec<_>>(),
         );
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(e) => {
         tracing::error!("ワークフロー定義一覧取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
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
pub async fn get_workflow_definition(
   State(state): State<Arc<WorkflowState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(definition_id): Path<Uuid>,
) -> impl IntoResponse {
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // Core Service を呼び出し
   match state
      .core_service_client
      .get_workflow_definition(definition_id, *session_data.tenant_id().as_uuid())
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowDefinitionData::from(core_response.data));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::WorkflowDefinitionNotFound) => not_found_response(
         "workflow-definition-not-found",
         "Workflow Definition Not Found",
         "ワークフロー定義が見つかりません",
      ),
      Err(e) => {
         tracing::error!("ワークフロー定義取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
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
pub async fn list_my_workflows(
   State(state): State<Arc<WorkflowState>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse {
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // Core Service を呼び出し
   match state
      .core_service_client
      .list_my_workflows(
         *session_data.tenant_id().as_uuid(),
         *session_data.user_id().as_uuid(),
      )
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(
            core_response
               .data
               .into_iter()
               .map(WorkflowData::from)
               .collect::<Vec<_>>(),
         );
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(e) => {
         tracing::error!("ワークフロー一覧取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
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
pub async fn get_workflow(
   State(state): State<Arc<WorkflowState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(display_number): Path<i64>,
) -> impl IntoResponse {
   // display_number の検証
   if display_number <= 0 {
      return validation_error_response("display_number は 1 以上である必要があります");
   }

   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // Core Service を呼び出し
   match state
      .core_service_client
      .get_workflow_by_display_number(display_number, *session_data.tenant_id().as_uuid())
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowData::from(core_response.data));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::WorkflowInstanceNotFound) => not_found_response(
         "workflow-instance-not-found",
         "Workflow Instance Not Found",
         "ワークフローインスタンスが見つかりません",
      ),
      Err(e) => {
         tracing::error!("ワークフロー取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

// ===== タスクハンドラ =====

/// GET /api/v1/workflows/{display_number}/tasks/{step_display_number}
///
/// display_number でタスク詳細を取得する
pub async fn get_task_by_display_numbers(
   State(state): State<Arc<WorkflowState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(params): Path<StepPathParams>,
) -> impl IntoResponse {
   // display_number の検証
   if params.display_number <= 0 {
      return validation_error_response("display_number は 1 以上である必要があります");
   }
   if params.step_display_number <= 0 {
      return validation_error_response("step_display_number は 1 以上である必要があります");
   }

   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   match state
      .core_service_client
      .get_task_by_display_numbers(
         params.display_number,
         params.step_display_number,
         *session_data.tenant_id().as_uuid(),
         *session_data.user_id().as_uuid(),
      )
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(crate::handler::task::TaskDetailData::from(
            core_response.data,
         ));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::StepNotFound) => {
         not_found_response("task-not-found", "Task Not Found", "タスクが見つかりません")
      }
      Err(CoreServiceError::WorkflowInstanceNotFound) => not_found_response(
         "workflow-instance-not-found",
         "Workflow Instance Not Found",
         "ワークフローインスタンスが見つかりません",
      ),
      Err(CoreServiceError::Forbidden(detail)) => forbidden_response(&detail),
      Err(e) => {
         tracing::error!("タスク詳細取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}
