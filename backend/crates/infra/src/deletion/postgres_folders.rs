//! # PostgreSQL フォルダー Deleter
//!
//! `folders` テーブルは `parent_id → folders(id) ON DELETE RESTRICT` の自己参照 FK を持つ。
//! RESTRICT は各行削除時に即座にチェックされるため、単純な `DELETE FROM folders WHERE tenant_id = $1`
//! では親フォルダの削除が子フォルダより先に処理された場合に FK 違反が発生する。
//!
//! このため depth の深い順（5 → 1）に削除する。

use async_trait::async_trait;
use ringiflow_domain::tenant::TenantId;
use sqlx::PgPool;

use super::{DeletionResult, TenantDeleter};
use crate::error::InfraError;

/// PostgreSQL フォルダー Deleter
///
/// 自己参照 FK のため depth 降順で削除する。
pub struct PostgresFoldersDeleter {
    pool: PgPool,
}

impl PostgresFoldersDeleter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantDeleter for PostgresFoldersDeleter {
    fn name(&self) -> &'static str {
        "postgres:folders"
    }

    async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
        let mut total_deleted: u64 = 0;

        // 自己参照 FK (parent_id → id, ON DELETE RESTRICT) のため、
        // 深い階層から順に削除する（depth 5 → 1）
        for depth in (1..=5).rev() {
            let result = sqlx::query!(
                "DELETE FROM folders WHERE tenant_id = $1 AND depth = $2",
                tenant_id.as_uuid(),
                depth
            )
            .execute(&self.pool)
            .await?;
            total_deleted += result.rows_affected();
        }

        Ok(DeletionResult {
            deleted_count: total_deleted,
        })
    }

    async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM folders WHERE tenant_id = $1"#,
            tenant_id.as_uuid()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }
}
