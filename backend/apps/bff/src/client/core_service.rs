//! # Core Service クライアント
//!
//! BFF から Core Service への通信を担当する。
//!
//! ## エンドポイント
//!
//! - `GET /internal/users/by-email` - メールアドレスでユーザーを検索
//! - `GET /internal/users/{user_id}` - ユーザー情報を取得
//! - `POST /internal/workflows` - ワークフロー作成
//! - `POST /internal/workflows/{id}/submit` - ワークフロー申請
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/08_AuthService設計.md)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Core Service クライアントエラー
#[derive(Debug, Clone, Error)]
pub enum CoreServiceError {
   /// ユーザーが見つからない（404）
   #[error("ユーザーが見つかりません")]
   UserNotFound,

   /// ワークフロー定義が見つからない（404）
   #[error("ワークフロー定義が見つかりません")]
   WorkflowDefinitionNotFound,

   /// ワークフローインスタンスが見つからない（404）
   #[error("ワークフローインスタンスが見つかりません")]
   WorkflowInstanceNotFound,

   /// ステップが見つからない（404）
   #[error("ステップが見つかりません")]
   StepNotFound,

   /// バリデーションエラー（400）
   #[error("バリデーションエラー: {0}")]
   ValidationError(String),

   /// 権限不足（403）
   #[error("権限がありません: {0}")]
   Forbidden(String),

   /// 競合（409）
   #[error("競合が発生しました: {0}")]
   Conflict(String),

   /// ネットワークエラー
   #[error("ネットワークエラー: {0}")]
   Network(String),

   /// 予期しないエラー
   #[error("予期しないエラー: {0}")]
   Unexpected(String),
}

impl From<reqwest::Error> for CoreServiceError {
   fn from(err: reqwest::Error) -> Self {
      CoreServiceError::Network(err.to_string())
   }
}

// --- レスポンス型 ---

/// ユーザー情報レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct UserResponse {
   pub id:        Uuid,
   pub tenant_id: Uuid,
   pub email:     String,
   pub name:      String,
   pub status:    String,
}

/// メールアドレス検索レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct GetUserByEmailResponse {
   pub user: UserResponse,
}

/// ユーザー詳細レスポンス（権限付き）
#[derive(Debug, Clone, Deserialize)]
pub struct UserWithPermissionsResponse {
   pub user:        UserResponse,
   pub roles:       Vec<String>,
   pub permissions: Vec<String>,
}

// --- ワークフロー関連の型 ---

/// ワークフロー作成リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct CreateWorkflowRequest {
   pub definition_id: Uuid,
   pub title:         String,
   pub form_data:     serde_json::Value,
   pub tenant_id:     Uuid,
   pub user_id:       Uuid,
}

/// ワークフロー申請リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct SubmitWorkflowRequest {
   pub assigned_to: Uuid,
   pub tenant_id:   Uuid,
}

/// ステップ承認/却下リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct ApproveRejectRequest {
   pub version:   i32,
   pub comment:   Option<String>,
   pub tenant_id: Uuid,
   pub user_id:   Uuid,
}

/// ワークフローステップ DTO
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowStepDto {
   pub id:           String,
   pub step_id:      String,
   pub step_name:    String,
   pub step_type:    String,
   pub status:       String,
   pub version:      i32,
   pub assigned_to:  Option<String>,
   pub decision:     Option<String>,
   pub comment:      Option<String>,
   pub due_date:     Option<String>,
   pub started_at:   Option<String>,
   pub completed_at: Option<String>,
   pub created_at:   String,
   pub updated_at:   String,
}

/// ワークフローインスタンス DTO
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowInstanceDto {
   pub id: String,
   pub title: String,
   pub definition_id: String,
   pub status: String,
   pub version: i32,
   pub form_data: serde_json::Value,
   pub initiated_by: String,
   pub current_step_id: Option<String>,
   #[serde(default)]
   pub steps: Vec<WorkflowStepDto>,
   pub submitted_at: Option<String>,
   pub completed_at: Option<String>,
   pub created_at: String,
   pub updated_at: String,
}

/// ワークフローレスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowResponse {
   pub data: WorkflowInstanceDto,
}

/// ワークフロー一覧レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowListResponse {
   pub data: Vec<WorkflowInstanceDto>,
}

