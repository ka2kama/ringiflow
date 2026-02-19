//! # タスク API ハンドラ
//!
//! BFF のタスク関連エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/tasks/my` - 自分のタスク一覧
//! - `GET /api/v1/workflows/{display_number}/tasks/{step_display_number}` -
//!   タスク詳細（workflow ハンドラに移動）

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_shared::{ApiResponse, ErrorResponse};
use serde::Serialize;
use utoipa::ToSchema;

use super::workflow::{UserRefData, WorkflowData, WorkflowState, WorkflowStepData};
use crate::error::{authenticate, log_and_convert_core_error};

// --- レスポンス型 ---

/// ワークフロー概要データ（タスク一覧用）
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskWorkflowSummaryData {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub title: String,
    pub status: String,
    pub initiated_by: UserRefData,
    pub submitted_at: Option<String>,
}

impl From<crate::client::TaskWorkflowSummaryDto> for TaskWorkflowSummaryData {
    fn from(dto: crate::client::TaskWorkflowSummaryDto) -> Self {
        Self {
            id: dto.id,
            display_id: dto.display_id,
            display_number: dto.display_number,
            title: dto.title,
            status: dto.status,
            initiated_by: UserRefData::from(dto.initiated_by),
            submitted_at: dto.submitted_at,
        }
    }
}

/// タスク一覧の要素データ
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskItemData {
    pub id: String,
    pub display_number: i64,
    pub step_name: String,
    pub status: String,
    pub version: i32,
    pub assigned_to: Option<UserRefData>,
    pub due_date: Option<String>,
    pub started_at: Option<String>,
    pub created_at: String,
    pub workflow: TaskWorkflowSummaryData,
}

impl From<crate::client::TaskItemDto> for TaskItemData {
    fn from(dto: crate::client::TaskItemDto) -> Self {
        Self {
            id: dto.id,
            display_number: dto.display_number,
            step_name: dto.step_name,
            status: dto.status,
            version: dto.version,
            assigned_to: dto.assigned_to.map(UserRefData::from),
            due_date: dto.due_date,
            started_at: dto.started_at,
            created_at: dto.created_at,
            workflow: TaskWorkflowSummaryData::from(dto.workflow),
        }
    }
}

/// タスク詳細データ
#[derive(Debug, Serialize, ToSchema)]
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
#[utoipa::path(
   get,
   path = "/api/v1/tasks/my",
   tag = "tasks",
   security(("session_auth" = [])),
   responses(
      (status = 200, description = "タスク一覧", body = ApiResponse<Vec<TaskItemData>>),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn list_my_tasks(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .list_my_tasks(
            *session_data.tenant_id().as_uuid(),
            *session_data.user_id().as_uuid(),
        )
        .await
        .map_err(|e| log_and_convert_core_error("タスク一覧取得", e))?;

    let response = ApiResponse::new(
        core_response
            .data
            .into_iter()
            .map(TaskItemData::from)
            .collect::<Vec<_>>(),
    );
    Ok((StatusCode::OK, Json(response)).into_response())
}
