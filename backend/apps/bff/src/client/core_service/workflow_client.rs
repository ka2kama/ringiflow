//! ワークフロー関連の Core Service クライアント

use async_trait::async_trait;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
   client_impl::CoreServiceClientImpl,
   error::CoreServiceError,
   types::{
      ApproveRejectRequest,
      CreateWorkflowRequest,
      SubmitWorkflowRequest,
      WorkflowDefinitionDto,
      WorkflowInstanceDto,
   },
};

/// ワークフロー関連の Core Service クライアントトレイト
#[async_trait]
pub trait CoreServiceWorkflowClient: Send + Sync {
   /// ワークフローを作成する（下書き）
   ///
   /// Core Service の `POST /internal/workflows` を呼び出す。
   async fn create_workflow(
      &self,
      req: CreateWorkflowRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// ワークフローを申請する
   ///
   /// Core Service の `POST /internal/workflows/{id}/submit` を呼び出す。
   async fn submit_workflow(
      &self,
      workflow_id: Uuid,
      req: SubmitWorkflowRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// ワークフロー定義一覧を取得する
   ///
   /// Core Service の `GET /internal/workflow-definitions` を呼び出す。
   async fn list_workflow_definitions(
      &self,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<Vec<WorkflowDefinitionDto>>, CoreServiceError>;

   /// ワークフロー定義の詳細を取得する
   ///
   /// Core Service の `GET /internal/workflow-definitions/{id}` を呼び出す。
   async fn get_workflow_definition(
      &self,
      definition_id: Uuid,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError>;

   /// 自分のワークフロー一覧を取得する
   ///
   /// Core Service の `GET /internal/workflows` を呼び出す。
   async fn list_my_workflows(
      &self,
      tenant_id: Uuid,
      user_id: Uuid,
   ) -> Result<ApiResponse<Vec<WorkflowInstanceDto>>, CoreServiceError>;

   /// ワークフローの詳細を取得する
   ///
   /// Core Service の `GET /internal/workflows/{id}` を呼び出す。
   async fn get_workflow(
      &self,
      workflow_id: Uuid,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// ワークフローステップを承認する
   ///
   /// Core Service の `POST /internal/workflows/{id}/steps/{step_id}/approve` を呼び出す。
   async fn approve_step(
      &self,
      workflow_id: Uuid,
      step_id: Uuid,
      req: ApproveRejectRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// ワークフローステップを却下する
   ///
   /// Core Service の `POST /internal/workflows/{id}/steps/{step_id}/reject` を呼び出す。
   async fn reject_step(
      &self,
      workflow_id: Uuid,
      step_id: Uuid,
      req: ApproveRejectRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// display_number でワークフローの詳細を取得する
   ///
   /// Core Service の `GET /internal/workflows/by-display-number/{display_number}` を呼び出す。
   async fn get_workflow_by_display_number(
      &self,
      display_number: i64,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// display_number でワークフローを申請する
   ///
   /// Core Service の `POST /internal/workflows/by-display-number/{display_number}/submit` を呼び出す。
   async fn submit_workflow_by_display_number(
      &self,
      display_number: i64,
      req: SubmitWorkflowRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// display_number でワークフローステップを承認する
   ///
   /// Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/approve` を呼び出す。
   async fn approve_step_by_display_number(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      req: ApproveRejectRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

   /// display_number でワークフローステップを却下する
   ///
   /// Core Service の `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/reject` を呼び出す。
   async fn reject_step_by_display_number(
      &self,
      workflow_display_number: i64,
      step_display_number: i64,
      req: ApproveRejectRequest,
   ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;
}

#[async_trait]
impl CoreServiceWorkflowClient for CoreServiceClientImpl {
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
}