/// ワークフロー定義 DTO
#[derive(Debug, Clone, Deserialize)]
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

/// ワークフロー定義レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowDefinitionResponse {
   pub data: WorkflowDefinitionDto,
}

/// ワークフロー定義一覧レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowDefinitionListResponse {
   pub data: Vec<WorkflowDefinitionDto>,
}

// --- タスク関連の型 ---

/// ワークフロー概要 DTO（タスク一覧用）
#[derive(Debug, Clone, Deserialize)]
pub struct TaskWorkflowSummaryDto {
   pub id:           String,
   pub title:        String,
   pub status:       String,
   pub initiated_by: String,
   pub submitted_at: Option<String>,
}

/// タスク一覧の要素 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct TaskItemDto {
   pub id:          String,
   pub step_name:   String,
   pub status:      String,
   pub version:     i32,
   pub assigned_to: Option<String>,
   pub due_date:    Option<String>,
   pub started_at:  Option<String>,
   pub created_at:  String,
   pub workflow:    TaskWorkflowSummaryDto,
}

/// タスク一覧レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct TaskListResponse {
   pub data: Vec<TaskItemDto>,
}

/// タスク詳細 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct TaskDetailDto {
   pub step:     WorkflowStepDto,
   pub workflow: WorkflowInstanceDto,
}

/// タスク詳細レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct TaskDetailResponse {
   pub data: TaskDetailDto,
}

// --- ダッシュボード関連の型 ---

/// ダッシュボード統計 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct DashboardStatsDto {
   pub pending_tasks: i64,
   pub my_workflows_in_progress: i64,
   pub completed_today: i64,
}

/// ダッシュボード統計レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct DashboardStatsResponse {
   pub data: DashboardStatsDto,
}

/// Core Service クライアントトレイト
///
/// テスト時にスタブを使用できるようトレイトで定義。
#[async_trait]
pub trait CoreServiceClient: Send + Sync {
   /// メールアドレスでユーザーを検索する
   ///
   /// Core Service の `GET /internal/users/by-email` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `email`: メールアドレス
   ///
   /// # 戻り値
   ///
   /// ユーザーが存在すれば `GetUserByEmailResponse`、なければ `CoreServiceError::UserNotFound`
   async fn get_user_by_email(
      &self,
      tenant_id: Uuid,
      email: &str,
   ) -> Result<GetUserByEmailResponse, CoreServiceError>;

   /// ユーザー情報を取得する
   ///
   /// Core Service の `GET /internal/users/{user_id}` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `user_id`: ユーザー ID
   ///
   /// # 戻り値
   ///
   /// ユーザーが存在すれば `UserWithPermissionsResponse`、なければ `CoreServiceError::UserNotFound`
   async fn get_user(&self, user_id: Uuid)
   -> Result<UserWithPermissionsResponse, CoreServiceError>;

   /// ワークフローを作成する（下書き）
   ///
   /// Core Service の `POST /internal/workflows` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `req`: ワークフロー作成リクエスト
   ///
   /// # 戻り値
   ///
   /// 作成されたワークフローインスタンス
   async fn create_workflow(
      &self,
      req: CreateWorkflowRequest,
   ) -> Result<WorkflowResponse, CoreServiceError>;

   /// ワークフローを申請する
   ///
   /// Core Service の `POST /internal/workflows/{id}/submit` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `workflow_id`: ワークフローインスタンス ID
   /// - `req`: ワークフロー申請リクエスト
   ///
   /// # 戻り値
   ///
   /// 更新されたワークフローインスタンス
   async fn submit_workflow(
      &self,
      workflow_id: Uuid,
      req: SubmitWorkflowRequest,
   ) -> Result<WorkflowResponse, CoreServiceError>;

   // ===== GET 系メソッド =====

   /// ワークフロー定義一覧を取得する
   ///
   /// Core Service の `GET /internal/workflow-definitions` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   ///
   /// # 戻り値
   ///
   /// 公開済みワークフロー定義の一覧
   async fn list_workflow_definitions(
      &self,
      tenant_id: Uuid,
   ) -> Result<WorkflowDefinitionListResponse, CoreServiceError>;

