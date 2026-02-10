//! タスク・ダッシュボード関連の Core Service クライアント

use async_trait::async_trait;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
   client_impl::CoreServiceClientImpl,
   error::CoreServiceError,
   types::{DashboardStatsDto, TaskDetailDto, TaskItemDto},
};

/// タスク・ダッシュボード関連の Core Service クライアントトレイト
#[async_trait]
pub trait CoreServiceTaskClient: Send + Sync {
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

   /// ダッシュボード統計情報を取得する
   ///
   /// Core Service の `GET /internal/dashboard/stats` を呼び出す。
   async fn get_dashboard_stats(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<ApiResponse<DashboardStatsDto>, CoreServiceError>;

   /// display_number でタスク詳細を取得する
   ///
   /// Core Service の `GET /internal/workflows/by-display-number/{wf_dn}/tasks/{step_dn}` を呼び出す。
   async fn get_task_by_display_numbers(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<ApiResponse<TaskDetailDto>, CoreServiceError>;
}

#[async_trait]
impl CoreServiceTaskClient for CoreServiceClientImpl {
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
