//! # ワークフロー API ハンドラ
//!
//! BFF のワークフロー関連エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! ### ワークフロー定義
//! - `GET /api/v1/workflow-definitions` - ワークフロー定義一覧
//! - `GET /api/v1/workflow-definitions/{id}` - ワークフロー定義詳細
//!
//! ### ワークフローインスタンス
//! - `GET /api/v1/workflows` - 自分の申請一覧
//! - `GET /api/v1/workflows/{id}` - ワークフロー詳細
//! - `POST /api/v1/workflows` - ワークフローを作成（下書き）
//! - `POST /api/v1/workflows/{id}/submit` - ワークフローを申請
//!
//! ## BFF の責務
//!
//! 1. セッションから `tenant_id`, `user_id` を取得
//! 2. Core Service の内部 API を呼び出し
//! 3. レスポンスをクライアントに返す

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, State},
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_infra::SessionManager;
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
   client::{CoreServiceClient, CoreServiceError},
   error::{
      conflict_response,
      extract_tenant_id,
      forbidden_response,
      get_session,
      internal_error_response,
      not_found_response,
      validation_error_response,
   },
};

/// ワークフローハンドラの共有状態
pub struct WorkflowState<C, S>
where
   C: CoreServiceClient,
   S: SessionManager,
{
   pub core_service_client: C,
   pub session_manager:     S,
}

// --- リクエスト/レスポンス型 ---

/// ワークフロー作成リクエスト（BFF 公開 API）
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
   /// ワークフロー定義 ID
   pub definition_id: Uuid,
   /// ワークフロータイトル
   pub title:         String,
   /// フォームデータ
   pub form_data:     serde_json::Value,
}

/// ワークフロー申請リクエスト（BFF 公開 API）
#[derive(Debug, Deserialize)]
pub struct SubmitWorkflowRequest {
   /// 承認者のユーザー ID
   pub assigned_to: Uuid,
}

/// ステップ承認/却下リクエスト（BFF 公開 API）
#[derive(Debug, Deserialize)]
pub struct ApproveRejectRequest {
   /// 楽観的ロック用バージョン
   pub version: i32,
   /// コメント（任意）
   pub comment: Option<String>,
}

/// ステップパスパラメータ（display_number 用）
#[derive(Debug, Deserialize)]
pub struct StepPathParams {
   /// ワークフローの表示用連番
   pub display_number:      i64,
   /// ステップの表示用連番
   pub step_display_number: i64,
}

/// ユーザー参照データ（フロントエンドへの Serialize 用）
#[derive(Debug, Serialize)]
pub struct UserRefData {
   pub id:   String,
   pub name: String,
}

impl From<crate::client::UserRefDto> for UserRefData {
   fn from(dto: crate::client::UserRefDto) -> Self {
      Self {
         id:   dto.id,
         name: dto.name,
      }
   }
}

/// ワークフローステップデータ
#[derive(Debug, Serialize)]
pub struct WorkflowStepData {
   pub id: String,
   pub display_id: String,
   pub display_number: i64,
   pub step_id: String,
   pub step_name: String,
   pub step_type: String,
   pub status: String,
   pub version: i32,
   pub assigned_to: Option<UserRefData>,
   pub decision: Option<String>,
   pub comment: Option<String>,
   pub due_date: Option<String>,
   pub started_at: Option<String>,
   pub completed_at: Option<String>,
   pub created_at: String,
   pub updated_at: String,
}

impl From<crate::client::WorkflowStepDto> for WorkflowStepData {
   fn from(dto: crate::client::WorkflowStepDto) -> Self {
      Self {
         id: dto.id,
         display_id: dto.display_id,
         display_number: dto.display_number,
         step_id: dto.step_id,
         step_name: dto.step_name,
         step_type: dto.step_type,
         status: dto.status,
         version: dto.version,
         assigned_to: dto.assigned_to.map(UserRefData::from),
         decision: dto.decision,
         comment: dto.comment,
         due_date: dto.due_date,
         started_at: dto.started_at,
         completed_at: dto.completed_at,
         created_at: dto.created_at,
         updated_at: dto.updated_at,
      }
   }
}

