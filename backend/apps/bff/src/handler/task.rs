//! # タスク API ハンドラ
//!
//! BFF のタスク関連エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/tasks/my` - 自分のタスク一覧
//! - `GET /api/v1/tasks/{id}` - タスク詳細

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, State},
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_infra::SessionManager;
use serde::Serialize;
use uuid::Uuid;

use super::workflow::{
   WorkflowData,
   WorkflowState,
   WorkflowStepData,
   extract_tenant_id,
   forbidden_response,
   get_session,
   internal_error_response,
   not_found_response,
};
use crate::client::{CoreServiceClient, CoreServiceError};

// --- レスポンス型 ---

/// ワークフロー概要データ（タスク一覧用）
#[derive(Debug, Serialize)]
pub struct TaskWorkflowSummaryData {
   pub id:           String,
   pub title:        String,
   pub status:       String,
   pub initiated_by: String,
   pub submitted_at: Option<String>,
}

impl From<crate::client::TaskWorkflowSummaryDto> for TaskWorkflowSummaryData {
   fn from(dto: crate::client::TaskWorkflowSummaryDto) -> Self {
      Self {
         id:           dto.id,
         title:        dto.title,
         status:       dto.status,
         initiated_by: dto.initiated_by,
         submitted_at: dto.submitted_at,
      }
   }
}

/// タスク一覧の要素データ
#[derive(Debug, Serialize)]
pub struct TaskItemData {
   pub id:          String,
   pub step_name:   String,
   pub status:      String,
   pub version:     i32,
   pub assigned_to: Option<String>,
   pub due_date:    Option<String>,
   pub started_at:  Option<String>,
   pub created_at:  String,
   pub workflow:    TaskWorkflowSummaryData,
}

impl From<crate::client::TaskItemDto> for TaskItemData {
   fn from(dto: crate::client::TaskItemDto) -> Self {
      Self {
         id:          dto.id,
         step_name:   dto.step_name,
         status:      dto.status,
         version:     dto.version,
         assigned_to: dto.assigned_to,
         due_date:    dto.due_date,
         started_at:  dto.started_at,
         created_at:  dto.created_at,
         workflow:    TaskWorkflowSummaryData::from(dto.workflow),
      }
   }
}

/// タスク一覧レスポンス
#[derive(Debug, Serialize)]
pub struct TaskListResponse {
   pub data: Vec<TaskItemData>,
}

/// タスク詳細データ
#[derive(Debug, Serialize)]
pub struct TaskDetailData {
   pub step:     WorkflowStepData,
   pub workflow: WorkflowData,
}

impl From<crate::client::TaskDetailDto> for TaskDetailData {
   fn from(dto: crate::client::TaskDetailDto) -> Self {
      Self {
         step:     WorkflowStepData::from(dto.step),
         workflow: WorkflowData::from(dto.workflow),
      }
   }
}

/// タスク詳細レスポンス
#[derive(Debug, Serialize)]
pub struct TaskDetailResponse {
   pub data: TaskDetailData,
}

// --- ハンドラ ---

/// GET /api/v1/tasks/my
///
/// 自分のタスク一覧を取得する
pub async fn list_my_tasks<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   match state
      .core_service_client
      .list_my_tasks(
         *session_data.tenant_id().as_uuid(),
         *session_data.user_id().as_uuid(),
      )
      .await
   {
      Ok(core_response) => {
         let response = TaskListResponse {
            data: core_response
               .data
               .into_iter()
               .map(TaskItemData::from)
               .collect(),
         };
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(e) => {
         tracing::error!("タスク一覧取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// GET /api/v1/tasks/{id}
///
/// タスク詳細を取得する
pub async fn get_task<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(task_id): Path<Uuid>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   match state
      .core_service_client
      .get_task(
         task_id,
         *session_data.tenant_id().as_uuid(),
         *session_data.user_id().as_uuid(),
      )
      .await
   {
      Ok(core_response) => {
         let response = TaskDetailResponse {
            data: TaskDetailData::from(core_response.data),
         };
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::StepNotFound) => not_found_response(
         "https://ringiflow.example.com/errors/task-not-found",
         "Task Not Found",
         "タスクが見つかりません",
      ),
      Err(CoreServiceError::Forbidden(detail)) => forbidden_response(&detail),
      Err(e) => {
         tracing::error!("タスク詳細取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}
