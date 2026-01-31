//! # ワークフロー API ハンドラ
//!
//! Core Service のワークフロー関連エンドポイントを実装する。

use std::{
   collections::{HashMap, HashSet},
   sync::Arc,
};

use axum::{
   Json,
   extract::{Path, Query, State},
   http::StatusCode,
   response::{IntoResponse, Response},
};
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::Version,
   workflow::{
      WorkflowDefinition,
      WorkflowDefinitionId,
      WorkflowInstance,
      WorkflowInstanceId,
      WorkflowStep,
      WorkflowStepId,
   },
};
use ringiflow_infra::repository::{
   UserRepository,
   WorkflowDefinitionRepository,
   WorkflowInstanceRepository,
   WorkflowStepRepository,
};
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
   error::CoreError,
   usecase::{
      ApproveRejectInput,
      CreateWorkflowInput,
      SubmitWorkflowInput,
      WorkflowUseCaseImpl,
      WorkflowWithSteps,
   },
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

/// ステップ承認/却下リクエスト
#[derive(Debug, Deserialize)]
pub struct ApproveRejectRequest {
   /// 楽観的ロック用バージョン
   pub version:   i32,
   /// コメント（任意）
   pub comment:   Option<String>,
   /// テナント ID (内部 API 用)
   pub tenant_id: Uuid,
   /// 操作するユーザー ID (内部 API 用)
   pub user_id:   Uuid,
}

/// ステップパスパラメータ
#[derive(Debug, Deserialize)]
pub struct StepPathParams {
   /// ワークフローインスタンス ID
   /// 注: 現在の実装では step_id のみで検索するため未使用だが、
   /// 将来的に所属関係のバリデーションに使用する可能性あり
   #[allow(dead_code)]
   pub id:      Uuid,
   /// ステップ ID
   pub step_id: Uuid,
}

/// テナント指定クエリパラメータ（GET リクエスト用）
#[derive(Debug, Deserialize)]
pub struct TenantQuery {
   /// テナント ID
   pub tenant_id: Uuid,
}

/// ユーザー指定クエリパラメータ（GET リクエスト用）
#[derive(Debug, Deserialize)]
pub struct UserQuery {
   /// テナント ID
   pub tenant_id: Uuid,
   /// ユーザー ID
   pub user_id:   Uuid,
}

/// ユーザー参照 DTO
///
/// UUID 文字列の代わりに、ID とユーザー名をペアで返す。
/// フロントエンドでの表示用。
#[derive(Debug, Clone, Serialize)]
pub struct UserRefDto {
   pub id:   String,
   pub name: String,
}

/// ユーザー名マップからユーザー参照を作成する
///
/// ユーザーが見つからない場合は「（不明なユーザー）」にフォールバック。
pub(crate) fn to_user_ref(user_id: &UserId, user_names: &HashMap<UserId, String>) -> UserRefDto {
   let id = user_id.to_string();
   let name = user_names.get(user_id).cloned().unwrap_or_else(|| {
      tracing::warn!(user_id = %user_id, "User not found when resolving user name");
      "（不明なユーザー）".to_string()
   });
   UserRefDto { id, name }
}

/// ワークフロー定義 DTO
#[derive(Debug, Serialize)]
pub struct WorkflowDefinitionDto {
   pub id:          String,
   pub name:        String,
   pub description: Option<String>,
   pub version:     i32,
   pub definition:  serde_json::Value,
   pub status:      String,
   pub created_by:  String,
   pub created_at:  String,
   pub updated_at:  String,
}

impl From<WorkflowDefinition> for WorkflowDefinitionDto {
   fn from(def: WorkflowDefinition) -> Self {
      Self {
         id:          def.id().to_string(),
         name:        def.name().to_string(),
         description: def.description().map(|s| s.to_string()),
         version:     def.version().as_i32(),
         definition:  def.definition().clone(),
         status:      format!("{:?}", def.status()),
         created_by:  def.created_by().to_string(),
         created_at:  def.created_at().to_rfc3339(),
         updated_at:  def.updated_at().to_rfc3339(),
      }
   }
}

/// ワークフローステップ DTO
#[derive(Debug, Serialize)]
pub struct WorkflowStepDto {
   pub id:           String,
   pub step_id:      String,
   pub step_name:    String,
   pub step_type:    String,
   pub status:       String,
   pub version:      i32,
   pub assigned_to:  Option<UserRefDto>,
   pub decision:     Option<String>,
   pub comment:      Option<String>,
   pub due_date:     Option<String>,
   pub started_at:   Option<String>,
   pub completed_at: Option<String>,
   pub created_at:   String,
   pub updated_at:   String,
}

