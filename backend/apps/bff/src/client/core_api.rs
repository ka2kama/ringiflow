//! # Core API クライアント
//!
//! BFF から Core API への通信を担当する。
//!
//! ## エンドポイント
//!
//! - `GET /internal/users/by-email` - メールアドレスでユーザーを検索
//! - `GET /internal/users/{user_id}` - ユーザー情報を取得
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/08_AuthService設計.md)

use async_trait::async_trait;
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

/// Core API クライアントエラー
#[derive(Debug, Clone, Error)]
pub enum CoreApiError {
   /// ユーザーが見つからない（404）
   #[error("ユーザーが見つかりません")]
   UserNotFound,

   /// ネットワークエラー
   #[error("ネットワークエラー: {0}")]
   Network(String),

   /// 予期しないエラー
   #[error("予期しないエラー: {0}")]
   Unexpected(String),
}

impl From<reqwest::Error> for CoreApiError {
   fn from(err: reqwest::Error) -> Self {
      CoreApiError::Network(err.to_string())
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

/// Core API クライアントトレイト
///
/// テスト時にスタブを使用できるようトレイトで定義。
#[async_trait]
pub trait CoreApiClient: Send + Sync {
   /// メールアドレスでユーザーを検索する
   ///
   /// Core API の `GET /internal/users/by-email` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `email`: メールアドレス
   ///
   /// # 戻り値
   ///
   /// ユーザーが存在すれば `GetUserByEmailResponse`、なければ `CoreApiError::UserNotFound`
   async fn get_user_by_email(
      &self,
      tenant_id: Uuid,
      email: &str,
   ) -> Result<GetUserByEmailResponse, CoreApiError>;

   /// ユーザー情報を取得する
   ///
   /// Core API の `GET /internal/users/{user_id}` を呼び出す。
   ///
   /// # 引数
   ///
   /// - `user_id`: ユーザー ID
   ///
   /// # 戻り値
   ///
   /// ユーザーが存在すれば `UserWithPermissionsResponse`、なければ `CoreApiError::UserNotFound`
   async fn get_user(&self, user_id: Uuid) -> Result<UserWithPermissionsResponse, CoreApiError>;
}

/// Core API クライアント実装
pub struct CoreApiClientImpl {
   base_url: String,
   client:   reqwest::Client,
}

impl CoreApiClientImpl {
   /// 新しい CoreApiClient を作成する
   ///
   /// # 引数
   ///
   /// - `base_url`: Core API のベース URL（例: `http://localhost:13001`）
   pub fn new(base_url: &str) -> Self {
      Self {
         base_url: base_url.trim_end_matches('/').to_string(),
         client:   reqwest::Client::new(),
      }
   }
}

#[async_trait]
impl CoreApiClient for CoreApiClientImpl {
   async fn get_user_by_email(
      &self,
      tenant_id: Uuid,
      email: &str,
   ) -> Result<GetUserByEmailResponse, CoreApiError> {
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
         reqwest::StatusCode::NOT_FOUND => Err(CoreApiError::UserNotFound),
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreApiError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }

   async fn get_user(&self, user_id: Uuid) -> Result<UserWithPermissionsResponse, CoreApiError> {
      let url = format!("{}/internal/users/{}", self.base_url, user_id);

      let response = self.client.get(&url).send().await?;

      match response.status() {
         status if status.is_success() => {
            let body = response.json::<UserWithPermissionsResponse>().await?;
            Ok(body)
         }
         reqwest::StatusCode::NOT_FOUND => Err(CoreApiError::UserNotFound),
         status => {
            let body = response.text().await.unwrap_or_default();
            Err(CoreApiError::Unexpected(format!(
               "予期しないステータス {}: {}",
               status, body
            )))
         }
      }
   }
}

#[cfg(test)]
mod tests {
   // 統合テストで実際の Core API との通信をテストする
}