/// ワークフローデータ
#[derive(Debug, Serialize)]
pub struct WorkflowData {
   pub id: String,
   pub display_id: String,
   pub display_number: i64,
   pub title: String,
   pub definition_id: String,
   pub status: String,
   pub version: i32,
   pub form_data: serde_json::Value,
   pub initiated_by: UserRefData,
   pub current_step_id: Option<String>,
   pub steps: Vec<WorkflowStepData>,
   pub submitted_at: Option<String>,
   pub completed_at: Option<String>,
   pub created_at: String,
   pub updated_at: String,
}

impl From<crate::client::WorkflowInstanceDto> for WorkflowData {
   fn from(dto: crate::client::WorkflowInstanceDto) -> Self {
      Self {
         id: dto.id,
         display_id: dto.display_id,
         display_number: dto.display_number,
         title: dto.title,
         definition_id: dto.definition_id,
         status: dto.status,
         version: dto.version,
         form_data: dto.form_data,
         initiated_by: UserRefData::from(dto.initiated_by),
         current_step_id: dto.current_step_id,
         steps: dto.steps.into_iter().map(WorkflowStepData::from).collect(),
         submitted_at: dto.submitted_at,
         completed_at: dto.completed_at,
         created_at: dto.created_at,
         updated_at: dto.updated_at,
      }
   }
}

/// ワークフロー定義データ
#[derive(Debug, Serialize)]
pub struct WorkflowDefinitionData {
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

impl From<crate::client::WorkflowDefinitionDto> for WorkflowDefinitionData {
   fn from(dto: crate::client::WorkflowDefinitionDto) -> Self {
      Self {
         id:          dto.id,
         name:        dto.name,
         description: dto.description,
         version:     dto.version,
         definition:  dto.definition,
         status:      dto.status,
         created_by:  dto.created_by,
         created_at:  dto.created_at,
         updated_at:  dto.updated_at,
      }
   }
}

// --- ハンドラ ---

/// POST /api/v1/workflows
///
/// ワークフローを作成する（下書き）
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows` を呼び出し
/// 3. レスポンスを返す
pub async fn create_workflow<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Json(req): Json<CreateWorkflowRequest>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // Core Service を呼び出し
   let core_req = crate::client::CreateWorkflowRequest {
      definition_id: req.definition_id,
      title:         req.title,
      form_data:     req.form_data,
      tenant_id:     *session_data.tenant_id().as_uuid(),
      user_id:       *session_data.user_id().as_uuid(),
   };

   match state.core_service_client.create_workflow(core_req).await {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowData::from(core_response.data));
         (StatusCode::CREATED, Json(response)).into_response()
      }
      Err(CoreServiceError::WorkflowDefinitionNotFound) => not_found_response(
         "workflow-definition-not-found",
         "Workflow Definition Not Found",
         "ワークフロー定義が見つかりません",
      ),
      Err(CoreServiceError::ValidationError(detail)) => validation_error_response(&detail),
      Err(e) => {
         tracing::error!("ワークフロー作成で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// POST /api/v1/workflows/{display_number}/submit
///
/// ワークフローを申請する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{display_number}/submit` を呼び出し
/// 3. レスポンスを返す
pub async fn submit_workflow<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(display_number): Path<i64>,
   Json(req): Json<SubmitWorkflowRequest>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
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
   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // Core Service を呼び出し
   let core_req = crate::client::SubmitWorkflowRequest {
      assigned_to: req.assigned_to,
      tenant_id:   *session_data.tenant_id().as_uuid(),
   };

