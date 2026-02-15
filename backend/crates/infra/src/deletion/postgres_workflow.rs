//! # PostgresWorkflowDeleter
//!
//! テナントのワークフローデータを削除する。
//! workflow_comments → workflow_steps → workflow_instances → workflow_definitions の順で DELETE する。
//!
//! ## FK 制約
//!
//! - workflow_comments.instance_id → workflow_instances(id) ON DELETE CASCADE
//! - workflow_steps.instance_id → workflow_instances(id) ON DELETE CASCADE
//! - workflow_instances.definition_id → workflow_definitions(id)（CASCADE なし）
//!
//! CASCADE があるため instances 削除で comments/steps も消えるが、
//! 明示的に全テーブルを削除し、正確な件数を返す。

use async_trait::async_trait;
use ringiflow_domain::tenant::TenantId;
use sqlx::PgPool;

use super::{DeletionResult, TenantDeleter};
use crate::error::InfraError;

/// PostgreSQL ワークフロー Deleter
pub struct PostgresWorkflowDeleter {
    pool: PgPool,
}

impl PostgresWorkflowDeleter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantDeleter for PostgresWorkflowDeleter {
    fn name(&self) -> &'static str {
        "postgres:workflows"
    }

    async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
        let mut tx = self.pool.begin().await?;

        // FK 制約に従い子テーブルから順に削除（トランザクションで一貫性を保証）
        let comments = sqlx::query!(
            "DELETE FROM workflow_comments WHERE tenant_id = $1",
            tenant_id.as_uuid()
        )
        .execute(&mut *tx)
        .await?;

        let steps = sqlx::query!(
            "DELETE FROM workflow_steps WHERE tenant_id = $1",
            tenant_id.as_uuid()
        )
        .execute(&mut *tx)
        .await?;

        let instances = sqlx::query!(
            "DELETE FROM workflow_instances WHERE tenant_id = $1",
            tenant_id.as_uuid()
        )
        .execute(&mut *tx)
        .await?;

        let definitions = sqlx::query!(
            "DELETE FROM workflow_definitions WHERE tenant_id = $1",
            tenant_id.as_uuid()
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(DeletionResult {
            deleted_count: comments.rows_affected()
                + steps.rows_affected()
                + instances.rows_affected()
                + definitions.rows_affected(),
        })
    }

    async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError> {
        // workflow_definitions の件数を返す（代表テーブル）
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM workflow_definitions WHERE tenant_id = $1"#,
            tenant_id.as_uuid()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }
}