   /// ワークフロー定義の詳細を取得する
   ///
   /// Core Service の `GET /internal/workflow-definitions/{id}` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `definition_id`: ワークフロー定義 ID
   /// - `tenant_id`: テナント ID
   ///
   /// # 戻り値
   ///
   /// ワークフロー定義
   async fn get_workflow_definition(
      &self,
      definition_id: Uuid,
      tenant_id: Uuid,
   ) -> Result<WorkflowDefinitionResponse, CoreServiceError>;

   /// 自分のワークフロー一覧を取得する
   ///
   /// Core Service の `GET /internal/workflows` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `user_id`: ユーザー ID
   ///
   /// # 戻り値
   ///
   /// 自分が申請したワークフローインスタンスの一覧
   async fn list_my_workflows(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<WorkflowListResponse, CoreServiceError>;

   /// ワークフローの詳細を取得する
   ///
   /// Core Service の `GET /internal/workflows/{id}` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `workflow_id`: ワークフローインスタンス ID
   /// - `tenant_id`: テナント ID
   ///
   /// # 戻り値
   ///
   /// ワークフローインスタンス
   async fn get_workflow(
      &self,
      workflow_id: Uuid,
      tenant_id: Uuid,
   ) -> Result<WorkflowResponse, CoreServiceError>;

   // ===== 承認/却下系メソッド =====

   /// ワークフローステップを承認する
   ///
   /// Core Service の `POST /internal/workflows/{id}/steps/{step_id}/approve` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `workflow_id`: ワークフローインスタンス ID
   /// - `step_id`: ステップ ID
   /// - `req`: 承認リクエスト
   ///
   /// # 戻り値
   ///
   /// 更新されたワークフローインスタンス（ステップ情報含む）
   async fn approve_step(
      &self,
      workflow_id: Uuid,
      step_id: Uuid,
      req: ApproveRejectRequest,
   ) -> Result<WorkflowResponse, CoreServiceError>;

   /// ワークフローステップを却下する
   ///
   /// Core Service の `POST /internal/workflows/{id}/steps/{step_id}/reject` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `workflow_id`: ワークフローインスタンス ID
   /// - `step_id`: ステップ ID
   /// - `req`: 却下リクエスト
   ///
   /// # 戻り値
   ///
   /// 更新されたワークフローインスタンス（ステップ情報含む）
   async fn reject_step(
      &self,
      workflow_id: Uuid,
      step_id: Uuid,
      req: ApproveRejectRequest,
   ) -> Result<WorkflowResponse, CoreServiceError>;

   // ===== タスク系メソッド =====

   /// 自分のタスク一覧を取得する
   ///
   /// Core Service の `GET /internal/tasks/my` を呼び出す。
   async fn list_my_tasks(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<TaskListResponse, CoreServiceError>;

   /// タスク詳細を取得する
   ///
   /// Core Service の `GET /internal/tasks/{id}` を呼び出す。
   async fn get_task(
      &self,
      task_id: Uuid,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<TaskDetailResponse, CoreServiceError>;

   // ===== ダッシュボード系メソッド =====

   /// ダッシュボード統計情報を取得する
   ///
   /// Core Service の `GET /internal/dashboard/stats` を呼び出す。
   async fn get_dashboard_stats(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<DashboardStatsResponse, CoreServiceError>;
}

/// Core Service クライアント実装
#[derive(Clone)]
pub struct CoreServiceClientImpl {
   base_url: String,
   client:   reqwest::Client,
}

impl CoreServiceClientImpl {
   /// 新しい CoreServiceClient を作成する
   ///
   /// # 引数
   ///
   /// - `base_url`: Core Service のベース URL（例: `http://localhost:13001`）
   pub fn new(base_url: &str) -> Self {
      Self {
         base_url: base_url.trim_end_matches('/').to_string(),
         client:   reqwest::Client::new(),
      }
   }
}

#[async_trait]
impl CoreServiceClient for CoreServiceClientImpl {
   async fn get_user_by_email(
      &self,
      tenant_id: Uuid,
      email: &str,
   ) -> Result<GetUserByEmailResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/users/by-email?email={}&tenant_id={}",
         self.base_url,
         urlencoding::encode(email),
         tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<GetUserByEmailResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::UserNotFound),
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn get_user(
      &self,
      user_id: Uuid,
   ) -> Result<UserWithPermissionsResponse, CoreServiceError> {
      let url = format!("{}/internal/users/{}", self.base_url, user_id);

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<UserWithPermissionsResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::UserNotFound),
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn create_workflow(
      &self,
      req: CreateWorkflowRequest,
   ) -> Result<WorkflowResponse, CoreServiceError> {
      let url = format!("{}/internal/workflows", self.base_url);

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<WorkflowResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::WorkflowDefinitionNotFound),
         reqwest::StatusCode::BAD_REQUEST => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::ValidationError(body))
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn submit_workflow(
      &self,
      workflow_id: Uuid,
      req: SubmitWorkflowRequest,
   ) -> Result<WorkflowResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/{}/submit",
         self.base_url, workflow_id
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<WorkflowResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::WorkflowInstanceNotFound),
         reqwest::StatusCode::BAD_REQUEST => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::ValidationError(body))
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   // ===== GET 系メソッドの実装 =====