   match state
      .core_service_client
      .submit_workflow_by_display_number(display_number, core_req)
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
      Err(CoreServiceError::ValidationError(detail)) => validation_error_response(&detail),
      Err(e) => {
         tracing::error!("ワークフロー申請で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

// ===== GET ハンドラ =====

/// GET /api/v1/workflow-definitions
///
/// ワークフロー定義一覧を取得する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `GET /internal/workflow-definitions` を呼び出し
/// 3. レスポンスを返す
pub async fn list_workflow_definitions<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
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
pub async fn get_workflow_definition<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(definition_id): Path<Uuid>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
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
pub async fn list_my_workflows<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
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
pub async fn get_workflow<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(display_number): Path<i64>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
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
   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
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

// ===== 承認/却下ハンドラ =====

/// POST /api/v1/workflows/{display_number}/steps/{step_display_number}/approve
///
/// ワークフローステップを承認する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/approve` を呼び出し
/// 3. 200 OK + 更新されたワークフローを返す
pub async fn approve_step<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(params): Path<StepPathParams>,
   Json(req): Json<ApproveRejectRequest>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
   // display_number の検証
   if params.display_number <= 0 {
      return validation_error_response("display_number は 1 以上である必要があります");
   }
   if params.step_display_number <= 0 {
      return validation_error_response("step_display_number は 1 以上である必要があります");
   }

   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // Core Service を呼び出し
   let core_req = crate::client::ApproveRejectRequest {
      version:   req.version,
      comment:   req.comment,
      tenant_id: *session_data.tenant_id().as_uuid(),
      user_id:   *session_data.user_id().as_uuid(),
   };

   match state
      .core_service_client
      .approve_step_by_display_number(params.display_number, params.step_display_number, core_req)
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowData::from(core_response.data));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::StepNotFound) => not_found_response(
         "step-not-found",
         "Step Not Found",
         "ステップが見つかりません",
      ),
      Err(CoreServiceError::WorkflowInstanceNotFound) => not_found_response(
         "workflow-instance-not-found",
         "Workflow Instance Not Found",
         "ワークフローインスタンスが見つかりません",
      ),
      Err(CoreServiceError::ValidationError(detail)) => validation_error_response(&detail),
      Err(CoreServiceError::Forbidden(detail)) => forbidden_response(&detail),
      Err(CoreServiceError::Conflict(detail)) => conflict_response(&detail),
      Err(e) => {
         tracing::error!("ステップ承認で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// POST /api/v1/workflows/{display_number}/steps/{step_display_number}/reject
///
/// ワークフローステップを却下する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/reject` を呼び出し
/// 3. 200 OK + 更新されたワークフローを返す
pub async fn reject_step<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(params): Path<StepPathParams>,
   Json(req): Json<ApproveRejectRequest>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
   // display_number の検証
   if params.display_number <= 0 {
      return validation_error_response("display_number は 1 以上である必要があります");
   }
   if params.step_display_number <= 0 {
      return validation_error_response("step_display_number は 1 以上である必要があります");
   }

   // X-Tenant-ID ヘッダーからテナント ID を取得
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   // セッションを取得
   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // Core Service を呼び出し
   let core_req = crate::client::ApproveRejectRequest {
      version:   req.version,
      comment:   req.comment,
      tenant_id: *session_data.tenant_id().as_uuid(),
      user_id:   *session_data.user_id().as_uuid(),
   };

   match state
      .core_service_client
      .reject_step_by_display_number(params.display_number, params.step_display_number, core_req)
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowData::from(core_response.data));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::StepNotFound) => not_found_response(
         "step-not-found",
         "Step Not Found",
         "ステップが見つかりません",
      ),
      Err(CoreServiceError::WorkflowInstanceNotFound) => not_found_response(
         "workflow-instance-not-found",
         "Workflow Instance Not Found",
         "ワークフローインスタンスが見つかりません",
      ),
      Err(CoreServiceError::ValidationError(detail)) => validation_error_response(&detail),
      Err(CoreServiceError::Forbidden(detail)) => forbidden_response(&detail),
      Err(CoreServiceError::Conflict(detail)) => conflict_response(&detail),
      Err(e) => {
         tracing::error!("ステップ却下で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

// ===== タスクハンドラ =====

/// GET /api/v1/workflows/{display_number}/tasks/{step_display_number}
///
/// display_number でタスク詳細を取得する
pub async fn get_task_by_display_numbers<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(params): Path<StepPathParams>,
) -> impl IntoResponse
where
   C: CoreServiceClient,
   S: SessionManager,
{
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

   let session_data = match get_session(&state.session_manager, &jar, tenant_id).await {
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
         let response = ApiResponse::new(super::task::TaskDetailData::from(core_response.data));
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
