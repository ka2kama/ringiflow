//! WorkflowDefinitionRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test workflow_definition_repository_test
//! ```

mod common;

use common::{seed_definition_id, seed_tenant_id};
use ringiflow_domain::{tenant::TenantId, workflow::WorkflowDefinitionId};
use ringiflow_infra::repository::{
   PostgresWorkflowDefinitionRepository,
   WorkflowDefinitionRepository,
};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_published_by_tenant_returns_published_definitions(pool: PgPool) {
   let repo = PostgresWorkflowDefinitionRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let result = repo.find_published_by_tenant(&tenant_id).await;

   assert!(result.is_ok());
   let definitions = result.unwrap();
   assert!(!definitions.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_published_by_tenant_filters_by_tenant(pool: PgPool) {
   let repo = PostgresWorkflowDefinitionRepository::new(pool);
   let other_tenant_id = TenantId::new();

   let result = repo.find_published_by_tenant(&other_tenant_id).await;

   assert!(result.is_ok());
   let definitions = result.unwrap();
   assert!(definitions.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_returns_definition_when_exists(pool: PgPool) {
   let repo = PostgresWorkflowDefinitionRepository::new(pool);
   let definition_id = seed_definition_id();
   let tenant_id = seed_tenant_id();

   let result = repo.find_by_id(&definition_id, &tenant_id).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_returns_none_when_not_exists(pool: PgPool) {
   let repo = PostgresWorkflowDefinitionRepository::new(pool);
   let definition_id = WorkflowDefinitionId::new();
   let tenant_id = TenantId::new();

   let result = repo.find_by_id(&definition_id, &tenant_id).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}
