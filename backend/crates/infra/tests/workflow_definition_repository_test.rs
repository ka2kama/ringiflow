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
async fn test_テナントの公開済み定義一覧を取得できる(pool: PgPool) {
    let sut = PostgresWorkflowDefinitionRepository::new(pool);
    let tenant_id = seed_tenant_id();

    let result = sut.find_published_by_tenant(&tenant_id).await;

    assert!(result.is_ok());
    let definitions = result.unwrap();
    assert!(!definitions.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_別テナントの定義は取得できない(pool: PgPool) {
    let sut = PostgresWorkflowDefinitionRepository::new(pool);
    let other_tenant_id = TenantId::new();

    let result = sut.find_published_by_tenant(&other_tenant_id).await;

    assert!(result.is_ok());
    let definitions = result.unwrap();
    assert!(definitions.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_idで定義を取得できる(pool: PgPool) {
    let sut = PostgresWorkflowDefinitionRepository::new(pool);
    let definition_id = seed_definition_id();
    let tenant_id = seed_tenant_id();

    let result = sut.find_by_id(&definition_id, &tenant_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_存在しないidの場合noneを返す(pool: PgPool) {
    let sut = PostgresWorkflowDefinitionRepository::new(pool);
    let definition_id = WorkflowDefinitionId::new();
    let tenant_id = TenantId::new();

    let result = sut.find_by_id(&definition_id, &tenant_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}