impl WorkflowStepDto {
   pub(crate) fn from_step(step: &WorkflowStep, user_names: &HashMap<UserId, String>) -> Self {
      Self {
         id:           step.id().to_string(),
         step_id:      step.step_id().to_string(),
         step_name:    step.step_name().to_string(),
         step_type:    step.step_type().to_string(),
         status:       format!("{:?}", step.status()),
         version:      step.version().as_i32(),
         assigned_to:  step.assigned_to().map(|u| to_user_ref(u, user_names)),
         decision:     step.decision().map(|d| format!("{:?}", d)),
         comment:      step.comment().map(|s| s.to_string()),
         due_date:     step.due_date().map(|t| t.to_rfc3339()),
         started_at:   step.started_at().map(|t| t.to_rfc3339()),
         completed_at: step.completed_at().map(|t| t.to_rfc3339()),
         created_at:   step.created_at().to_rfc3339(),
         updated_at:   step.updated_at().to_rfc3339(),
      }
   }
}

/// ワークフローインスタンス DTO
#[derive(Debug, Serialize)]
pub struct WorkflowInstanceDto {
   pub id: String,
   pub title: String,
   pub definition_id: String,
   pub status: String,
   pub version: i32,
   pub form_data: serde_json::Value,
   pub initiated_by: UserRefDto,
   pub current_step_id: Option<String>,
   pub steps: Vec<WorkflowStepDto>,
   pub submitted_at: Option<String>,
   pub completed_at: Option<String>,
   pub created_at: String,
   pub updated_at: String,
}

impl WorkflowInstanceDto {
   /// 一覧 API 用: ステップなしの変換
   fn from_instance(instance: &WorkflowInstance, user_names: &HashMap<UserId, String>) -> Self {
      Self {
         id: instance.id().to_string(),
         title: instance.title().to_string(),
         definition_id: instance.definition_id().to_string(),
         status: format!("{:?}", instance.status()),
         version: instance.version().as_i32(),
         form_data: instance.form_data().clone(),
         initiated_by: to_user_ref(instance.initiated_by(), user_names),
         current_step_id: instance.current_step_id().map(|s| s.to_string()),
         steps: Vec::new(),
         submitted_at: instance.submitted_at().map(|t| t.to_rfc3339()),
         completed_at: instance.completed_at().map(|t| t.to_rfc3339()),
         created_at: instance.created_at().to_rfc3339(),
         updated_at: instance.updated_at().to_rfc3339(),
      }
   }

   /// 詳細 API 用: ステップ付きの変換
   pub(crate) fn from_workflow_with_steps(
      data: &WorkflowWithSteps,
      user_names: &HashMap<UserId, String>,
   ) -> Self {
      let instance = &data.instance;
      Self {
         id: instance.id().to_string(),
         title: instance.title().to_string(),
         definition_id: instance.definition_id().to_string(),
         status: format!("{:?}", instance.status()),
         version: instance.version().as_i32(),
         form_data: instance.form_data().clone(),
         initiated_by: to_user_ref(instance.initiated_by(), user_names),
         current_step_id: instance.current_step_id().map(|s| s.to_string()),
         steps: data
            .steps
            .iter()
            .map(|s| WorkflowStepDto::from_step(s, user_names))
            .collect(),
         submitted_at: instance.submitted_at().map(|t| t.to_rfc3339()),
         completed_at: instance.completed_at().map(|t| t.to_rfc3339()),
         created_at: instance.created_at().to_rfc3339(),
         updated_at: instance.updated_at().to_rfc3339(),
      }
   }
}