   async fn list_workflow_definitions(
      &self,
      tenant_id: Uuid,
   ) -> Result<WorkflowDefinitionListResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/workflow-definitions?tenant_id={}",
         self.base_url, tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<WorkflowDefinitionListResponse>().await?;
            Ok(body)
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn get_workflow_definition(
      &self,
      definition_id: Uuid,
      tenant_id: Uuid,
   ) -> Result<WorkflowDefinitionResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/workflow-definitions/{}?tenant_id={}",
         self.base_url, definition_id, tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<WorkflowDefinitionResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::WorkflowDefinitionNotFound),
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn list_my_workflows(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<WorkflowListResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows?tenant_id={}&user_id={}",
         self.base_url, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<WorkflowListResponse>().await?;
            Ok(body)
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn get_workflow(
      &self,
      workflow_id: Uuid,
      tenant_id: Uuid,
   ) -> Result<WorkflowResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/{}?tenant_id={}",
         self.base_url, workflow_id, tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<WorkflowResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::WorkflowInstanceNotFound),
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   // ===== 承認/却下系メソッドの実装 =====

   async fn approve_step(
      &self,
      workflow_id: Uuid,
      step_id: Uuid,
      req: ApproveRejectRequest,
   ) -> Result<WorkflowResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/{}/steps/{}/approve",
         self.base_url, workflow_id, step_id
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<WorkflowResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::StepNotFound),
         reqwest::StatusCode::BAD_REQUEST => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::ValidationError(body))
         }
         reqwest::StatusCode::FORBIDDEN => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Forbidden(body))
         }
         reqwest::StatusCode::CONFLICT => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Conflict(body))
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn reject_step(
      &self,
      workflow_id: Uuid,
      step_id: Uuid,
      req: ApproveRejectRequest,
   ) -> Result<WorkflowResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/{}/steps/{}/reject",
         self.base_url, workflow_id, step_id
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<WorkflowResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::StepNotFound),
         reqwest::StatusCode::BAD_REQUEST => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::ValidationError(body))
         }
         reqwest::StatusCode::FORBIDDEN => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Forbidden(body))
         }
         reqwest::StatusCode::CONFLICT => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Conflict(body))
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   // ===== タスク系メソッドの実装 =====

   async fn list_my_tasks(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<TaskListResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/tasks/my?tenant_id={}&user_id={}",
         self.base_url, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<TaskListResponse>().await?;
            Ok(body)
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn get_task(
      &self,
      task_id: Uuid,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<TaskDetailResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/tasks/{}?tenant_id={}&user_id={}",
         self.base_url, task_id, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<TaskDetailResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::StepNotFound),
         reqwest::StatusCode::FORBIDDEN => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Forbidden(body))
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   // ===== ダッシュボード系メソッドの実装 =====

   async fn get_dashboard_stats(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<DashboardStatsResponse, CoreServiceError> {
      let url = format!(
         "{}/internal/dashboard/stats?tenant_id={}&user_id={}",
         self.base_url, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<DashboardStatsResponse>().await?;
            Ok(body)
         }
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreServiceError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }
}

#[cfg(test)]
mod tests {
   // 統合テストで実際の Core Service との通信をテストする
}
