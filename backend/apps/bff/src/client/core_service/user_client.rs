//! ユーザー関連の Core Service クライアント

use async_trait::async_trait;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
   client_impl::CoreServiceClientImpl,
   error::CoreServiceError,
   response::handle_response,
   types::{
      CreateUserCoreRequest,
      CreateUserCoreResponse,
      UpdateUserCoreRequest,
      UpdateUserStatusCoreRequest,
      UserItemDto,
      UserResponse,
      UserWithPermissionsData,
   },
};

/// ユーザー関連の Core Service クライアントトレイト
#[async_trait]
pub trait CoreServiceUserClient: Send + Sync {
   /// テナント内のユーザー一覧を取得する
   ///
   /// Core Service の `GET /internal/users` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `status`: ステータスフィルタ（省略時は deleted 以外すべて）
   async fn list_users(
      &self,
      tenant_id: Uuid,
      status: Option<&str>,
   ) -> Result<ApiResponse<Vec<UserItemDto>>, CoreServiceError>;

   /// メールアドレスでユーザーを検索する
   ///
   /// Core Service の `GET /internal/users/by-email` を呼び出す。
   async fn get_user_by_email(
      &self,
      tenant_id: Uuid,
      email: &str,
   ) -> Result<ApiResponse<UserResponse>, CoreServiceError>;

   /// ユーザー情報を取得する
   ///
   /// Core Service の `GET /internal/users/{user_id}` を呼び出す。
   async fn get_user(
      &self,
      user_id: Uuid,
   ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError>;

   /// ユーザーを作成する
   ///
   /// Core Service の `POST /internal/users` を呼び出す。
   async fn create_user(
      &self,
      req: &CreateUserCoreRequest,
   ) -> Result<ApiResponse<CreateUserCoreResponse>, CoreServiceError>;

   /// ユーザー情報を更新する
   ///
   /// Core Service の `PATCH /internal/users/{user_id}` を呼び出す。
   async fn update_user(
      &self,
      user_id: Uuid,
      req: &UpdateUserCoreRequest,
   ) -> Result<ApiResponse<UserResponse>, CoreServiceError>;

   /// ユーザーステータスを変更する
   ///
   /// Core Service の `PATCH /internal/users/{user_id}/status` を呼び出す。
   async fn update_user_status(
      &self,
      user_id: Uuid,
      req: &UpdateUserStatusCoreRequest,
   ) -> Result<ApiResponse<UserResponse>, CoreServiceError>;

   /// 表示用連番でユーザーを取得する
   ///
   /// Core Service の `GET /internal/users/by-display-number/{display_number}` を呼び出す。
   async fn get_user_by_display_number(
      &self,
      tenant_id: Uuid,
      display_number: i64,
   ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError>;
}

#[async_trait]
impl CoreServiceUserClient for CoreServiceClientImpl {
   async fn list_users(
      &self,
      tenant_id: Uuid,
      status: Option<&str>,
   ) -> Result<ApiResponse<Vec<UserItemDto>>, CoreServiceError> {
      let mut url = format!("{}/internal/users?tenant_id={}", self.base_url, tenant_id);
      if let Some(s) = status {
         url.push_str(&format!("&status={}", s));
      }

      let response = self.client.get(&url).send().await?;
      handle_response(response, None).await
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
      handle_response(response, Some(CoreServiceError::UserNotFound)).await
   }

   async fn get_user(
      &self,
      user_id: Uuid,
   ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError> {
      let url = format!("{}/internal/users/{}", self.base_url, user_id);

      let response = self.client.get(&url).send().await?;
      handle_response(response, Some(CoreServiceError::UserNotFound)).await
   }

   async fn create_user(
      &self,
      req: &CreateUserCoreRequest,
   ) -> Result<ApiResponse<CreateUserCoreResponse>, CoreServiceError> {
      let url = format!("{}/internal/users", self.base_url);

      let response = self.client.post(&url).json(req).send().await?;
      handle_response(response, None).await
   }

   async fn update_user(
      &self,
      user_id: Uuid,
      req: &UpdateUserCoreRequest,
   ) -> Result<ApiResponse<UserResponse>, CoreServiceError> {
      let url = format!("{}/internal/users/{}", self.base_url, user_id);

      let response = self.client.patch(&url).json(req).send().await?;
      handle_response(response, Some(CoreServiceError::UserNotFound)).await
   }

   async fn update_user_status(
      &self,
      user_id: Uuid,
      req: &UpdateUserStatusCoreRequest,
   ) -> Result<ApiResponse<UserResponse>, CoreServiceError> {
      let url = format!("{}/internal/users/{}/status", self.base_url, user_id);

      let response = self.client.patch(&url).json(req).send().await?;
      handle_response(response, Some(CoreServiceError::UserNotFound)).await
   }

   async fn get_user_by_display_number(
      &self,
      tenant_id: Uuid,
      display_number: i64,
   ) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError> {
      let url = format!(
         "{}/internal/users/by-display-number/{}?tenant_id={}",
         self.base_url, display_number, tenant_id
      );

      let response = self.client.get(&url).send().await?;
      handle_response(response, Some(CoreServiceError::UserNotFound)).await
   }
}
