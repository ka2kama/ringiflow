//! PostgreSQL Deleter + Auth Credentials Deleter 統合テスト
//!
//! 実行方法:
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-infra --test postgres_deleter_test
//! ```

mod common;

use common::{create_other_tenant, insert_user_raw, setup_test_data};
use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::deletion::{
    AuthCredentialsDeleter,
    DeletionRegistry,
    PostgresDisplayIdCounterDeleter,
    PostgresRoleDeleter,
    PostgresUserDeleter,
    PostgresWorkflowDeleter,
    TenantDeleter,
};
use sqlx::PgPool;
use uuid::Uuid;

/// テスト用に別テナントのデータも作成するヘルパー
async fn setup_two_tenants(pool: &PgPool) -> (TenantId, TenantId) {
    let (tenant_a, _) = setup_test_data(pool).await;
    let tenant_b = create_other_tenant(pool).await;
    insert_user_raw(pool, &tenant_b, 1, "b@example.com", "User B", "active").await;
    (tenant_a, tenant_b)
}

/// count → delete → count=0 の共通アサーション
async fn assert_count_delete_count<T: TenantDeleter>(
    sut: &T,
    tenant_id: &TenantId,
    expected_count: u64,
    expected_deleted: u64,
) {
    let count = sut.count(tenant_id).await.unwrap();
    assert_eq!(count, expected_count);
    let result = sut.delete(tenant_id).await.unwrap();
    assert_eq!(result.deleted_count, expected_deleted);
    let count_after = sut.count(tenant_id).await.unwrap();
    assert_eq!(count_after, 0);
}

