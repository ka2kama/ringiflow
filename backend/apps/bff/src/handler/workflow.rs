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
   response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::SessionManager;
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::client::{CoreServiceClient, CoreServiceError};

/// Cookie 名
const SESSION_COOKIE_NAME: &str = "session_id";

// --- エラー型 ---

/// テナント ID 抽出エラー
#[derive(Debug)]
pub enum TenantIdError {
   /// ヘッダーが存在しない
   Missing,
   /// UUID の形式が不正
   InvalidFormat,
}

impl IntoResponse for TenantIdError {
   fn into_response(self) -> Response {
      let (status, detail) = match self {
         TenantIdError::Missing => (StatusCode::BAD_REQUEST, "X-Tenant-ID ヘッダーが必要です"),
         TenantIdError::InvalidFormat => (StatusCode::BAD_REQUEST, "X-Tenant-ID の形式が不正です"),
      };
      (
         status,
         Json(ErrorResponse {
            error_type: "https://ringiflow.example.com/errors/validation-error".to_string(),
            title:      "Validation Error".to_string(),
            status:     status.as_u16(),
            detail:     detail.to_string(),
         }),
      )
         .into_response()
   }
}

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

/// ステップパスパラメータ
#[derive(Debug, Deserialize)]
pub struct StepPathParams {
   /// ワークフローインスタンス ID
   pub id:      Uuid,
   /// ステップ ID
   pub step_id: Uuid,
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
   pub id:           String,
   pub display_id:   String,
   pub step_id:      String,
   pub step_name:    String,
   pub step_type:    String,
   pub status:       String,
   pub version:      i32,
   pub assigned_to:  Option<UserRefData>,
   pub decision:     Option<String>,
   pub comment:      Option<String>,
   pub due_date:     Option<String>,
   pub started_at:   Option<String>,
   pub completed_at: Option<String>,
   pub created_at:   String,
   pub updated_at:   String,
}

impl From<crate::client::WorkflowStepDto> for WorkflowStepData {
   fn from(dto: crate::client::WorkflowStepDto) -> Self {
      Self {
         id:           dto.id,
         display_id:   dto.display_id,
         step_id:      dto.step_id,
         step_name:    dto.step_name,
         step_type:    dto.step_type,
         status:       dto.status,
         version:      dto.version,
         assigned_to:  dto.assigned_to.map(UserRefData::from),
         decision:     dto.decision,
         comment:      dto.comment,
         due_date:     dto.due_date,
         started_at:   dto.started_at,
         completed_at: dto.completed_at,
         created_at:   dto.created_at,
         updated_at:   dto.updated_at,
      }
   }
}

/// ワークフローデータ
#[derive(Debug, Serialize)]
pub struct WorkflowData {
   pub id: String,
   pub display_id: String,
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

/// エラーレスポンス（RFC 7807 Problem Details）
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
   #[serde(rename = "type")]
   pub error_type: String,
   pub title:      String,
   pub status:     u16,
   pub detail:     String,
}

