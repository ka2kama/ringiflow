//! # タスク API ハンドラ
//!
//! Core Service のタスク関連エンドポイントを実装する。

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, Query, State},
   http::StatusCode,
   response::{IntoResponse, Response},
};
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   workflow::{WorkflowInstance, WorkflowStepId},
};
use ringiflow_infra::repository::{WorkflowInstanceRepository, WorkflowStepRepository};
use ringiflow_shared::ApiResponse;
use serde::Serialize;
use uuid::Uuid;

use crate::{
   error::CoreError,
   handler::workflow::{UserQuery, WorkflowInstanceDto, WorkflowStepDto},
   usecase::task::{TaskDetail, TaskItem, TaskUseCaseImpl},
};

/// タスクハンドラーの State
pub struct TaskState<I, S> {
   pub usecase: TaskUseCaseImpl<I, S>,
}

/// タスク一覧の要素 DTO
#[derive(Debug, Serialize)]
pub struct TaskItemDto {
   pub id:          String,
   pub step_name:   String,
   pub status:      String,
   pub version:     i32,
   pub assigned_to: Option<String>,
   pub due_date:    Option<String>,
   pub started_at:  Option<String>,
   pub created_at:  String,
   pub workflow:    WorkflowSummaryDto,
}

/// ワークフロー概要 DTO（タスク一覧に含める最小限の情報）
#[derive(Debug, Serialize)]
pub struct WorkflowSummaryDto {
   pub id:           String,
   pub title:        String,
   pub status:       String,
   pub initiated_by: String,
   pub submitted_at: Option<String>,
}

impl From<WorkflowInstance> for WorkflowSummaryDto {
   fn from(instance: WorkflowInstance) -> Self {
      Self {
         id:           instance.id().to_string(),
         title:        instance.title().to_string(),
         status:       format!("{:?}", instance.status()),
         initiated_by: instance.initiated_by().to_string(),
         submitted_at: instance.submitted_at().map(|t| t.to_rfc3339()),
      }
   }
}

impl From<TaskItem> for TaskItemDto {
   fn from(item: TaskItem) -> Self {
      Self {
         id:          item.step.id().to_string(),
         step_name:   item.step.step_name().to_string(),
         status:      format!("{:?}", item.step.status()),
         version:     item.step.version().as_i32(),
         assigned_to: item.step.assigned_to().map(|u| u.to_string()),
         due_date:    item.step.due_date().map(|t| t.to_rfc3339()),
         started_at:  item.step.started_at().map(|t| t.to_rfc3339()),
         created_at:  item.step.created_at().to_rfc3339(),
         workflow:    WorkflowSummaryDto::from(item.workflow),
      }
   }
}

/// タスク詳細 DTO
#[derive(Debug, Serialize)]
pub struct TaskDetailDto {
   pub step:     WorkflowStepDto,
   pub workflow: WorkflowInstanceDto,
}

impl From<TaskDetail> for TaskDetailDto {
   fn from(detail: TaskDetail) -> Self {
      // ワークフローインスタンスをステップ付きで変換
      let workflow_dto = WorkflowInstanceDto::from(crate::usecase::workflow::WorkflowWithSteps {
         instance: detail.workflow,
         steps:    detail.steps,
      });

      Self {
         step:     WorkflowStepDto::from(detail.step),
         workflow: workflow_dto,
      }
   }
}

/// 自分のタスク一覧を取得する
///
/// ## エンドポイント
/// GET /internal/tasks/my?tenant_id={tenant_id}&user_id={user_id}
pub async fn list_my_tasks<I, S>(
   State(state): State<Arc<TaskState<I, S>>>,
   Query(query): Query<UserQuery>,
) -> Result<Response, CoreError>
where
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
{
   let tenant_id = TenantId::from_uuid(query.tenant_id);
   let user_id = UserId::from_uuid(query.user_id);

   let tasks = state.usecase.list_my_tasks(tenant_id, user_id).await?;

   let response = ApiResponse::new(tasks.into_iter().map(TaskItemDto::from).collect::<Vec<_>>());

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// タスク詳細を取得する
///
/// ## エンドポイント
/// GET /internal/tasks/{id}?tenant_id={tenant_id}&user_id={user_id}
pub async fn get_task<I, S>(
   State(state): State<Arc<TaskState<I, S>>>,
   Path(id): Path<Uuid>,
   Query(query): Query<UserQuery>,
) -> Result<Response, CoreError>
where
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
{
   let step_id = WorkflowStepId::from_uuid(id);
   let tenant_id = TenantId::from_uuid(query.tenant_id);
   let user_id = UserId::from_uuid(query.user_id);

   let detail = state.usecase.get_task(step_id, tenant_id, user_id).await?;

   let response = ApiResponse::new(TaskDetailDto::from(detail));

   Ok((StatusCode::OK, Json(response)).into_response())
}
