//! # ワークフロー API ハンドラ
//!
//! Core Service のワークフロー関連エンドポイントを実装する。

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, State},
   http::StatusCode,
   response::{IntoResponse, Response},
};
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   workflow::{WorkflowDefinitionId, WorkflowInstance, WorkflowInstanceId},
};
use ringiflow_infra::repository::{
   WorkflowDefinitionRepository,
   WorkflowInstanceRepository,
   WorkflowStepRepository,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
   error::CoreError,
   usecase::{CreateWorkflowInput, SubmitWorkflowInput, WorkflowUseCaseImpl},
};

/// ワークフロー作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
   /// ワークフロー定義 ID
   pub definition_id: Uuid,
   /// ワークフロータイトル
   pub title:         String,
   /// フォームデータ
   pub form_data:     serde_json::Value,
   /// テナント ID (内部 API 用)
   pub tenant_id:     Uuid,
   /// 申請者のユーザー ID (内部 API 用)
   pub user_id:       Uuid,
}

/// ワークフロー申請リクエスト
#[derive(Debug, Deserialize)]
pub struct SubmitWorkflowRequest {
   /// 承認者のユーザー ID
   pub assigned_to: Uuid,
   /// テナント ID (内部 API 用)
   pub tenant_id:   Uuid,
}

/// ワークフローレスポンス
#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
   pub data: WorkflowInstanceDto,
}

/// ワークフローインスタンス DTO
#[derive(Debug, Serialize)]
pub struct WorkflowInstanceDto {
   pub id: String,
   pub title: String,
   pub definition_id: String,
   pub status: String,
   pub form_data: serde_json::Value,
   pub initiated_by: String,
   pub current_step_id: Option<String>,
   pub submitted_at: Option<String>,
   pub created_at: String,
   pub updated_at: String,
}

impl From<WorkflowInstance> for WorkflowInstanceDto {
   fn from(instance: WorkflowInstance) -> Self {
      Self {
         id: instance.id().to_string(),
         title: instance.title().to_string(),
         definition_id: instance.definition_id().to_string(),
         status: format!("{:?}", instance.status()),
         form_data: instance.form_data().clone(),
         initiated_by: instance.initiated_by().to_string(),
         current_step_id: instance.current_step_id().map(|s| s.to_string()),
         submitted_at: instance.submitted_at().map(|t| t.to_rfc3339()),
         created_at: instance.created_at().to_rfc3339(),
         updated_at: instance.updated_at().to_rfc3339(),
      }
   }
}

/// ワークフローハンドラーの State
pub struct WorkflowState<D, I, S> {
   pub usecase: WorkflowUseCaseImpl<D, I, S>,
}

/// ワークフローを作成する（下書き）
///
/// ## エンドポイント
/// POST /internal/workflows
///
/// ## 処理フロー
/// 1. リクエストをパース
/// 2. ユースケースを呼び出し
/// 3. レスポンスを返す
pub async fn create_workflow<D, I, S>(
   State(state): State<Arc<WorkflowState<D, I, S>>>,
   Json(req): Json<CreateWorkflowRequest>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
{
   // ID を変換
   let tenant_id = TenantId::from_uuid(req.tenant_id);
   let user_id = UserId::from_uuid(req.user_id);
   let definition_id = WorkflowDefinitionId::from_uuid(req.definition_id);

   // ユースケースを呼び出し
   let input = CreateWorkflowInput {
      definition_id,
      title: req.title,
      form_data: req.form_data,
   };

   let instance = state
      .usecase
      .create_workflow(input, tenant_id, user_id)
      .await?;

   // レスポンスを返す
   let response = WorkflowResponse {
      data: WorkflowInstanceDto::from(instance),
   };

   Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// ワークフローを申請する
///
/// ## エンドポイント
/// POST /internal/workflows/{id}/submit
///
/// ## 処理フロー
/// 1. パスパラメータから ID を取得
/// 2. リクエストをパース
/// 3. ユースケースを呼び出し
/// 4. レスポンスを返す
pub async fn submit_workflow<D, I, S>(
   State(state): State<Arc<WorkflowState<D, I, S>>>,
   Path(id): Path<Uuid>,
   Json(req): Json<SubmitWorkflowRequest>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
{
   // ID を変換
   let instance_id = WorkflowInstanceId::from_uuid(id);
   let tenant_id = TenantId::from_uuid(req.tenant_id);
   let assigned_to = UserId::from_uuid(req.assigned_to);

   // ユースケースを呼び出し
   let input = SubmitWorkflowInput { assigned_to };

   let instance = state
      .usecase
      .submit_workflow(input, instance_id, tenant_id)
      .await?;

   // レスポンスを返す
   let response = WorkflowResponse {
      data: WorkflowInstanceDto::from(instance),
   };

   Ok((StatusCode::OK, Json(response)).into_response())
}
