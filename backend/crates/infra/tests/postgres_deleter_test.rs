//! PostgreSQL Deleter + Auth Credentials Deleter 統合テスト
//!
//! 実行方法:
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-infra --test postgres_deleter_test
//! ```

mod common;

use common::setup_test_data;
use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::deletion::{
   AuthCredentialsDeleter,
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

   // テナント B を作成
   let tenant_b = TenantId::from_uuid(Uuid::now_v7());
   sqlx::query!(
      "INSERT INTO tenants (id, name, subdomain, plan, status) VALUES ($1, 'Tenant B', 'tenant-b', 'free', 'active')",
      tenant_b.as_uuid()
   )
   .execute(pool)
   .await
   .expect("テナント B 作成に失敗");

   // テナント B にユーザーを作成
   let user_b_id = Uuid::now_v7();
   sqlx::query!(
      "INSERT INTO users (id, tenant_id, display_number, email, name, status) VALUES ($1, $2, 1, 'b@example.com', 'User B', 'active')",
      user_b_id,
      tenant_b.as_uuid()
   )
   .execute(pool)
   .await
   .expect("テナント B ユーザー作成に失敗");

   (tenant_a, tenant_b)
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

   let count = sut.count(&tenant_id).await.unwrap();
   assert_eq!(count, 1);

   let result = sut.delete(&tenant_id).await.unwrap();
   assert_eq!(result.deleted_count, 1);

   let count_after = sut.count(&tenant_id).await.unwrap();
   assert_eq!(count_after, 0);
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

   // count は definitions の件数
   let count = sut.count(&tenant_id).await.unwrap();
   assert_eq!(count, 1);

   // delete は steps + instances + definitions の合計件数
   let result = sut.delete(&tenant_id).await.unwrap();
   assert_eq!(result.deleted_count, 3); // 1 step + 1 instance + 1 definition

   let count_after = sut.count(&tenant_id).await.unwrap();
   assert_eq!(count_after, 0);
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

   let count = sut.count(&tenant_id).await.unwrap();
   assert_eq!(count, 1);

   let result = sut.delete(&tenant_id).await.unwrap();
   assert_eq!(result.deleted_count, 1);

   let count_after = sut.count(&tenant_id).await.unwrap();
   assert_eq!(count_after, 0);
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

   let count = sut.count(&tenant_id).await.unwrap();
   assert_eq!(count, 1);

   let result = sut.delete(&tenant_id).await.unwrap();
   assert_eq!(result.deleted_count, 1);

   let count_after = sut.count(&tenant_id).await.unwrap();
   assert_eq!(count_after, 0);
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
