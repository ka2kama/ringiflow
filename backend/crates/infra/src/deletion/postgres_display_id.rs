//! # PostgresDisplayIdCounterDeleter
//!
//! テナントの表示用 ID カウンターを削除する。

use async_trait::async_trait;
use ringiflow_domain::tenant::TenantId;
use sqlx::PgPool;

use super::{DeletionResult, TenantDeleter};
use crate::error::InfraError;

/// PostgreSQL 表示用 ID カウンター Deleter
pub struct PostgresDisplayIdCounterDeleter {
   pool: PgPool,
}

impl PostgresDisplayIdCounterDeleter {
   pub fn new(pool: PgPool) -> Self {
      Self { pool }
   }
}

#[async_trait]
impl TenantDeleter for PostgresDisplayIdCounterDeleter {
   fn name(&self) -> &'static str {
      "postgres:display_id_counters"
   }

   async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
      let result = sqlx::query!(
         "DELETE FROM display_id_counters WHERE tenant_id = $1",
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
         r#"SELECT COUNT(*) as "count!" FROM display_id_counters WHERE tenant_id = $1"#,
         tenant_id.as_uuid()
      )
      .fetch_one(&self.pool)
      .await?;

      Ok(count as u64)
   }
}