// =============================================================================
// PostgresUserDeleter
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_user_deleter_countがテナントのユーザー数を返す(pool: PgPool) {
    let (tenant_id, _) = setup_test_data(&pool).await;
    let sut = PostgresUserDeleter::new(pool);

    let count = sut.count(&tenant_id).await.unwrap();

    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_user_deleter_deleteがテナントのユーザーを削除し件数を返す(
    pool: PgPool,
) {
    let (tenant_id, _) = setup_test_data(&pool).await;
    let sut = PostgresUserDeleter::new(pool.clone());

    let result = sut.delete(&tenant_id).await.unwrap();

    assert_eq!(result.deleted_count, 1);

    // 削除後のカウントが 0
    let count = sut.count(&tenant_id).await.unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_user_deleter_他テナントのユーザーは削除されない(pool: PgPool) {
    let (tenant_a, tenant_b) = setup_two_tenants(&pool).await;
    let sut = PostgresUserDeleter::new(pool);

    sut.delete(&tenant_a).await.unwrap();

    // テナント B のユーザーは残っている
    let count_b = sut.count(&tenant_b).await.unwrap();
    assert_eq!(count_b, 1);
}

// =============================================================================
// PostgresRoleDeleter
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_role_deleter_countとdeleteが正しく動作する(pool: PgPool) {
    let (tenant_id, _) = setup_test_data(&pool).await;

    // テナント固有ロールを作成
    sqlx::query!(
      "INSERT INTO roles (id, tenant_id, name, description, permissions, is_system) VALUES ($1, $2, 'custom_role', 'Custom', '[]', false)",
      Uuid::now_v7(),
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    let sut = PostgresRoleDeleter::new(pool);

    assert_count_delete_count(&sut, &tenant_id, 1, 1).await;
}

// =============================================================================
// PostgresWorkflowDeleter
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_workflow_deleter_countとdeleteが正しく動作する(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;

    // ワークフロー定義を作成
    let def_id = Uuid::now_v7();
    sqlx::query!(
      "INSERT INTO workflow_definitions (id, tenant_id, name, description, definition, version, status, created_by) VALUES ($1, $2, 'Test WF', 'desc', '{}', 1, 'published', $3)",
      def_id,
      tenant_id.as_uuid(),
      user_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    // ワークフローインスタンスを作成
    let inst_id = Uuid::now_v7();
    sqlx::query!(
      "INSERT INTO workflow_instances (id, tenant_id, definition_id, definition_version, display_number, title, form_data, status, initiated_by) VALUES ($1, $2, $3, 1, 100, 'Test Instance', '{}', 'pending', $4)",
      inst_id,
      tenant_id.as_uuid(),
      def_id,
      user_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    // ワークフローステップを作成
    sqlx::query!(
      "INSERT INTO workflow_steps (id, instance_id, tenant_id, display_number, step_id, step_name, step_type, status, assigned_to) VALUES ($1, $2, $3, 1, 'step1', 'Approval', 'approval', 'pending', $4)",
      Uuid::now_v7(),
      inst_id,
      tenant_id.as_uuid(),
      user_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    let sut = PostgresWorkflowDeleter::new(pool);

    // count は definitions の件数、delete は steps + instances + definitions の合計件数
    assert_count_delete_count(&sut, &tenant_id, 1, 3).await;
}

// =============================================================================
// PostgresDisplayIdCounterDeleter
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_display_id_counter_deleter_countとdeleteが正しく動作する(pool: PgPool) {
    let (tenant_id, _) = setup_test_data(&pool).await;

    // カウンターを作成
    sqlx::query!(
      "INSERT INTO display_id_counters (tenant_id, entity_type, last_number) VALUES ($1, 'user', 10)",
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    let sut = PostgresDisplayIdCounterDeleter::new(pool);

    assert_count_delete_count(&sut, &tenant_id, 1, 1).await;
}

// =============================================================================
// AuthCredentialsDeleter
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_auth_credentials_deleter_countとdeleteが正しく動作する(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;

    // 認証情報を作成
    sqlx::query!(
      "INSERT INTO auth.credentials (id, user_id, tenant_id, credential_type, credential_data) VALUES ($1, $2, $3, 'password', '$argon2id$hash')",
      Uuid::now_v7(),
      user_id.as_uuid(),
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    let sut = AuthCredentialsDeleter::new(pool);

    assert_count_delete_count(&sut, &tenant_id, 1, 1).await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_auth_credentials_deleter_他テナントのcredentialsは削除されない(
    pool: PgPool,
) {
    let (tenant_a, tenant_b) = setup_two_tenants(&pool).await;

    // 両テナントに credentials を作成
    let user_b_id: Uuid = sqlx::query_scalar!(
        r#"SELECT id as "id!" FROM users WHERE tenant_id = $1 LIMIT 1"#,
        tenant_b.as_uuid()
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let user_a_id: Uuid = sqlx::query_scalar!(
        r#"SELECT id as "id!" FROM users WHERE tenant_id = $1 LIMIT 1"#,
        tenant_a.as_uuid()
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query!(
      "INSERT INTO auth.credentials (id, user_id, tenant_id, credential_type, credential_data) VALUES ($1, $2, $3, 'password', 'hash_a')",
      Uuid::now_v7(),
      user_a_id,
      tenant_a.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    sqlx::query!(
      "INSERT INTO auth.credentials (id, user_id, tenant_id, credential_type, credential_data) VALUES ($1, $2, $3, 'password', 'hash_b')",
      Uuid::now_v7(),
      user_b_id,
      tenant_b.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    let sut = AuthCredentialsDeleter::new(pool);

    sut.delete(&tenant_a).await.unwrap();

    let count_b = sut.count(&tenant_b).await.unwrap();
    assert_eq!(count_b, 1);
}

// =============================================================================
// DeletionRegistry::delete_all 統合テスト
// =============================================================================

/// 全 PostgreSQL Deleter + Auth の delete_all が FK
/// 制約に違反せず完了することを検証する。 DynamoDB / Redis は DB
/// 統合テスト環境では接続できないため、PostgreSQL 系のみ登録。
#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_allがfk制約に違反せず全テーブルを削除できる(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;

    // ロールを作成
    sqlx::query!(
      "INSERT INTO roles (id, tenant_id, name, description, permissions, is_system) VALUES ($1, $2, 'test_role', 'Test', '[]', false)",
      Uuid::now_v7(),
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    // ワークフロー定義を作成（created_by → users FK）
    let def_id = Uuid::now_v7();
    sqlx::query!(
      "INSERT INTO workflow_definitions (id, tenant_id, name, description, definition, version, status, created_by) VALUES ($1, $2, 'WF', 'desc', '{}', 1, 'published', $3)",
      def_id,
      tenant_id.as_uuid(),
      user_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    // ワークフローインスタンスを作成（initiated_by → users FK）
    let inst_id = Uuid::now_v7();
    sqlx::query!(
      "INSERT INTO workflow_instances (id, tenant_id, definition_id, definition_version, display_number, title, form_data, status, initiated_by) VALUES ($1, $2, $3, 1, 200, 'Instance', '{}', 'pending', $4)",
      inst_id,
      tenant_id.as_uuid(),
      def_id,
      user_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    // ワークフローステップを作成（assigned_to → users FK）
    sqlx::query!(
      "INSERT INTO workflow_steps (id, instance_id, tenant_id, display_number, step_id, step_name, step_type, status, assigned_to) VALUES ($1, $2, $3, 1, 'step1', 'Approval', 'approval', 'pending', $4)",
      Uuid::now_v7(),
      inst_id,
      tenant_id.as_uuid(),
      user_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    // カウンターを作成
    sqlx::query!(
      "INSERT INTO display_id_counters (tenant_id, entity_type, last_number) VALUES ($1, 'user', 10)",
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    // 認証情報を作成
    sqlx::query!(
      "INSERT INTO auth.credentials (id, user_id, tenant_id, credential_type, credential_data) VALUES ($1, $2, $3, 'password', 'hash')",
      Uuid::now_v7(),
      user_id.as_uuid(),
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .unwrap();

    // DeletionRegistry に PostgreSQL 系 Deleter のみ登録（FK 安全な順序で）
    let mut registry = DeletionRegistry::new();
    registry.register(Box::new(PostgresWorkflowDeleter::new(pool.clone())));
    registry.register(Box::new(AuthCredentialsDeleter::new(pool.clone())));
    registry.register(Box::new(PostgresDisplayIdCounterDeleter::new(pool.clone())));
    registry.register(Box::new(PostgresRoleDeleter::new(pool.clone())));
    registry.register(Box::new(PostgresUserDeleter::new(pool.clone())));

    // delete_all が FK 制約に違反せず完了すること
    let report = registry.delete_all(&tenant_id).await;
    assert!(
        !report.has_failures(),
        "削除に失敗した Deleter: {:?}",
        report.failed
    );

    assert_eq!(report.succeeded["postgres:workflows"].deleted_count, 3); // step + instance + definition
    assert_eq!(report.succeeded["auth:credentials"].deleted_count, 1);
    assert_eq!(
        report.succeeded["postgres:display_id_counters"].deleted_count,
        1
    );
    assert_eq!(report.succeeded["postgres:roles"].deleted_count, 1);
    assert_eq!(report.succeeded["postgres:users"].deleted_count, 1);

    // 全テーブルが 0 件
    let counts = registry.count_all(&tenant_id).await.unwrap();
    for (name, count) in &counts {
        assert_eq!(*count, 0, "{} に残存データあり", name);
    }
}
