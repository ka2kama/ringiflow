//! ユーザー関連の Core Service クライアント

use async_trait::async_trait;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
   client_impl::CoreServiceClientImpl,
   error::CoreServiceError,
   types::{UserItemDto, UserResponse, UserWithPermissionsData},
};

/// ユーザー関連の Core Service クライアントトレイト
#[async_trait]
pub trait CoreServiceUserClient: Send + Sync {
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
}

#[async_trait]
impl CoreServiceUserClient for CoreServiceClientImpl {
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
}
