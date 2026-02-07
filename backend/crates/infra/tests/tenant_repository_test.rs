//! TenantRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test tenant_repository_test
//! ```

mod common;

use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::repository::{PostgresTenantRepository, TenantRepository};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn test_idでテナントを取得できる(pool: PgPool) {
   // テスト用テナントを作成
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   sqlx::query!(
      r#"
        INSERT INTO tenants (id, name, subdomain, plan, status)
        VALUES ($1, 'Test Tenant', 'test-tenant', 'free', 'active')
        "#,
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("テナント作成に失敗");

   let repo = PostgresTenantRepository::new(pool);

   let result = repo.find_by_id(&tenant_id).await;

   assert!(result.is_ok());
   let tenant = result.unwrap();
   assert!(tenant.is_some());
   let tenant = tenant.unwrap();
   assert_eq!(tenant.id(), &tenant_id);
   assert_eq!(tenant.name().as_str(), "Test Tenant");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_存在しないidの場合noneを返す(pool: PgPool) {
   let nonexistent_id = TenantId::from_uuid(Uuid::now_v7());
   let repo = PostgresTenantRepository::new(pool);

   let result = repo.find_by_id(&nonexistent_id).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}
