//! DisplayIdCounterRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test display_id_counter_repository_test
//! ```

use ringiflow_domain::{
   tenant::TenantId,
   value_objects::{DisplayIdEntityType, DisplayNumber},
};
use ringiflow_infra::repository::{DisplayIdCounterRepository, PostgresDisplayIdCounterRepository};
use sqlx::PgPool;

/// テスト用: カウンター行を挿入する
async fn insert_counter(pool: &PgPool, tenant_id: &TenantId, entity_type: &str, last_number: i64) {
   sqlx::query!(
      r#"
      INSERT INTO display_id_counters (tenant_id, entity_type, last_number)
      VALUES ($1, $2, $3)
      ON CONFLICT (tenant_id, entity_type) DO UPDATE SET last_number = $3
      "#,
      tenant_id.as_uuid(),
      entity_type,
      last_number,
   )
   .execute(pool)
   .await
   .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_初回採番で1を返す(pool: PgPool) {
   let repo = PostgresDisplayIdCounterRepository::new(pool.clone());
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // workflow_step のカウンターを初期化（last_number=0）
   insert_counter(
      &pool,
      &tenant_id,
      DisplayIdEntityType::WorkflowStep.as_str(),
      0,
   )
   .await;

   let result = repo
      .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
      .await;

   assert_eq!(result.unwrap(), DisplayNumber::new(1).unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_連続採番で連番を返す(pool: PgPool) {
   let repo = PostgresDisplayIdCounterRepository::new(pool.clone());
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // workflow_step のカウンターを初期化（last_number=0）
   insert_counter(
      &pool,
      &tenant_id,
      DisplayIdEntityType::WorkflowStep.as_str(),
      0,
   )
   .await;

   let num1 = repo
      .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
      .await
      .unwrap();
   let num2 = repo
      .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
      .await
      .unwrap();
   let num3 = repo
      .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
      .await
      .unwrap();

   assert_eq!(num1, DisplayNumber::new(1).unwrap());
   assert_eq!(num2, DisplayNumber::new(2).unwrap());
   assert_eq!(num3, DisplayNumber::new(3).unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_異なるエンティティ型は独立して採番される(pool: PgPool) {
   let repo = PostgresDisplayIdCounterRepository::new(pool.clone());
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // workflow_step のカウンターを初期化（last_number=0）
   // workflow_instance はマイグレーションで既に初期化済み
   insert_counter(
      &pool,
      &tenant_id,
      DisplayIdEntityType::WorkflowStep.as_str(),
      0,
   )
   .await;

   // workflow_step の採番（初回 → 1）
   let step_num = repo
      .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
      .await
      .unwrap();

   // workflow_instance の採番（シードデータの次の番号）
   let instance_num = repo
      .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowInstance)
      .await
      .unwrap();

   // workflow_step は 1（workflow_instance の値に影響されない）
   assert_eq!(step_num, DisplayNumber::new(1).unwrap());
   // workflow_instance はシードデータの次の番号（1 より大きい）
   assert!(instance_num.as_i64() > 1);
}
