//! WorkflowStepRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test workflow_sutsitory_test
//! ```

mod common;

use common::{create_test_instance, create_test_step, seed_tenant_id, seed_user_id, test_now};
use ringiflow_domain::{
   tenant::TenantId,
   value_objects::{DisplayNumber, Version},
   workflow::{StepDecision, WorkflowInstanceId, WorkflowStepId},
};
use ringiflow_infra::repository::{
   PostgresWorkflowInstanceRepository,
   PostgresWorkflowStepRepository,
   WorkflowInstanceRepository,
   WorkflowStepRepository,
};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_insert_で新規ステップを作成できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   instance_repo.insert(&instance).await.unwrap();

   let step = create_test_step(instance.id(), 1);

   let result = sut.insert(&step, &tenant_id).await;

   assert!(result.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_でステップを取得できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   instance_repo.insert(&instance).await.unwrap();

   let step = create_test_step(instance.id(), 1);
   let step_id = step.id().clone();
   sut.insert(&step, &tenant_id).await.unwrap();

   let result = sut.find_by_id(&step_id, &tenant_id).await;

   assert!(result.is_ok());
   let found = result.unwrap();
   assert!(found.is_some());
   let found = found.unwrap();
   assert_eq!(found.id(), &step_id);
   assert_eq!(found.step_id(), "step1");
   assert_eq!(found.step_name(), "承認");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_存在しない場合はnoneを返す(pool: PgPool) {
   let sut = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::new();
   let step_id = WorkflowStepId::new();

   let result = sut.find_by_id(&step_id, &tenant_id).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_instance_インスタンスのステップ一覧を取得できる(
   pool: PgPool,
) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   let step1 = create_test_step(&instance_id, 1);
   let step2 = create_test_step(&instance_id, 2);
   sut.insert(&step1, &tenant_id).await.unwrap();
   sut.insert(&step2, &tenant_id).await.unwrap();

   let result = sut.find_by_instance(&instance_id, &tenant_id).await;

   assert!(result.is_ok());
   let steps = result.unwrap();
   assert_eq!(steps.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_instance_別テナントのステップは取得できない(pool: PgPool) {
   let sut = PostgresWorkflowStepRepository::new(pool);

   let other_tenant_id = TenantId::new();
   let instance_id = WorkflowInstanceId::new();

   let result = sut.find_by_instance(&instance_id, &other_tenant_id).await;

   assert!(result.is_ok());
   let steps = result.unwrap();
   assert!(steps.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_assigned_to_担当者のタスク一覧を取得できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();
   let user_id = seed_user_id();

   let instance = create_test_instance(100);
   instance_repo.insert(&instance).await.unwrap();

   let step = create_test_step(instance.id(), 1);
   sut.insert(&step, &tenant_id).await.unwrap();

   let result = sut.find_by_assigned_to(&tenant_id, &user_id).await;

   assert!(result.is_ok());
   let steps = result.unwrap();
   assert!(!steps.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン一致で更新できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();
   let now = test_now();

   let instance = create_test_instance(100);
   instance_repo.insert(&instance).await.unwrap();

   let step = create_test_step(instance.id(), 1);
   let step_id = step.id().clone();
   let expected_version = step.version();
   sut.insert(&step, &tenant_id).await.unwrap();

   // アクティブ化（バージョンインクリメント）
   let activated_step = step.activated(now);

   let result = sut
      .update_with_version_check(&activated_step, expected_version)
      .await;

   assert!(result.is_ok());

   let found = sut.find_by_id(&step_id, &tenant_id).await.unwrap().unwrap();
   assert!(found.started_at().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン不一致でconflictエラーを返す(
   pool: PgPool,
) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();
   let now = test_now();

   let instance = create_test_instance(100);
   instance_repo.insert(&instance).await.unwrap();

   let step = create_test_step(instance.id(), 1);
   sut.insert(&step, &tenant_id).await.unwrap();

   // アクティブ化（バージョンインクリメント）
   let activated_step = step.activated(now);

   // 不一致バージョン（version 2）で更新を試みる
   let wrong_version = Version::initial().next();
   let result = sut
      .update_with_version_check(&activated_step, wrong_version)
      .await;

   assert!(result.is_err());
   let err = result.unwrap_err();
   assert!(
      matches!(err, ringiflow_infra::InfraError::Conflict { .. }),
      "InfraError::Conflict を期待したが {:?} が返った",
      err
   );
}

// ============================================================================
// find_by_display_number テスト
// ============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_存在するdisplay_numberで検索できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   let step = create_test_step(&instance_id, 1);
   let step_id = step.id().clone();
   sut.insert(&step, &tenant_id).await.unwrap();

   let display_number = DisplayNumber::new(1).unwrap();
   let result = sut
      .find_by_display_number(display_number, &instance_id, &tenant_id)
      .await;

   assert!(result.is_ok());
   let found = result.unwrap();
   assert!(found.is_some());
   let found = found.unwrap();
   assert_eq!(found.id(), &step_id);
   assert_eq!(found.display_number().as_i64(), 1);
   assert_eq!(found.step_id(), "step1");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_存在しない場合はnoneを返す(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   let display_number = DisplayNumber::new(999).unwrap();
   let result = sut
      .find_by_display_number(display_number, &instance_id, &tenant_id)
      .await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_別のinstance_idでは見つからない(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance_a = create_test_instance(100);
   let instance_a_id = instance_a.id().clone();
   instance_repo.insert(&instance_a).await.unwrap();

   let instance_b = create_test_instance(101);
   let instance_b_id = instance_b.id().clone();
   instance_repo.insert(&instance_b).await.unwrap();

   // インスタンス A にステップを作成
   let step = create_test_step(&instance_a_id, 1);
   sut.insert(&step, &tenant_id).await.unwrap();

   // インスタンス B の display_number: 1 を検索 → 見つからないはず
   let display_number = DisplayNumber::new(1).unwrap();
   let result = sut
      .find_by_display_number(display_number, &instance_b_id, &tenant_id)
      .await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ステップを完了できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowStepRepository::new(pool);
   let tenant_id = seed_tenant_id();
   let now = test_now();

   let instance = create_test_instance(100);
   instance_repo.insert(&instance).await.unwrap();

   let step = create_test_step(instance.id(), 1);
   let step_id = step.id().clone();
   let v1 = step.version();
   sut.insert(&step, &tenant_id).await.unwrap();

   // ステップをアクティブ化
   let active_step = step.activated(now);
   let v2 = active_step.version();
   sut.update_with_version_check(&active_step, v1)
      .await
      .unwrap();

   // ステップを完了
   let completed_step = active_step
      .completed(StepDecision::Approved, Some("承認します".to_string()), now)
      .unwrap();
   sut.update_with_version_check(&completed_step, v2)
      .await
      .unwrap();

   // 確認
   let result = sut.find_by_id(&step_id, &tenant_id).await;
   assert!(result.is_ok());
   let found = result.unwrap().unwrap();
   assert!(found.completed_at().is_some());
   assert_eq!(found.decision(), Some(StepDecision::Approved));
   assert_eq!(found.comment(), Some("承認します"));
}
