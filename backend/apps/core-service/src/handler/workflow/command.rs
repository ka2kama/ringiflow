//! ワークフローハンドラの状態変更操作

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
   value_objects::{DisplayNumber, Version},
   workflow::{WorkflowDefinitionId, WorkflowInstanceId, WorkflowStepId},
};
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
   ApproveRejectRequest,
   CreateWorkflowRequest,
   PostCommentRequest,
   StepByDisplayNumberPathParams,
   StepPathParams,
   SubmitWorkflowRequest,
   WorkflowCommentDto,
   WorkflowInstanceDto,
   WorkflowState,
};
use crate::{
   error::CoreError,
   usecase::{
      ApproveRejectInput,
      CreateWorkflowInput,
      PostCommentInput,
      StepApprover,
      SubmitWorkflowInput,
   },
};

/// ワークフローを作成する（下書き）
///
/// ## エンドポイント
/// POST /internal/workflows
///
/// ## 処理フロー
/// 1. リクエストをパース
/// 2. ユースケースを呼び出し
/// 3. レスポンスを返す
pub async fn create_workflow(
   State(state): State<Arc<WorkflowState>>,
   Json(req): Json<CreateWorkflowRequest>,
) -> Result<Response, CoreError> {
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

   // ユーザー名を解決
   let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(&instance, &[]);
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   // レスポンスを返す
   let response = ApiResponse::new(WorkflowInstanceDto::from_instance(&instance, &user_names));

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
pub async fn submit_workflow(
   State(state): State<Arc<WorkflowState>>,
   Path(id): Path<Uuid>,
   Json(req): Json<SubmitWorkflowRequest>,
) -> Result<Response, CoreError> {
   // ID を変換
   let instance_id = WorkflowInstanceId::from_uuid(id);
   let tenant_id = TenantId::from_uuid(req.tenant_id);

   // ユースケースを呼び出し
   let input = SubmitWorkflowInput {
      approvers: req
         .approvers
         .into_iter()
         .map(|a| StepApprover {
            step_id:     a.step_id,
            assigned_to: UserId::from_uuid(a.assigned_to),
         })
         .collect(),
   };

   let instance = state
      .usecase
      .submit_workflow(input, instance_id, tenant_id)
      .await?;

   // ユーザー名を解決
   let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(&instance, &[]);
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   // レスポンスを返す
   let response = ApiResponse::new(WorkflowInstanceDto::from_instance(&instance, &user_names));

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// ワークフローステップを承認する
///
/// ## エンドポイント
/// POST /internal/workflows/{id}/steps/{step_id}/approve
///
/// ## 処理フロー
/// 1. パスパラメータから ID を取得
/// 2. リクエストをパース
/// 3. ユースケースを呼び出し
/// 4. 200 OK + 更新されたワークフローを返す
pub async fn approve_step(
   State(state): State<Arc<WorkflowState>>,
   Path(params): Path<StepPathParams>,
   Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, CoreError> {
   let step_id = WorkflowStepId::from_uuid(params.step_id);
   let tenant_id = TenantId::from_uuid(req.tenant_id);
   let user_id = UserId::from_uuid(req.user_id);
   let version = Version::try_from(req.version)
      .map_err(|e| CoreError::BadRequest(format!("不正なバージョン: {}", e)))?;

   let input = ApproveRejectInput {
      version,
      comment: req.comment,
   };

   let workflow_with_steps = state
      .usecase
      .approve_step(input, step_id, tenant_id, user_id)
      .await?;

   // ユーザー名を解決
   let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(
      &workflow_with_steps.instance,
      &workflow_with_steps.steps,
   );
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   let response = ApiResponse::new(WorkflowInstanceDto::from_workflow_with_steps(
      &workflow_with_steps,
      &user_names,
   ));

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// ワークフローステップを却下する
///
/// ## エンドポイント
/// POST /internal/workflows/{id}/steps/{step_id}/reject
///
/// ## 処理フロー
/// 1. パスパラメータから ID を取得
/// 2. リクエストをパース
/// 3. ユースケースを呼び出し
/// 4. 200 OK + 更新されたワークフローを返す
pub async fn reject_step(
   State(state): State<Arc<WorkflowState>>,
   Path(params): Path<StepPathParams>,
   Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, CoreError> {
   let step_id = WorkflowStepId::from_uuid(params.step_id);
   let tenant_id = TenantId::from_uuid(req.tenant_id);
   let user_id = UserId::from_uuid(req.user_id);
   let version = Version::try_from(req.version)
      .map_err(|e| CoreError::BadRequest(format!("不正なバージョン: {}", e)))?;

   let input = ApproveRejectInput {
      version,
      comment: req.comment,
   };

   let workflow_with_steps = state
      .usecase
      .reject_step(input, step_id, tenant_id, user_id)
      .await?;

   // ユーザー名を解決
   let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(
      &workflow_with_steps.instance,
      &workflow_with_steps.steps,
   );
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   let response = ApiResponse::new(WorkflowInstanceDto::from_workflow_with_steps(
      &workflow_with_steps,
      &user_names,
   ));

   Ok((StatusCode::OK, Json(response)).into_response())
}

// ===== display_number 対応ハンドラ =====

/// display_number でワークフローを申請する
///
/// ## エンドポイント
/// POST /internal/workflows/by-display-number/{display_number}/submit
///
/// ## 処理フロー
/// 1. パスパラメータから display_number を取得
/// 2. リクエストをパース
/// 3. ユースケースを呼び出し
/// 4. 200 OK + 申請後のワークフローを返す
pub async fn submit_workflow_by_display_number(
   State(state): State<Arc<WorkflowState>>,
   Path(display_number): Path<i64>,
   Json(req): Json<SubmitWorkflowRequest>,
) -> Result<Response, CoreError> {
   let display_number = DisplayNumber::try_from(display_number)
      .map_err(|e| CoreError::BadRequest(format!("不正な display_number: {}", e)))?;
   let tenant_id = TenantId::from_uuid(req.tenant_id);

   let input = SubmitWorkflowInput {
      approvers: req
         .approvers
         .into_iter()
         .map(|a| StepApprover {
            step_id:     a.step_id,
            assigned_to: UserId::from_uuid(a.assigned_to),
         })
         .collect(),
   };

   let instance = state
      .usecase
      .submit_workflow_by_display_number(input, display_number, tenant_id)
      .await?;

   // ユーザー名を解決
   let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(&instance, &[]);
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   let response = ApiResponse::new(WorkflowInstanceDto::from_instance(&instance, &user_names));

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// display_number でワークフローステップを承認する
///
/// ## エンドポイント
/// POST /internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/approve
///
/// ## 処理フロー
/// 1. パスパラメータから display_number を取得
/// 2. リクエストをパース
/// 3. ユースケースを呼び出し
/// 4. 200 OK + 更新されたワークフローを返す
pub async fn approve_step_by_display_number(
   State(state): State<Arc<WorkflowState>>,
   Path(params): Path<StepByDisplayNumberPathParams>,
   Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, CoreError> {
   let workflow_display_number = DisplayNumber::try_from(params.display_number)
      .map_err(|e| CoreError::BadRequest(format!("不正な display_number: {}", e)))?;
   let step_display_number = DisplayNumber::try_from(params.step_display_number)
      .map_err(|e| CoreError::BadRequest(format!("不正な step_display_number: {}", e)))?;
   let tenant_id = TenantId::from_uuid(req.tenant_id);
   let user_id = UserId::from_uuid(req.user_id);
   let version = Version::try_from(req.version)
      .map_err(|e| CoreError::BadRequest(format!("不正なバージョン: {}", e)))?;

   let input = ApproveRejectInput {
      version,
      comment: req.comment,
   };

   let workflow_with_steps = state
      .usecase
      .approve_step_by_display_number(
         input,
         workflow_display_number,
         step_display_number,
         tenant_id,
         user_id,
      )
      .await?;

   // ユーザー名を解決
   let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(
      &workflow_with_steps.instance,
      &workflow_with_steps.steps,
   );
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   let response = ApiResponse::new(WorkflowInstanceDto::from_workflow_with_steps(
      &workflow_with_steps,
      &user_names,
   ));

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// display_number でワークフローステップを却下する
///
/// ## エンドポイント
/// POST /internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/reject
///
/// ## 処理フロー
/// 1. パスパラメータから display_number を取得
/// 2. リクエストをパース
/// 3. ユースケースを呼び出し
/// 4. 200 OK + 更新されたワークフローを返す
pub async fn reject_step_by_display_number(
   State(state): State<Arc<WorkflowState>>,
   Path(params): Path<StepByDisplayNumberPathParams>,
   Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, CoreError> {
   let workflow_display_number = DisplayNumber::try_from(params.display_number)
      .map_err(|e| CoreError::BadRequest(format!("不正な display_number: {}", e)))?;
   let step_display_number = DisplayNumber::try_from(params.step_display_number)
      .map_err(|e| CoreError::BadRequest(format!("不正な step_display_number: {}", e)))?;
   let tenant_id = TenantId::from_uuid(req.tenant_id);
   let user_id = UserId::from_uuid(req.user_id);
   let version = Version::try_from(req.version)
      .map_err(|e| CoreError::BadRequest(format!("不正なバージョン: {}", e)))?;

   let input = ApproveRejectInput {
      version,
      comment: req.comment,
   };

   let workflow_with_steps = state
      .usecase
      .reject_step_by_display_number(
         input,
         workflow_display_number,
         step_display_number,
         tenant_id,
         user_id,
      )
      .await?;

   // ユーザー名を解決
   let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(
      &workflow_with_steps.instance,
      &workflow_with_steps.steps,
   );
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   let response = ApiResponse::new(WorkflowInstanceDto::from_workflow_with_steps(
      &workflow_with_steps,
      &user_names,
   ));

   Ok((StatusCode::OK, Json(response)).into_response())
}

// ===== コメントハンドラ =====

/// ワークフローにコメントを投稿する
///
/// ## エンドポイント
/// POST /internal/workflows/by-display-number/{display_number}/comments
///
/// ## 処理フロー
/// 1. パスパラメータから display_number を取得
/// 2. リクエストをパース
/// 3. ユースケースを呼び出し
/// 4. 201 Created + コメントを返す
pub async fn post_comment(
   State(state): State<Arc<WorkflowState>>,
   Path(display_number): Path<i64>,
   Json(req): Json<PostCommentRequest>,
) -> Result<Response, CoreError> {
   let display_number = DisplayNumber::try_from(display_number)
      .map_err(|e| CoreError::BadRequest(format!("不正な display_number: {}", e)))?;
   let tenant_id = TenantId::from_uuid(req.tenant_id);
   let user_id = UserId::from_uuid(req.user_id);

   let input = PostCommentInput { body: req.body };

   let comment = state
      .usecase
      .post_comment(input, display_number, tenant_id, user_id)
      .await?;

   // ユーザー名を解決
   let user_ids = vec![comment.posted_by().clone()];
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   let response = ApiResponse::new(WorkflowCommentDto::from_comment(&comment, &user_names));

   Ok((StatusCode::CREATED, Json(response)).into_response())
}
