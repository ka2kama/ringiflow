//! # Auth Service クライアント
//!
//! BFF から Auth Service への通信を担当する。
//!
//! ## エンドポイント
//!
//! - `POST /internal/auth/verify` - パスワード認証
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/
//! 08_AuthService設計.md)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Auth Service クライアントエラー
#[derive(Debug, Clone, Error)]
pub enum AuthServiceError {
    /// 認証失敗（401）
    #[error("認証に失敗しました")]
    AuthenticationFailed,

    /// リクエストエラー（400）
    #[error("リクエストエラー: {0}")]
    BadRequest(String),

    /// ネットワークエラー
    #[error("ネットワークエラー: {0}")]
    Network(String),

    /// Auth Service が利用不可（503）
    #[error("Auth Service が一時的に利用できません")]
    ServiceUnavailable,

    /// 予期しないエラー
    #[error("予期しないエラー: {0}")]
    Unexpected(String),
}

impl From<reqwest::Error> for AuthServiceError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() || err.is_timeout() {
            AuthServiceError::ServiceUnavailable
        } else {
            AuthServiceError::Network(err.to_string())
        }
    }
}

// --- リクエスト/レスポンス型 ---

/// パスワード認証リクエスト
#[derive(Debug, Serialize)]
struct VerifyRequest {
    tenant_id: Uuid,
    user_id:   Uuid,
    password:  String,
}

/// パスワード認証レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct VerifyResponse {
    pub verified:      bool,
    pub credential_id: Option<Uuid>,
}

/// 認証情報作成レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCredentialsResponse {
    pub credential_id: Uuid,
}

/// Auth Service クライアントトレイト
///
/// テスト時にスタブを使用できるようトレイトで定義。
#[async_trait]
pub trait AuthServiceClient: Send + Sync {
    /// パスワード認証を実行する
    ///
    /// Auth Service の `POST /internal/auth/verify` を呼び出す。
    async fn verify_password(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        password: &str,
    ) -> Result<VerifyResponse, AuthServiceError>;

    /// 認証情報を作成する
    ///
    /// Auth Service の `POST /internal/auth/credentials` を呼び出す。
    async fn create_credentials(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        credential_type: &str,
        credential_data: &str,
    ) -> Result<CreateCredentialsResponse, AuthServiceError>;
}

/// Auth Service クライアント実装
pub struct AuthServiceClientImpl {
    base_url: String,
    client:   reqwest::Client,
}

impl AuthServiceClientImpl {
    /// 新しい AuthServiceClient を作成する
    ///
    /// # 引数
    ///
    /// - `base_url`: Auth Service のベース URL（例: `http://localhost:13002`）
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client:   reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AuthServiceClient for AuthServiceClientImpl {
    async fn verify_password(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        password: &str,
    ) -> Result<VerifyResponse, AuthServiceError> {
        let url = format!("{}/internal/auth/verify", self.base_url);
        let request = VerifyRequest {
            tenant_id,
            user_id,
            password: password.to_string(),
        };

        let response = self.client.post(&url).json(&request).send().await?;

        match response.status() {
            status if status.is_success() => {
                let body = response.json::<VerifyResponse>().await?;
                if body.verified {
                    Ok(body)
                } else {
                    Err(AuthServiceError::AuthenticationFailed)
                }
            }
            reqwest::StatusCode::UNAUTHORIZED => Err(AuthServiceError::AuthenticationFailed),
            reqwest::StatusCode::SERVICE_UNAVAILABLE => Err(AuthServiceError::ServiceUnavailable),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(AuthServiceError::Unexpected(format!(
                    "予期しないステータス {}: {}",
                    status, body
                )))
            }
        }
    }

    async fn create_credentials(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        credential_type: &str,
        credential_data: &str,
    ) -> Result<CreateCredentialsResponse, AuthServiceError> {
        let url = format!("{}/internal/auth/credentials", self.base_url);

        #[derive(Serialize)]
        struct CreateCredentialsRequest {
            user_id:         Uuid,
            tenant_id:       Uuid,
            credential_type: String,
            credential_data: String,
        }

        let request = CreateCredentialsRequest {
            user_id,
            tenant_id,
            credential_type: credential_type.to_string(),
            credential_data: credential_data.to_string(),
        };

        let response = self.client.post(&url).json(&request).send().await?;

        match response.status() {
            status if status.is_success() => {
                let body = response.json::<CreateCredentialsResponse>().await?;
                Ok(body)
            }
            reqwest::StatusCode::BAD_REQUEST => {
                let body = response.text().await.unwrap_or_default();
                Err(AuthServiceError::BadRequest(body))
            }
            reqwest::StatusCode::SERVICE_UNAVAILABLE => Err(AuthServiceError::ServiceUnavailable),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(AuthServiceError::Unexpected(format!(
                    "予期しないステータス {}: {}",
                    status, body
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // 統合テストで実際の Auth Service との通信をテストする
}
