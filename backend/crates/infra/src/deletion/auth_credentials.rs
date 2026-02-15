//! # AuthCredentialsDeleter
//!
//! テナントの認証情報を削除する。
//! auth.credentials テーブルは auth スキーマに属し、
//! users テーブルへの FK 制約を持たない（サービス境界の独立性のため）。

use async_trait::async_trait;
use ringiflow_domain::tenant::TenantId;
use sqlx::PgPool;

use super::{DeletionResult, TenantDeleter};
use crate::error::InfraError;

/// 認証情報 Deleter
pub struct AuthCredentialsDeleter {
    pool: PgPool,
}

impl AuthCredentialsDeleter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantDeleter for AuthCredentialsDeleter {
    fn name(&self) -> &'static str {
        "auth:credentials"
    }

    async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
        let result = sqlx::query!(
            "DELETE FROM auth.credentials WHERE tenant_id = $1",
            tenant_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(DeletionResult {
            deleted_count: result.rows_affected(),
        })
    }

    async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM auth.credentials WHERE tenant_id = $1"#,
            tenant_id.as_uuid()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }
}
