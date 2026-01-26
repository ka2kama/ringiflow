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

   /// バリデーションエラー（400）
   #[error("バリデーションエラー: {0}")]
   ValidationError(String),

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

/// ワークフローインスタンス DTO
#[derive(Debug, Clone, Deserialize)]
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

/// ワークフローレスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowResponse {
   pub data: WorkflowInstanceDto,
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
}

#[cfg(test)]
mod tests {
   // 統合テストで実際の Core Service との通信をテストする
}
