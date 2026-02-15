//! # TenantRepository
//!
//! テナント情報の取得を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **読み取り専用**: テナント作成・更新は将来のスコープ
//! - **型安全なクエリ**: sqlx のコンパイル時検証を活用

use async_trait::async_trait;
use ringiflow_domain::tenant::{Tenant, TenantId, TenantName};
use sqlx::PgPool;

use crate::error::InfraError;

/// テナントリポジトリトレイト
///
/// テナント情報の読み取り操作を定義する。
#[async_trait]
pub trait TenantRepository: Send + Sync {
    /// ID でテナントを検索
    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, InfraError>;
}

/// PostgreSQL 実装の TenantRepository
#[derive(Debug, Clone)]
pub struct PostgresTenantRepository {
    pool: PgPool,
}

impl PostgresTenantRepository {
    /// 新しいリポジトリインスタンスを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantRepository for PostgresTenantRepository {
    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT id, name
            FROM tenants
            WHERE id = $1
            "#,
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let tenant = Tenant::from_db(
            TenantId::from_uuid(row.id),
            TenantName::new(&row.name).map_err(|e| InfraError::Unexpected(e.to_string()))?,
        );

        Ok(Some(tenant))
    }
}