/// ワークフローハンドラーの State
pub struct WorkflowState<D, I, S, U> {
   pub usecase: WorkflowUseCaseImpl<D, I, S, U>,
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
pub async fn create_workflow<D, I, S, U>(
   State(state): State<Arc<WorkflowState<D, I, S, U>>>,
   Json(req): Json<CreateWorkflowRequest>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
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
pub async fn submit_workflow<D, I, S, U>(
   State(state): State<Arc<WorkflowState<D, I, S, U>>>,
   Path(id): Path<Uuid>,
   Json(req): Json<SubmitWorkflowRequest>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
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

   // ユーザー名を解決
   let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(&instance, &[]);
   let user_names = state.usecase.resolve_user_names(&user_ids).await?;

   // レスポンスを返す
   let response = ApiResponse::new(WorkflowInstanceDto::from_instance(&instance, &user_names));

   Ok((StatusCode::OK, Json(response)).into_response())
}

// ===== GET ハンドラ =====

/// ワークフロー定義一覧を取得する
///
/// ## エンドポイント
/// GET /internal/workflow-definitions?tenant_id={tenant_id}
///
/// ## 処理フロー
/// 1. クエリパラメータからテナント ID を取得
/// 2. ユースケースを呼び出し
/// 3. レスポンスを返す
pub async fn list_workflow_definitions<D, I, S, U>(
   State(state): State<Arc<WorkflowState<D, I, S, U>>>,
   Query(query): Query<TenantQuery>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
{
   let tenant_id = TenantId::from_uuid(query.tenant_id);

   let definitions = state.usecase.list_workflow_definitions(tenant_id).await?;

   let response = ApiResponse::new(
      definitions
         .into_iter()
         .map(WorkflowDefinitionDto::from)
         .collect::<Vec<_>>(),
   );

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// ワークフロー定義の詳細を取得する
///
/// ## エンドポイント
/// GET /internal/workflow-definitions/{id}?tenant_id={tenant_id}
///
/// ## 処理フロー
/// 1. パスパラメータから ID を取得
/// 2. クエリパラメータからテナント ID を取得
/// 3. ユースケースを呼び出し
/// 4. レスポンスを返す
pub async fn get_workflow_definition<D, I, S, U>(
   State(state): State<Arc<WorkflowState<D, I, S, U>>>,
   Path(id): Path<Uuid>,
   Query(query): Query<TenantQuery>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
{
   let definition_id = WorkflowDefinitionId::from_uuid(id);
   let tenant_id = TenantId::from_uuid(query.tenant_id);

   let definition = state
      .usecase
      .get_workflow_definition(definition_id, tenant_id)
      .await?;

   let response = ApiResponse::new(WorkflowDefinitionDto::from(definition));

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// 自分のワークフロー一覧を取得する
///
/// ## エンドポイント
/// GET /internal/workflows?tenant_id={tenant_id}&user_id={user_id}
///
/// ## 処理フロー
/// 1. クエリパラメータからテナント ID とユーザー ID を取得
/// 2. ユースケースを呼び出し
/// 3. レスポンスを返す
pub async fn list_my_workflows<D, I, S, U>(
   State(state): State<Arc<WorkflowState<D, I, S, U>>>,
   Query(query): Query<UserQuery>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
{
   let tenant_id = TenantId::from_uuid(query.tenant_id);
   let user_id = UserId::from_uuid(query.user_id);

   let workflows = state.usecase.list_my_workflows(tenant_id, user_id).await?;

   // 全ワークフローの initiated_by を収集してユーザー名を一括解決
   let all_user_ids: Vec<UserId> = workflows
      .iter()
      .map(|w| w.initiated_by().clone())
      .collect::<HashSet<_>>()
      .into_iter()
      .collect();
   let user_names = state.usecase.resolve_user_names(&all_user_ids).await?;

   let response = ApiResponse::new(
      workflows
         .iter()
         .map(|w| WorkflowInstanceDto::from_instance(w, &user_names))
         .collect::<Vec<_>>(),
   );

   Ok((StatusCode::OK, Json(response)).into_response())
}

/// ワークフローの詳細を取得する
///
/// ## エンドポイント
/// GET /internal/workflows/{id}?tenant_id={tenant_id}
///
/// ## 処理フロー
/// 1. パスパラメータから ID を取得
/// 2. クエリパラメータからテナント ID を取得
/// 3. ユースケースを呼び出し
/// 4. レスポンスを返す
pub async fn get_workflow<D, I, S, U>(
   State(state): State<Arc<WorkflowState<D, I, S, U>>>,
   Path(id): Path<Uuid>,
   Query(query): Query<TenantQuery>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
{
   let instance_id = WorkflowInstanceId::from_uuid(id);
   let tenant_id = TenantId::from_uuid(query.tenant_id);

   let workflow_with_steps = state.usecase.get_workflow(instance_id, tenant_id).await?;

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

// ===== 承認/却下ハンドラ =====

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
pub async fn approve_step<D, I, S, U>(
   State(state): State<Arc<WorkflowState<D, I, S, U>>>,
   Path(params): Path<StepPathParams>,
   Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
{
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
pub async fn reject_step<D, I, S, U>(
   State(state): State<Arc<WorkflowState<D, I, S, U>>>,
   Path(params): Path<StepPathParams>,
   Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, CoreError>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
{
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
