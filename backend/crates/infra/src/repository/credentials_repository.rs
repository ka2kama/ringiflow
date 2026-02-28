//! # CredentialsRepository
//!
//! 認証情報の永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **auth スキーマ**: credentials テーブルは auth スキーマに配置
//! - **テナント分離**: すべてのクエリでテナント ID を考慮
//! - **型安全なクエリ**: sqlx のコンパイル時検証を活用
//!
//! 詳細: [Auth Service
//! 設計](../../../../docs/03_詳細設計書/08_AuthService設計.md)

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ringiflow_domain::{tenant::TenantId, user::UserId};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::InfraError;

/// Credential の種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialType {
    /// パスワード認証
    Password,
    /// TOTP（将来用）
    Totp,
    /// OIDC SSO（将来用）
    Oidc,
    /// SAML SSO（将来用）
    Saml,
}

impl CredentialType {
    /// 文字列への変換
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Totp => "totp",
            Self::Oidc => "oidc",
            Self::Saml => "saml",
        }
    }
}

impl FromStr for CredentialType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "password" => Ok(Self::Password),
            "totp" => Ok(Self::Totp),
            "oidc" => Ok(Self::Oidc),
            "saml" => Ok(Self::Saml),
            _ => Err(format!("不正な credential_type: {}", s)),
        }
    }
}

/// 認証情報エンティティ
#[derive(Debug, Clone)]
pub struct Credential {
    pub id: Uuid,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub credential_type: CredentialType,
    pub credential_data: String,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 認証情報リポジトリトレイト
///
/// 認証情報の永続化操作を定義する。
/// インフラ層で具体的な実装を提供し、Auth Service から利用する。
#[async_trait]
pub trait CredentialsRepository: Send + Sync {
    /// ユーザーの認証情報を取得（種別指定）
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `user_id`: ユーザー ID
    /// - `credential_type`: 認証種別
    ///
    /// # 戻り値
    ///
    /// - `Ok(Some(credential))`: 認証情報が見つかった場合
    /// - `Ok(None)`: 認証情報が見つからない場合
    /// - `Err(_)`: データベースエラー
    async fn find_by_user_and_type(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        credential_type: CredentialType,
    ) -> Result<Option<Credential>, InfraError>;

    /// 認証情報を作成
    ///
    /// # 引数
    ///
    /// - `user_id`: ユーザー ID
    /// - `tenant_id`: テナント ID
    /// - `credential_type`: 認証種別
    /// - `credential_data`: 認証データ（パスワードの場合はハッシュ値）
    ///
    /// # 戻り値
    ///
    /// - `Ok(credential_id)`: 作成された認証情報の ID
    /// - `Err(_)`: データベースエラー
    async fn create(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        credential_type: CredentialType,
        credential_data: &str,
    ) -> Result<Uuid, InfraError>;

    /// ユーザーの全認証情報を削除
    ///
    /// ユーザー削除時に呼び出される。
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `user_id`: ユーザー ID
    ///
    /// # 戻り値
    ///
    /// - `Ok(())`: 削除成功（該当なしも成功とみなす）
    /// - `Err(_)`: データベースエラー
    async fn delete_by_user(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<(), InfraError>;

    /// テナントの全認証情報を削除
    ///
    /// テナント退会時に呼び出される。
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    ///
    /// # 戻り値
    ///
    /// - `Ok(())`: 削除成功
    /// - `Err(_)`: データベースエラー
    async fn delete_by_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError>;

    /// 最終使用日時を更新
    ///
    /// 認証成功時に呼び出される。
    ///
    /// # 引数
    ///
    /// - `id`: 認証情報 ID
    async fn update_last_used(&self, id: Uuid) -> Result<(), InfraError>;
}

/// PostgreSQL 実装の CredentialsRepository
#[derive(Debug, Clone)]
pub struct PostgresCredentialsRepository {
    pool: PgPool,
}

impl PostgresCredentialsRepository {
    /// 新しいリポジトリインスタンスを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CredentialsRepository for PostgresCredentialsRepository {
    async fn find_by_user_and_type(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        credential_type: CredentialType,
    ) -> Result<Option<Credential>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                user_id,
                tenant_id,
                credential_type,
                credential_data,
                is_active,
                last_used_at,
                created_at,
                updated_at
            FROM auth.credentials
            WHERE tenant_id = $1 AND user_id = $2 AND credential_type = $3
            "#,
            tenant_id.as_uuid(),
            user_id.as_uuid(),
            credential_type.as_str()
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let cred_type: CredentialType = row
            .credential_type
            .parse()
            .map_err(InfraError::unexpected)?;

        Ok(Some(Credential {
            id: row.id,
            user_id: UserId::from_uuid(row.user_id),
            tenant_id: TenantId::from_uuid(row.tenant_id),
            credential_type: cred_type,
            credential_data: row.credential_data,
            is_active: row.is_active,
            last_used_at: row.last_used_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn create(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        credential_type: CredentialType,
        credential_data: &str,
    ) -> Result<Uuid, InfraError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO auth.credentials (user_id, tenant_id, credential_type, credential_data)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            user_id.as_uuid(),
            tenant_id.as_uuid(),
            credential_type.as_str(),
            credential_data
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    async fn delete_by_user(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            DELETE FROM auth.credentials
            WHERE tenant_id = $1 AND user_id = $2
            "#,
            tenant_id.as_uuid(),
            user_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_by_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            DELETE FROM auth.credentials
            WHERE tenant_id = $1
            "#,
            tenant_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_last_used(&self, id: Uuid) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            UPDATE auth.credentials
            SET last_used_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_type_変換() {
        assert_eq!(
            "password".parse::<CredentialType>().unwrap(),
            CredentialType::Password
        );
        assert_eq!(CredentialType::Password.as_str(), "password");
        assert!("invalid".parse::<CredentialType>().is_err());
    }

    #[test]
    fn test_トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PostgresCredentialsRepository>();
    }
}
