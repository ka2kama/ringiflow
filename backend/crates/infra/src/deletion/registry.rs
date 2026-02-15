//! # DeletionRegistry
//!
//! 全データストアの `TenantDeleter` を集約し、一括操作を提供する。

use std::collections::HashMap;

use aws_sdk_dynamodb::Client as DynamoDbClient;
use redis::aio::ConnectionManager;
use ringiflow_domain::tenant::TenantId;
use sqlx::PgPool;

use super::{
    AuthCredentialsDeleter,
    DeletionResult,
    DynamoDbAuditLogDeleter,
    PostgresDisplayIdCounterDeleter,
    PostgresRoleDeleter,
    PostgresUserDeleter,
    PostgresWorkflowDeleter,
    RedisSessionDeleter,
    TenantDeleter,
};
use crate::error::InfraError;

/// テナントデータ削除レジストリ
///
/// 全データストアの `TenantDeleter` を保持し、一括削除・件数確認を提供する。
pub struct DeletionRegistry {
    deleters: Vec<Box<dyn TenantDeleter>>,
}

impl Default for DeletionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DeletionRegistry {
    /// 空のレジストリを生成する
    pub fn new() -> Self {
        Self {
            deleters: Vec::new(),
        }
    }

    /// Deleter を登録する
    pub fn register(&mut self, deleter: Box<dyn TenantDeleter>) {
        self.deleters.push(deleter);
    }

    /// 全データストアの Deleter を登録済みのレジストリを生成する
    pub fn with_all_deleters(
        pg_pool: PgPool,
        dynamodb_client: DynamoDbClient,
        dynamodb_table_name: String,
        redis_conn: ConnectionManager,
    ) -> Self {
        let mut registry = Self::new();
        // FK 安全な順序: 参照元（子）→ 参照先（親）
        // workflow_definitions.created_by → users(id) (NO CASCADE)
        // workflow_instances.initiated_by → users(id) (NO CASCADE)
        // → workflows を users より先に削除する必要がある
        registry.register(Box::new(PostgresWorkflowDeleter::new(pg_pool.clone())));
        registry.register(Box::new(AuthCredentialsDeleter::new(pg_pool.clone())));
        registry.register(Box::new(PostgresDisplayIdCounterDeleter::new(
            pg_pool.clone(),
        )));
        registry.register(Box::new(PostgresRoleDeleter::new(pg_pool.clone())));
        registry.register(Box::new(PostgresUserDeleter::new(pg_pool)));
        registry.register(Box::new(DynamoDbAuditLogDeleter::new(
            dynamodb_client,
            dynamodb_table_name,
        )));
        registry.register(Box::new(RedisSessionDeleter::new(redis_conn)));
        registry
    }

    /// 期待される Deleter 名の一覧を返す（登録漏れ検出テスト用）
    pub fn expected_deleter_names() -> Vec<&'static str> {
        vec![
            "postgres:workflows",
            "auth:credentials",
            "postgres:display_id_counters",
            "postgres:roles",
            "postgres:users",
            "dynamodb:audit_logs",
            "redis:sessions",
        ]
    }

    /// 登録済み Deleter の名前一覧を返す
    pub fn registered_names(&self) -> Vec<&'static str> {
        self.deleters.iter().map(|d| d.name()).collect()
    }

    /// 全 Deleter でテナントデータを削除する
    ///
    /// 各 Deleter の結果を名前をキーとした HashMap で返す。
    /// いずれかの Deleter がエラーを返した場合、そのエラーを即座に返す。
    pub async fn delete_all(
        &self,
        tenant_id: &TenantId,
    ) -> Result<HashMap<&'static str, DeletionResult>, InfraError> {
        let mut results = HashMap::new();
        for deleter in &self.deleters {
            let result = deleter.delete(tenant_id).await?;
            results.insert(deleter.name(), result);
        }
        Ok(results)
    }

    /// 全 Deleter でテナントデータの件数を取得する
    ///
    /// 各 Deleter の件数を名前をキーとした HashMap で返す。
    pub async fn count_all(
        &self,
        tenant_id: &TenantId,
    ) -> Result<HashMap<&'static str, u64>, InfraError> {
        let mut results = HashMap::new();
        for deleter in &self.deleters {
            let count = deleter.count(tenant_id).await?;
            results.insert(deleter.name(), count);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    /// テスト用のモック Deleter
    struct MockDeleter {
        name:  &'static str,
        count: AtomicU64,
    }

    impl MockDeleter {
        fn new(name: &'static str, initial_count: u64) -> Self {
            Self {
                name,
                count: AtomicU64::new(initial_count),
            }
        }
    }

    #[async_trait::async_trait]
    impl TenantDeleter for MockDeleter {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn delete(&self, _tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
            let deleted = self.count.swap(0, Ordering::SeqCst);
            Ok(DeletionResult {
                deleted_count: deleted,
            })
        }

        async fn count(&self, _tenant_id: &TenantId) -> Result<u64, InfraError> {
            Ok(self.count.load(Ordering::SeqCst))
        }
    }

    #[test]
    fn test_空のレジストリのregistered_namesは空vecを返す() {
        let registry = DeletionRegistry::new();
        assert!(registry.registered_names().is_empty());
    }

    #[test]
    fn test_deleterを登録するとregistered_namesで名前を取得できる() {
        let mut registry = DeletionRegistry::new();
        registry.register(Box::new(MockDeleter::new("test:a", 0)));
        registry.register(Box::new(MockDeleter::new("test:b", 0)));

        let names = registry.registered_names();
        assert_eq!(names, vec!["test:a", "test:b"]);
    }

    #[tokio::test]
    async fn test_delete_allが全deleterのdeleteを呼び結果を返す() {
        let mut registry = DeletionRegistry::new();
        registry.register(Box::new(MockDeleter::new("test:a", 3)));
        registry.register(Box::new(MockDeleter::new("test:b", 5)));

        let tenant_id = TenantId::new();
        let results = registry.delete_all(&tenant_id).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results["test:a"].deleted_count, 3);
        assert_eq!(results["test:b"].deleted_count, 5);
    }

    #[tokio::test]
    async fn test_count_allが全deleterのcountを呼び結果を返す() {
        let mut registry = DeletionRegistry::new();
        registry.register(Box::new(MockDeleter::new("test:a", 10)));
        registry.register(Box::new(MockDeleter::new("test:b", 20)));

        let tenant_id = TenantId::new();
        let counts = registry.count_all(&tenant_id).await.unwrap();

        assert_eq!(counts.len(), 2);
        assert_eq!(counts["test:a"], 10);
        assert_eq!(counts["test:b"], 20);
    }
}
