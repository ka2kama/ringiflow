//! ロール関連の Core Service クライアント

use async_trait::async_trait;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
   client_impl::CoreServiceClientImpl,
   error::CoreServiceError,
   response::handle_response,
   types::{CreateRoleCoreRequest, RoleDetailDto, RoleItemDto, UpdateRoleCoreRequest},
};

/// ロール関連の Core Service クライアントトレイト
#[async_trait]
pub trait CoreServiceRoleClient: Send + Sync {
   /// テナント内のロール一覧を取得する
   ///
   /// Core Service の `GET /internal/roles` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   async fn list_roles(
      &self,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<Vec<RoleItemDto>>, CoreServiceError>;

   /// ロール詳細を取得する
   ///
   /// Core Service の `GET /internal/roles/{role_id}` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `role_id`: ロール ID
   /// - `tenant_id`: テナント ID（テナント分離用）
   async fn get_role(
      &self,
      role_id: Uuid,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<RoleDetailDto>, CoreServiceError>;

   /// カスタムロールを作成する
   ///
   /// Core Service の `POST /internal/roles` を呼び出す。
   async fn create_role(
      &self,
      req: &CreateRoleCoreRequest,
   ) -> Result<ApiResponse<RoleDetailDto>, CoreServiceError>;

   /// カスタムロールを更新する
   ///
   /// Core Service の `PATCH /internal/roles/{role_id}` を呼び出す。
   async fn update_role(
      &self,
      role_id: Uuid,
      req: &UpdateRoleCoreRequest,
   ) -> Result<ApiResponse<RoleDetailDto>, CoreServiceError>;

   /// カスタムロールを削除する
   ///
   /// Core Service の `DELETE /internal/roles/{role_id}` を呼び出す。
   async fn delete_role(&self, role_id: Uuid) -> Result<(), CoreServiceError>;
}

#[async_trait]
impl CoreServiceRoleClient for CoreServiceClientImpl {
   async fn list_roles(
      &self,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<Vec<RoleItemDto>>, CoreServiceError> {
      let url = format!("{}/internal/roles?tenant_id={}", self.base_url, tenant_id);

      let response = self.client.get(&url).send().await?;
      handle_response(response, None).await
   }

   async fn get_role(
      &self,
      role_id: Uuid,
      tenant_id: Uuid,
   ) -> Result<ApiResponse<RoleDetailDto>, CoreServiceError> {
      let url = format!(
         "{}/internal/roles/{}?tenant_id={}",
         self.base_url, role_id, tenant_id
      );

      let response = self.client.get(&url).send().await?;
      handle_response(response, Some(CoreServiceError::RoleNotFound)).await
   }

   async fn create_role(
      &self,
      req: &CreateRoleCoreRequest,
   ) -> Result<ApiResponse<RoleDetailDto>, CoreServiceError> {
      let url = format!("{}/internal/roles", self.base_url);

      let response = self.client.post(&url).json(req).send().await?;
      handle_response(response, None).await
   }

   async fn update_role(
      &self,
      role_id: Uuid,
      req: &UpdateRoleCoreRequest,
   ) -> Result<ApiResponse<RoleDetailDto>, CoreServiceError> {
      let url = format!("{}/internal/roles/{}", self.base_url, role_id);

      let response = self.client.patch(&url).json(req).send().await?;
      handle_response(response, Some(CoreServiceError::RoleNotFound)).await
   }

   async fn delete_role(&self, role_id: Uuid) -> Result<(), CoreServiceError> {
      let url = format!("{}/internal/roles/{}", self.base_url, role_id);

      let response = self.client.delete(&url).send().await?;
      let status = response.status();

      if status.is_success() {
         return Ok(());
      }

      let body = response.text().await.unwrap_or_default();

      let error = match status {
         reqwest::StatusCode::NOT_FOUND => CoreServiceError::RoleNotFound,
         reqwest::StatusCode::BAD_REQUEST => CoreServiceError::ValidationError(body),
         reqwest::StatusCode::CONFLICT => CoreServiceError::Conflict(body),
         _ => CoreServiceError::Unexpected(format!("予期しないステータス {}: {}", status, body)),
      };

      Err(error)
   }
}
