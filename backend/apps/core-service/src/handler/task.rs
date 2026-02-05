//! # タスク API ハンドラ
//!
//! Core Service のタスク関連エンドポイントを実装する。

use std::{collections::HashMap, sync::Arc};

use axum::{
   Json,
   extract::{Path, Query, State},
   http::StatusCode,
   response::{IntoResponse, Response},
};
use itertools::Itertools;
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::{DisplayId, display_prefix},
   workflow::{WorkflowInstance, WorkflowStepId},
};
use ringiflow_infra::repository::{
   UserRepository,
   WorkflowInstanceRepository,
   WorkflowStepRepository,
};
use ringiflow_shared::ApiResponse;
use serde::Serialize;
use uuid::Uuid;

use crate::{
   error::CoreError,
   handler::workflow::{UserQuery, UserRefDto, WorkflowInstanceDto, WorkflowStepDto, to_user_ref},
   usecase::task::{TaskItem, TaskUseCaseImpl},
};

/// タスクハンドラーの State
pub struct TaskState<I, S, U> {
   pub usecase: TaskUseCaseImpl<I, S, U>,
}

/// タスク一覧の要素 DTO
#[derive(Debug, Serialize)]
pub struct TaskItemDto {
   pub id:          String,
   pub step_name:   String,
   pub status:      String,
   pub version:     i32,
   pub assigned_to: Option<UserRefDto>,
   pub due_date:    Option<String>,
   pub started_at:  Option<String>,
   pub created_at:  String,
   pub workflow:    WorkflowSummaryDto,
}

/// ワークフロー概要 DTO（タスク一覧に含める最小限の情報）
#[derive(Debug, Serialize)]
pub struct WorkflowSummaryDto {
   pub id:           String,
   pub display_id:   String,
   pub title:        String,
   pub status:       String,
   pub initiated_by: UserRefDto,
   pub submitted_at: Option<String>,
}

impl WorkflowSummaryDto {
   fn from_instance(instance: &WorkflowInstance, user_names: &HashMap<UserId, String>) -> Self {
      Self {
         id:           instance.id().to_string(),
         display_id:   DisplayId::new(display_prefix::WORKFLOW_INSTANCE, instance.display_number())
            .to_string(),
         title:        instance.title().to_string(),
         status:       format!("{:?}", instance.status()),
         initiated_by: to_user_ref(instance.initiated_by(), user_names),
         submitted_at: instance.submitted_at().map(|t| t.to_rfc3339()),
      }
   }
}

impl TaskItemDto {
   fn from_task_item(item: &TaskItem, user_names: &HashMap<UserId, String>) -> Self {
      Self {
         id:          item.step.id().to_string(),
         step_name:   item.step.step_name().to_string(),
         status:      format!("{:?}", item.step.status()),
         version:     item.step.version().as_i32(),
         assigned_to: item.step.assigned_to().map(|u| to_user_ref(u, user_names)),
         due_date:    item.step.due_date().map(|t| t.to_rfc3339()),
         started_at:  item.step.started_at().map(|t| t.to_rfc3339()),
         created_at:  item.step.created_at().to_rfc3339(),
         workflow:    WorkflowSummaryDto::from_instance(&item.workflow, user_names),
      }
   }
}

/// タスク詳細 DTO
#[derive(Debug, Serialize)]
pub struct TaskDetailDto {
   pub step:     WorkflowStepDto,
   pub workflow: WorkflowInstanceDto,
}

/// 自分のタスク一覧を取得する
///
/// ## エンドポイント
/// GET /internal/tasks/my?tenant_id={tenant_id}&user_id={user_id}
pub async fn list_my_tasks<I, S, U>(
   State(state): State<Arc<TaskState<I, S, U>>>,
   Query(query): Query<UserQuery>,
) -> Result<Response, CoreError>
where
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
{
   let tenant_id = TenantId::from_uuid(query.tenant_id);
   let user_id = UserId::from_uuid(query.user_id);

   let tasks = state.usecase.list_my_tasks(tenant_id, user_id).await?;

   // 全タスクのユーザー ID を収集して一括解決
   let all_user_ids: Vec<UserId> = tasks
      .iter()
      .flat_map(|task| {
         std::iter::once(task.workflow.initiated_by().clone())
            .chain(task.step.assigned_to().cloned())
      })
      .unique()
      .collect();
   let user_names = state.usecase.resolve_user_names(&all_user_ids).await?;

   let response = ApiResponse::new(
      tasks
         .iter()
         .map(|t| TaskItemDto::from_task_item(t, &user_names))
         .collect::<Vec<_>>(),
   );

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// タスク詳細を取得する
///
/// ## エンドポイント
/// GET /internal/tasks/{id}?tenant_id={tenant_id}&user_id={user_id}
pub async fn get_task<I, S, U>(
   State(state): State<Arc<TaskState<I, S, U>>>,
   Path(id): Path<Uuid>,
   Query(query): Query<UserQuery>,
) -> Result<Response, CoreError>
where
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
{
   let step_id = WorkflowStepId::from_uuid(id);
   let tenant_id = TenantId::from_uuid(query.tenant_id);
   let user_id = UserId::from_uuid(query.user_id);

   let detail = state.usecase.get_task(step_id, tenant_id, user_id).await?;

   // ユーザー名を解決
   let user_ids =
      crate::usecase::workflow::collect_user_ids_from_workflow(&detail.workflow, &detail.steps);
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   let response = ApiResponse::new(TaskDetailDto {
      step:     WorkflowStepDto::from_step(&detail.step, &user_names),
      workflow: WorkflowInstanceDto::from_workflow_with_steps(
         &crate::usecase::workflow::WorkflowWithSteps {
            instance: detail.workflow,
            steps:    detail.steps,
         },
         &user_names,
      ),
   });

   Ok((StatusCode::OK, Json(response)).into_response())
}