impl From<crate::client::WorkflowInstanceDto> for WorkflowData {
   fn from(dto: crate::client::WorkflowInstanceDto) -> Self {
      Self {
         id: dto.id,
         display_id: dto.display_id,
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
         "https://ringiflow.example.com/errors/workflow-definition-not-found",
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

/// POST /api/v1/workflows/{id}/submit
///
/// ワークフローを申請する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `POST /internal/workflows/{id}/submit` を呼び出し
/// 3. レスポンスを返す
pub async fn submit_workflow<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(workflow_id): Path<Uuid>,
   Json(req): Json<SubmitWorkflowRequest>,
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
   let core_req = crate::client::SubmitWorkflowRequest {
      assigned_to: req.assigned_to,
      tenant_id:   *session_data.tenant_id().as_uuid(),
   };

   match state
      .core_service_client
      .submit_workflow(workflow_id, core_req)
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowData::from(core_response.data));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::WorkflowInstanceNotFound) => not_found_response(
         "https://ringiflow.example.com/errors/workflow-instance-not-found",
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

// --- ヘルパー関数 ---

/// X-Tenant-ID ヘッダーからテナント ID を抽出する
pub(crate) fn extract_tenant_id(headers: &HeaderMap) -> Result<Uuid, TenantIdError> {
   let tenant_id_str = headers
      .get("X-Tenant-ID")
      .and_then(|v| v.to_str().ok())
      .ok_or(TenantIdError::Missing)?;

   Uuid::parse_str(tenant_id_str).map_err(|_| TenantIdError::InvalidFormat)
}

/// セッションを取得する
pub(crate) async fn get_session<S>(
   session_manager: &S,
   jar: &CookieJar,
   tenant_id: Uuid,
) -> Result<ringiflow_infra::SessionData, Response>
where
   S: SessionManager,
{
   // Cookie からセッション ID を取得
   let session_id = jar
      .get(SESSION_COOKIE_NAME)
      .map(|cookie| cookie.value().to_string())
      .ok_or_else(unauthorized_response)?;

   let tenant_id = TenantId::from_uuid(tenant_id);

   // セッションを取得
   match session_manager.get(&tenant_id, &session_id).await {
      Ok(Some(data)) => Ok(data),
      Ok(None) => Err(unauthorized_response()),
      Err(e) => {
         tracing::error!("セッション取得で内部エラー: {}", e);
         Err(internal_error_response())
      }
   }
}

/// 未認証レスポンス
fn unauthorized_response() -> Response {
   (
      StatusCode::UNAUTHORIZED,
      Json(ErrorResponse {
         error_type: "https://ringiflow.example.com/errors/unauthorized".to_string(),
         title:      "Unauthorized".to_string(),
         status:     401,
         detail:     "認証が必要です".to_string(),
      }),
   )
      .into_response()
}

/// 内部エラーレスポンス
pub(crate) fn internal_error_response() -> Response {
   (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(ErrorResponse {
         error_type: "https://ringiflow.example.com/errors/internal-error".to_string(),
         title:      "Internal Server Error".to_string(),
         status:     500,
         detail:     "内部エラーが発生しました".to_string(),
      }),
   )
      .into_response()
}

/// 404 Not Found レスポンス
pub(crate) fn not_found_response(error_type: &str, title: &str, detail: &str) -> Response {
   (
      StatusCode::NOT_FOUND,
      Json(ErrorResponse {
         error_type: error_type.to_string(),
         title:      title.to_string(),
         status:     404,
         detail:     detail.to_string(),
      }),
   )
      .into_response()
}

/// バリデーションエラーレスポンス
fn validation_error_response(detail: &str) -> Response {
   (
      StatusCode::BAD_REQUEST,
      Json(ErrorResponse {
         error_type: "https://ringiflow.example.com/errors/validation-error".to_string(),
         title:      "Validation Error".to_string(),
         status:     400,
         detail:     detail.to_string(),
      }),
   )
      .into_response()
}

/// 403 Forbidden レスポンス
pub(crate) fn forbidden_response(detail: &str) -> Response {
   (
      StatusCode::FORBIDDEN,
      Json(ErrorResponse {
         error_type: "https://ringiflow.example.com/errors/forbidden".to_string(),
         title:      "Forbidden".to_string(),
         status:     403,
         detail:     detail.to_string(),
      }),
   )
      .into_response()
}

/// 409 Conflict レスポンス
fn conflict_response(detail: &str) -> Response {
   (
      StatusCode::CONFLICT,
      Json(ErrorResponse {
         error_type: "https://ringiflow.example.com/errors/conflict".to_string(),
         title:      "Conflict".to_string(),
         status:     409,
         detail:     detail.to_string(),
      }),
   )
      .into_response()
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
         "https://ringiflow.example.com/errors/workflow-definition-not-found",
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

/// GET /api/v1/workflows/{id}
///
/// ワークフローの詳細を取得する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id` を取得
/// 2. Core Service の `GET /internal/workflows/{id}` を呼び出し
/// 3. レスポンスを返す
pub async fn get_workflow<C, S>(
   State(state): State<Arc<WorkflowState<C, S>>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(workflow_id): Path<Uuid>,
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
      .get_workflow(workflow_id, *session_data.tenant_id().as_uuid())
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowData::from(core_response.data));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::WorkflowInstanceNotFound) => not_found_response(
         "https://ringiflow.example.com/errors/workflow-instance-not-found",
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

/// POST /api/v1/workflows/{id}/steps/{step_id}/approve
///
/// ワークフローステップを承認する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/{id}/steps/{step_id}/approve` を呼び出し
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
      .approve_step(params.id, params.step_id, core_req)
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowData::from(core_response.data));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::StepNotFound) => not_found_response(
         "https://ringiflow.example.com/errors/step-not-found",
         "Step Not Found",
         "ステップが見つかりません",
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

/// POST /api/v1/workflows/{id}/steps/{step_id}/reject
///
/// ワークフローステップを却下する
///
/// ## 処理フロー
///
/// 1. セッションから `tenant_id`, `user_id` を取得
/// 2. Core Service の `POST /internal/workflows/{id}/steps/{step_id}/reject` を呼び出し
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
      .reject_step(params.id, params.step_id, core_req)
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(WorkflowData::from(core_response.data));
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::StepNotFound) => not_found_response(
         "https://ringiflow.example.com/errors/step-not-found",
         "Step Not Found",
         "ステップが見つかりません",
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
