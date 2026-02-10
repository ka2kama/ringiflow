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

mod error;
mod types;

use async_trait::async_trait;
pub use error::*;
use ringiflow_shared::ApiResponse;
pub use types::*;
use uuid::Uuid;

/// Core Service クライアントトレイト
///
/// テスト時にスタブを使用できるようトレイトで定義。
#[async_trait]
pub trait CoreServiceClient: Send + Sync {
   /// テナント内のアクティブユーザー一覧を取得する
   ///
   /// Core Service の `GET /internal/users` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   ///
   /// # 戻り値
   ///
   /// アクティブユーザーの一覧
   async fn list_users(
      &self,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<Vec<UserItemDto>>, CoreServiceError>;

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
   /// ユーザーが存在すれば `ApiResponse<UserResponse>`、なければ `CoreServiceError::UserNotFound`
   async fn get_user_by_email(
      &self,
      tenant_id: Uuid,
      email: &str,
   ) -> Result<ApiResponse<UserResponse>, CoreServiceError>;

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
   /// ユーザーが存在すれば `ApiResponse<UserWithPermissionsData>`、なければ `CoreServiceError::UserNotFound`
   async fn get_user(
      &self,
      user_id: Uuid,
   ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError>;

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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

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
   ) -> Result<ApiResponse<Vec<WorkflowDefinitionDto>>, CoreServiceError>;

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
   ) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError>;

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
   ) -> Result<ApiResponse<Vec<WorkflowInstanceDto>>, CoreServiceError>;

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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   // ===== タスク系メソッド =====

   /// 自分のタスク一覧を取得する
   ///
   /// Core Service の `GET /internal/tasks/my` を呼び出す。
   async fn list_my_tasks(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<ApiResponse<Vec<TaskItemDto>>, CoreServiceError>;

   /// タスク詳細を取得する
   ///
   /// Core Service の `GET /internal/tasks/{id}` を呼び出す。
   async fn get_task(
      &self,
      task_id: Uuid,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<ApiResponse<TaskDetailDto>, CoreServiceError>;

   // ===== ダッシュボード系メソッド =====

   /// ダッシュボード統計情報を取得する
   ///
   /// Core Service の `GET /internal/dashboard/stats` を呼び出す。
   async fn get_dashboard_stats(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<ApiResponse<DashboardStatsDto>, CoreServiceError>;

   // ===== display_number 対応メソッド =====

   /// display_number でワークフローの詳細を取得する
   ///
   /// Core Service の `GET /internal/workflows/by-display-number/{display_number}` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `display_number`: 表示用連番
   /// - `tenant_id`: テナント ID
   ///
   /// # 戻り値
   ///
   /// ワークフローインスタンス
   async fn get_workflow_by_display_number(
      &self,
      display_number: i64,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// display_number でワークフローを申請する
   ///
   /// Core Service の `POST /internal/workflows/by-display-number/{display_number}/submit` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `display_number`: 表示用連番
   /// - `req`: ワークフロー申請リクエスト
   ///
   /// # 戻り値
   ///
   /// 更新されたワークフローインスタンス
   async fn submit_workflow_by_display_number(
      &self,
      display_number: i64,
      req: SubmitWorkflowRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// display_number でワークフローステップを承認する
   ///
   /// Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/approve` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `workflow_display_number`: ワークフローの表示用連番
   /// - `step_display_number`: ステップの表示用連番
   /// - `req`: 承認リクエスト
   ///
   /// # 戻り値
   ///
   /// 更新されたワークフローインスタンス（ステップ情報含む）
   async fn approve_step_by_display_number(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      req: ApproveRejectRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// display_number でワークフローステップを却下する
   ///
   /// Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/reject` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `workflow_display_number`: ワークフローの表示用連番
   /// - `step_display_number`: ステップの表示用連番
   /// - `req`: 却下リクエスト
   ///
   /// # 戻り値
   ///
   /// 更新されたワークフローインスタンス（ステップ情報含む）
   async fn reject_step_by_display_number(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      req: ApproveRejectRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// display_number でタスク詳細を取得する
   ///
   /// Core Service の `GET /internal/workflows/by-display-number/{wf_dn}/tasks/{step_dn}` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `workflow_display_number`: ワークフローの表示用連番
   /// - `step_display_number`: ステップの表示用連番
   /// - `tenant_id`: テナント ID
   /// - `user_id`: ユーザー ID
   ///
   /// # 戻り値
   ///
   /// タスク詳細（ステップ + ワークフロー情報）
   async fn get_task_by_display_numbers(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<ApiResponse<TaskDetailDto>, CoreServiceError>;
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
   async fn list_users(
      &self,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<Vec<UserItemDto>>, CoreServiceError> {
      let url = format!("{}/internal/users?tenant_id={}", self.base_url, tenant_id);

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<Vec<UserItemDto>>>().await?;
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

   async fn get_user_by_email(
      &self,
      tenant_id: Uuid,
      email: &str,
   ) -> Result<ApiResponse<UserResponse>, CoreServiceError> {
      let url = format!(
         "{}/internal/users/by-email?email={}&tenant_id={}",
         self.base_url,
         urlencoding::encode(email),
         tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<UserResponse>>().await?;
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
   ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError> {
      let url = format!("{}/internal/users/{}", self.base_url, user_id);

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response
               .json::<ApiResponse<UserWithPermissionsData>>()
               .await?;
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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!("{}/internal/workflows", self.base_url);

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/{}/submit",
         self.base_url, workflow_id
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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
   ) -> Result<ApiResponse<Vec<WorkflowDefinitionDto>>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflow-definitions?tenant_id={}",
         self.base_url, tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response
               .json::<ApiResponse<Vec<WorkflowDefinitionDto>>>()
               .await?;
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
   ) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflow-definitions/{}?tenant_id={}",
         self.base_url, definition_id, tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response
               .json::<ApiResponse<WorkflowDefinitionDto>>()
               .await?;
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
   ) -> Result<ApiResponse<Vec<WorkflowInstanceDto>>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows?tenant_id={}&user_id={}",
         self.base_url, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response
               .json::<ApiResponse<Vec<WorkflowInstanceDto>>>()
               .await?;
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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/{}?tenant_id={}",
         self.base_url, workflow_id, tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/{}/steps/{}/approve",
         self.base_url, workflow_id, step_id
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/{}/steps/{}/reject",
         self.base_url, workflow_id, step_id
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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
   ) -> Result<ApiResponse<Vec<TaskItemDto>>, CoreServiceError> {
      let url = format!(
         "{}/internal/tasks/my?tenant_id={}&user_id={}",
         self.base_url, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<Vec<TaskItemDto>>>().await?;
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
   ) -> Result<ApiResponse<TaskDetailDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/tasks/{}?tenant_id={}&user_id={}",
         self.base_url, task_id, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<TaskDetailDto>>().await?;
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
   ) -> Result<ApiResponse<DashboardStatsDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/dashboard/stats?tenant_id={}&user_id={}",
         self.base_url, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<DashboardStatsDto>>().await?;
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

   // ===== display_number 対応メソッドの実装 =====

   async fn get_workflow_by_display_number(
      &self,
      display_number: i64,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/by-display-number/{}?tenant_id={}",
         self.base_url, display_number, tenant_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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

   async fn submit_workflow_by_display_number(
      &self,
      display_number: i64,
      req: SubmitWorkflowRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/by-display-number/{}/submit",
         self.base_url, display_number
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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

   async fn approve_step_by_display_number(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      req: ApproveRejectRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/by-display-number/{}/steps/by-display-number/{}/approve",
         self.base_url, workflow_display_number, step_display_number
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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

   async fn reject_step_by_display_number(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      req: ApproveRejectRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/by-display-number/{}/steps/by-display-number/{}/reject",
         self.base_url, workflow_display_number, step_display_number
      );

      let response = self.client.post(&url).json(&req).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<WorkflowInstanceDto>>().await?;
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

   async fn get_task_by_display_numbers(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<ApiResponse<TaskDetailDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/workflows/by-display-number/{}/tasks/{}?tenant_id={}&user_id={}",
         self.base_url, workflow_display_number, step_display_number, tenant_id, user_id
      );

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<ApiResponse<TaskDetailDto>>().await?;
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
}

#[cfg(test)]
mod tests {
   // 統合テストで実際の Core Service との通信をテストする
}
