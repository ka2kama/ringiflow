//! # 単純な PostgreSQL Deleter
//!
//! 単一テーブルの `DELETE FROM ... WHERE tenant_id = $1` パターンを共通化するマクロと、
//! それを利用した Deleter 実装を提供する。

use async_trait::async_trait;
use ringiflow_domain::tenant::TenantId;
use sqlx::PgPool;

use super::{DeletionResult, TenantDeleter};
use crate::error::InfraError;

/// 単一テーブルの Deleter を定義するマクロ
///
/// SQL リテラルを直接渡すことで `sqlx::query!` / `sqlx::query_scalar!` の
/// コンパイル時検証を維持する。
macro_rules! define_simple_postgres_deleter {
    (
        name: $name:ident,
        deleter_name: $deleter_name:literal,
        delete_sql: $delete_sql:literal,
        count_sql: $count_sql:literal,
        doc: $doc:literal
    ) => {
        #[doc = $doc]
        pub struct $name {
            pool: PgPool,
        }

        impl $name {
            pub fn new(pool: PgPool) -> Self {
                Self { pool }
            }
        }

        #[async_trait]
        impl TenantDeleter for $name {
            fn name(&self) -> &'static str {
                $deleter_name
            }

            async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
                let result = sqlx::query!($delete_sql, tenant_id.as_uuid())
                    .execute(&self.pool)
                    .await?;

                Ok(DeletionResult {
                    deleted_count: result.rows_affected(),
                })
            }

            async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError> {
                let count = sqlx::query_scalar!($count_sql, tenant_id.as_uuid())
                    .fetch_one(&self.pool)
                    .await?;

                Ok(count as u64)
            }
        }
    };
}

define_simple_postgres_deleter!(
    name: PostgresUserDeleter,
    deleter_name: "postgres:users",
    delete_sql: "DELETE FROM users WHERE tenant_id = $1",
    count_sql: r#"SELECT COUNT(*) as "count!" FROM users WHERE tenant_id = $1"#,
    doc: "PostgreSQL ユーザー Deleter\n\nuser_roles は CASCADE で自動削除される。"
);

define_simple_postgres_deleter!(
    name: PostgresRoleDeleter,
    deleter_name: "postgres:roles",
    delete_sql: "DELETE FROM roles WHERE tenant_id = $1",
    count_sql: r#"SELECT COUNT(*) as "count!" FROM roles WHERE tenant_id = $1"#,
    doc: "PostgreSQL ロール Deleter"
);

define_simple_postgres_deleter!(
    name: PostgresDisplayIdCounterDeleter,
    deleter_name: "postgres:display_id_counters",
    delete_sql: "DELETE FROM display_id_counters WHERE tenant_id = $1",
    count_sql: r#"SELECT COUNT(*) as "count!" FROM display_id_counters WHERE tenant_id = $1"#,
    doc: "PostgreSQL 表示用 ID カウンター Deleter"
);

define_simple_postgres_deleter!(
    name: PostgresNotificationLogDeleter,
    deleter_name: "postgres:notification_logs",
    delete_sql: "DELETE FROM notification_logs WHERE tenant_id = $1",
    count_sql: r#"SELECT COUNT(*) as "count!" FROM notification_logs WHERE tenant_id = $1"#,
    doc: "PostgreSQL 通知ログ Deleter\n\nworkflow_instances の CASCADE でも削除されるが、正確な件数のため明示的に削除する。"
);

define_simple_postgres_deleter!(
    name: PostgresDocumentDeleter,
    deleter_name: "postgres:documents",
    delete_sql: "DELETE FROM documents WHERE tenant_id = $1",
    count_sql: r#"SELECT COUNT(*) as "count!" FROM documents WHERE tenant_id = $1"#,
    doc: "PostgreSQL ドキュメント Deleter\n\nworkflow_instances の CASCADE でも削除されるが、正確な件数のため明示的に削除する。"
);
