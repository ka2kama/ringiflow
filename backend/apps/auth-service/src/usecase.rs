//! # ユースケース層
//!
//! Auth Service のビジネスロジックを実装する。
//!
//! ## 設計方針
//!
//! - **トレイトベースの設計**: テスト可能性のためトレイトを定義
//! - **依存性注入**: リポジトリとパスワードチェッカーを外部から注入
//! - **薄いハンドラ**: ハンドラは薄く保ち、ロジックはユースケースに集約

pub mod auth;

use async_trait::async_trait;
pub use auth::{AuthUseCaseImpl, VerifyResult};
use uuid::Uuid;

use crate::error::AuthError;

/// 認証ユースケーストレイト
///
/// Auth Service のビジネスロジックを定義する。
/// 具体的な実装は `AuthUseCaseImpl` で提供される。
#[async_trait]
pub trait AuthUseCase: Send + Sync {
    /// パスワードを検証する
    ///
    /// ## 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `user_id`: ユーザー ID
    /// - `password`: 平文パスワード
    ///
    /// ## 戻り値
    ///
    /// - `Ok(VerifyResult)`: 検証結果
    /// - `Err(AuthError)`: エラー
    async fn verify_password(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        password: &str,
    ) -> Result<VerifyResult, AuthError>;

    /// 認証情報を作成する
    ///
    /// ## 引数
    ///
    /// - `user_id`: ユーザー ID
    /// - `tenant_id`: テナント ID
    /// - `credential_type`: 認証種別（"password", "totp" など）
    /// - `credential_data`: 認証データ（パスワードの場合はハッシュ値）
    ///
    /// ## 戻り値
    ///
    /// - `Ok(Uuid)`: 作成された認証情報の ID
    /// - `Err(AuthError)`: エラー
    async fn create_credential(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        credential_type: &str,
        credential_data: &str,
    ) -> Result<Uuid, AuthError>;

    /// ユーザーの全認証情報を削除する
    ///
    /// ## 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `user_id`: ユーザー ID
    async fn delete_credentials(&self, tenant_id: Uuid, user_id: Uuid) -> Result<(), AuthError>;
}

/// AuthUseCaseImpl に AuthUseCase トレイトを実装
#[async_trait]
impl AuthUseCase for AuthUseCaseImpl {
    async fn verify_password(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        password: &str,
    ) -> Result<VerifyResult, AuthError> {
        self.verify_password(tenant_id, user_id, password).await
    }

    async fn create_credential(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        credential_type: &str,
        credential_data: &str,
    ) -> Result<Uuid, AuthError> {
        self.create_credential(user_id, tenant_id, credential_type, credential_data)
            .await
    }

    async fn delete_credentials(&self, tenant_id: Uuid, user_id: Uuid) -> Result<(), AuthError> {
        self.delete_credentials(tenant_id, user_id).await
    }
}
