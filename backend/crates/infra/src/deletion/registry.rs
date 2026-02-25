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
    DeletionReport,
    DynamoDbAuditLogDeleter,
    PostgresDisplayIdCounterDeleter,
    PostgresFoldersDeleter,
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
        registry.register(Box::new(PostgresFoldersDeleter::new(pg_pool.clone())));
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
            "postgres:folders",
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
    /// 全 Deleter を実行し、成功/失敗を分けて [`DeletionReport`] で返す。
    /// 個別の Deleter がエラーを返しても、残りの Deleter は実行を継続する。
    pub async fn delete_all(&self, tenant_id: &TenantId) -> DeletionReport {
        let mut succeeded = HashMap::new();
        let mut failed = Vec::new();

        for deleter in &self.deleters {
            match deleter.delete(tenant_id).await {
                Ok(result) => {
                    succeeded.insert(deleter.name(), result);
                }
                Err(error) => {
                    tracing::error!(
                        deleter = deleter.name(),
                        error = %error,
                        "テナントデータ削除に失敗"
                    );
                    failed.push((deleter.name(), error));
                }
            }
        }

        DeletionReport { succeeded, failed }
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
    use crate::deletion::DeletionResult;

    /// テスト用のモック Deleter
    struct MockDeleter {
        name:        &'static str,
        count:       AtomicU64,
        should_fail: bool,
    }

    impl MockDeleter {
        fn new(name: &'static str, initial_count: u64) -> Self {
            Self {
                name,
                count: AtomicU64::new(initial_count),
                should_fail: false,
            }
        }

        fn failing(name: &'static str) -> Self {
            Self {
                name,
                count: AtomicU64::new(0),
                should_fail: true,
            }
        }
    }

    #[async_trait::async_trait]
    impl TenantDeleter for MockDeleter {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn delete(&self, _tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
            if self.should_fail {
                return Err(InfraError::Unexpected(format!(
                    "{}: テスト用エラー",
                    self.name
                )));
            }
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
    async fn test_delete_allが全成功時succeededに全結果を返しfailedが空() {
        let mut registry = DeletionRegistry::new();
        registry.register(Box::new(MockDeleter::new("test:a", 3)));
        registry.register(Box::new(MockDeleter::new("test:b", 5)));

        let tenant_id = TenantId::new();
        let report = registry.delete_all(&tenant_id).await;

        assert!(!report.has_failures());
        assert_eq!(report.succeeded.len(), 2);
        assert_eq!(report.succeeded["test:a"].deleted_count, 3);
        assert_eq!(report.succeeded["test:b"].deleted_count, 5);
        assert!(report.failed.is_empty());
    }

    #[tokio::test]
    async fn test_delete_allで1つ目が失敗しても残りのdeleterが実行される() {
        let mut registry = DeletionRegistry::new();
        registry.register(Box::new(MockDeleter::failing("test:a")));
        registry.register(Box::new(MockDeleter::new("test:b", 5)));

        let tenant_id = TenantId::new();
        let report = registry.delete_all(&tenant_id).await;

        assert!(report.has_failures());
        assert_eq!(report.succeeded.len(), 1);
        assert_eq!(report.succeeded["test:b"].deleted_count, 5);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, "test:a");
    }

    #[tokio::test]
    async fn test_delete_allで最後のdeleterが失敗しても先行の成功結果が残る() {
        let mut registry = DeletionRegistry::new();
        registry.register(Box::new(MockDeleter::new("test:a", 3)));
        registry.register(Box::new(MockDeleter::failing("test:b")));

        let tenant_id = TenantId::new();
        let report = registry.delete_all(&tenant_id).await;

        assert!(report.has_failures());
        assert_eq!(report.succeeded.len(), 1);
        assert_eq!(report.succeeded["test:a"].deleted_count, 3);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, "test:b");
    }

    #[tokio::test]
    async fn test_delete_allで全deleterが失敗した場合succeededが空でfailedに全エラー() {
        let mut registry = DeletionRegistry::new();
        registry.register(Box::new(MockDeleter::failing("test:a")));
        registry.register(Box::new(MockDeleter::failing("test:b")));

        let tenant_id = TenantId::new();
        let report = registry.delete_all(&tenant_id).await;

        assert!(report.has_failures());
        assert!(report.succeeded.is_empty());
        assert_eq!(report.failed.len(), 2);
        assert_eq!(report.failed[0].0, "test:a");
        assert_eq!(report.failed[1].0, "test:b");
    }

    #[tokio::test]
    async fn test_has_failuresが失敗なしでfalseを返す() {
        let mut registry = DeletionRegistry::new();
        registry.register(Box::new(MockDeleter::new("test:a", 1)));

        let tenant_id = TenantId::new();
        let report = registry.delete_all(&tenant_id).await;

        assert!(!report.has_failures());
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
