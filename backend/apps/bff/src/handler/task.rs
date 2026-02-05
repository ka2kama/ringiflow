//! # タスク API ハンドラ
//!
//! BFF のタスク関連エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/tasks/my` - 自分のタスク一覧
//! - `GET /api/v1/workflows/{display_number}/tasks/{step_display_number}` - タスク詳細（workflow ハンドラに移動）

use std::sync::Arc;

use axum::{
   Json,
   extract::State,
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_infra::SessionManager;
use ringiflow_shared::ApiResponse;
use serde::Serialize;

use super::workflow::{
   UserRefData,
   WorkflowData,
   WorkflowState,
   WorkflowStepData,
   extract_tenant_id,
   get_session,
   internal_error_response,
};
use crate::client::CoreServiceClient;

// --- レスポンス型 ---

/// ワークフロー概要データ（タスク一覧用）
#[derive(Debug, Serialize)]
pub struct TaskWorkflowSummaryData {
   pub id:           String,
   pub display_id:   String,
   pub title:        String,
   pub status:       String,
   pub initiated_by: UserRefData,
   pub submitted_at: Option<String>,
}

impl From<crate::client::TaskWorkflowSummaryDto> for TaskWorkflowSummaryData {
   fn from(dto: crate::client::TaskWorkflowSummaryDto) -> Self {
      Self {
         id:           dto.id,
         display_id:   dto.display_id,
         title:        dto.title,
         status:       dto.status,
         initiated_by: UserRefData::from(dto.initiated_by),
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
   pub assigned_to: Option<UserRefData>,
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
         assigned_to: dto.assigned_to.map(UserRefData::from),
         due_date:    dto.due_date,
         started_at:  dto.started_at,
         created_at:  dto.created_at,
         workflow:    TaskWorkflowSummaryData::from(dto.workflow),
      }
   }
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
         let response = ApiResponse::new(
            core_response
               .data
               .into_iter()
               .map(TaskItemData::from)
               .collect::<Vec<_>>(),
         );
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(e) => {
         tracing::error!("タスク一覧取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}
