//! WorkflowStepRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test workflow_step_repository_test
//! ```

use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::Version,
   workflow::{
      StepDecision,
      WorkflowDefinitionId,
      WorkflowInstance,
      WorkflowInstanceId,
      WorkflowStep,
      WorkflowStepId,
   },
};
use ringiflow_infra::repository::{
   PostgresWorkflowInstanceRepository,
   PostgresWorkflowStepRepository,
   WorkflowInstanceRepository,
   WorkflowStepRepository,
};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_save_で新規ステップを作成できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // インスタンスを作成
   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({}),
      user_id.clone(),
   );
   instance_repo.save(&instance).await.unwrap();

   // ステップを作成
   let step = WorkflowStep::new(
      instance.id().clone(),
      "step1".to_string(),
      "承認".to_string(),
      "approval".to_string(),
      Some(user_id),
   );

   let result = step_repo.save(&step).await;

   assert!(result.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_でステップを取得できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // インスタンスを作成
   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({}),
      user_id.clone(),
   );
   instance_repo.save(&instance).await.unwrap();

   // ステップを作成
   let step = WorkflowStep::new(
      instance.id().clone(),
      "step1".to_string(),
      "承認".to_string(),
      "approval".to_string(),
      Some(user_id),
   );
   let step_id = step.id().clone();
   step_repo.save(&step).await.unwrap();

   // 検索
   let result = step_repo.find_by_id(&step_id, &tenant_id).await;

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
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::new();
   let step_id = WorkflowStepId::new();

   let result = step_repo.find_by_id(&step_id, &tenant_id).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_instance_インスタンスのステップ一覧を取得できる(
   pool: PgPool,
) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // インスタンスを作成
   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({}),
      user_id.clone(),
   );
   let instance_id = instance.id().clone();
   instance_repo.save(&instance).await.unwrap();

   // 複数のステップを作成
   let step1 = WorkflowStep::new(
      instance_id.clone(),
      "step1".to_string(),
      "承認1".to_string(),
      "approval".to_string(),
      Some(user_id.clone()),
   );
   let step2 = WorkflowStep::new(
      instance_id.clone(),
      "step2".to_string(),
      "承認2".to_string(),
      "approval".to_string(),
      Some(user_id),
   );
   step_repo.save(&step1).await.unwrap();
   step_repo.save(&step2).await.unwrap();

   // 検索
   let result = step_repo.find_by_instance(&instance_id, &tenant_id).await;

   assert!(result.is_ok());
   let steps = result.unwrap();
   assert_eq!(steps.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_instance_別テナントのステップは取得できない(pool: PgPool) {
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let other_tenant_id = TenantId::new();
   let instance_id = WorkflowInstanceId::new();

   let result = step_repo
      .find_by_instance(&instance_id, &other_tenant_id)
      .await;

   assert!(result.is_ok());
   let steps = result.unwrap();
   assert!(steps.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_assigned_to_担当者のタスク一覧を取得できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // インスタンスを作成
   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({}),
      user_id.clone(),
   );
   instance_repo.save(&instance).await.unwrap();

   // ステップを作成
   let step = WorkflowStep::new(
      instance.id().clone(),
      "step1".to_string(),
      "承認".to_string(),
      "approval".to_string(),
      Some(user_id.clone()),
   );
   step_repo.save(&step).await.unwrap();

   // 検索
   let result = step_repo.find_by_assigned_to(&tenant_id, &user_id).await;

   assert!(result.is_ok());
   let steps = result.unwrap();
   assert!(!steps.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_save_で既存ステップを更新できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // インスタンスを作成
   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({}),
      user_id.clone(),
   );
   instance_repo.save(&instance).await.unwrap();

   // ステップを作成
   let step = WorkflowStep::new(
      instance.id().clone(),
      "step1".to_string(),
      "承認".to_string(),
      "approval".to_string(),
      Some(user_id),
   );
   let step_id = step.id().clone();
   step_repo.save(&step).await.unwrap();

   // ステップをアクティブ化
   let activated_step = step.activated();
   step_repo.save(&activated_step).await.unwrap();

   // 確認
   let result = step_repo.find_by_id(&step_id, &tenant_id).await;
   assert!(result.is_ok());
   let found = result.unwrap().unwrap();
   assert!(found.started_at().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ステップを完了できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // インスタンスを作成
   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({}),
      user_id.clone(),
   );
   instance_repo.save(&instance).await.unwrap();

   // ステップを作成してアクティブ化
   let step = WorkflowStep::new(
      instance.id().clone(),
      "step1".to_string(),
      "承認".to_string(),
      "approval".to_string(),
      Some(user_id),
   )
   .activated();
   let step_id = step.id().clone();
   step_repo.save(&step).await.unwrap();

   // ステップを完了
   let completed_step = step
      .completed(StepDecision::Approved, Some("承認します".to_string()))
      .unwrap();
   step_repo.save(&completed_step).await.unwrap();

   // 確認
   let result = step_repo.find_by_id(&step_id, &tenant_id).await;
   assert!(result.is_ok());
   let found = result.unwrap().unwrap();
   assert!(found.completed_at().is_some());
   assert_eq!(found.decision(), Some(StepDecision::Approved));
   assert_eq!(found.comment(), Some("承認します"));
}
