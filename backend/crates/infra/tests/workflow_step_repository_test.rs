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

use chrono::DateTime;
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::{DisplayNumber, Version},
   workflow::{
      NewWorkflowInstance,
      NewWorkflowStep,
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
async fn test_insert_で新規ステップを作成できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   instance_repo.insert(&instance).await.unwrap();

   // ステップを作成
   let step = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance.id().clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id),
      now,
   });

   let result = step_repo.insert(&step).await;

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
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   instance_repo.insert(&instance).await.unwrap();

   // ステップを作成
   let step = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance.id().clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id),
      now,
   });
   let step_id = step.id().clone();
   step_repo.insert(&step).await.unwrap();

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
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   // 複数のステップを作成
   let step1 = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance_id.clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認1".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id.clone()),
      now,
   });
   let step2 = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance_id.clone(),
      display_number: DisplayNumber::new(2).unwrap(),
      step_id: "step2".to_string(),
      step_name: "承認2".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id),
      now,
   });
   step_repo.insert(&step1).await.unwrap();
   step_repo.insert(&step2).await.unwrap();

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
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   instance_repo.insert(&instance).await.unwrap();

   // ステップを作成
   let step = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance.id().clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id.clone()),
      now,
   });
   step_repo.insert(&step).await.unwrap();

   // 検索
   let result = step_repo.find_by_assigned_to(&tenant_id, &user_id).await;

   assert!(result.is_ok());
   let steps = result.unwrap();
   assert!(!steps.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン一致で更新できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   instance_repo.insert(&instance).await.unwrap();

   // ステップを作成して INSERT
   let step = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance.id().clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id),
      now,
   });
   let step_id = step.id().clone();
   let expected_version = step.version();
   step_repo.insert(&step).await.unwrap();

   // アクティブ化（バージョンインクリメント）
   let activated_step = step.activated(now);

   // バージョン一致で更新
   let result = step_repo
      .update_with_version_check(&activated_step, expected_version)
      .await;

   assert!(result.is_ok());

   // 更新結果を確認
   let found = step_repo
      .find_by_id(&step_id, &tenant_id)
      .await
      .unwrap()
      .unwrap();
   assert!(found.started_at().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン不一致でconflictエラーを返す(
   pool: PgPool,
) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   instance_repo.insert(&instance).await.unwrap();

   // ステップを作成して INSERT
   let step = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance.id().clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id),
      now,
   });
   step_repo.insert(&step).await.unwrap();

   // アクティブ化（バージョンインクリメント）
   let activated_step = step.activated(now);

   // 不一致バージョン（version 2）で更新を試みる
   let wrong_version = Version::initial().next();
   let result = step_repo
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
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   // ステップを作成（display_number: 1）
   let step = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance_id.clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id),
      now,
   });
   let step_id = step.id().clone();
   step_repo.insert(&step).await.unwrap();

   // display_number で検索
   let display_number = DisplayNumber::new(1).unwrap();
   let result = step_repo
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
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成（ステップは作成しない）
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id,
      now,
   });
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   // 存在しない display_number で検索
   let display_number = DisplayNumber::new(999).unwrap();
   let result = step_repo
      .find_by_display_number(display_number, &instance_id, &tenant_id)
      .await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_別のinstance_idでは見つからない(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンス A を作成
   let instance_a = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id: definition_id.clone(),
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "申請A".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   let instance_a_id = instance_a.id().clone();
   instance_repo.insert(&instance_a).await.unwrap();

   // インスタンス B を作成
   let instance_b = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(101).unwrap(),
      title: "申請B".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   let instance_b_id = instance_b.id().clone();
   instance_repo.insert(&instance_b).await.unwrap();

   // インスタンス A にステップを作成（display_number: 1）
   let step = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance_a_id.clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id),
      now,
   });
   step_repo.insert(&step).await.unwrap();

   // インスタンス B の display_number: 1 を検索 → 見つからないはず
   let display_number = DisplayNumber::new(1).unwrap();
   let result = step_repo
      .find_by_display_number(display_number, &instance_b_id, &tenant_id)
      .await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ステップを完了できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool);

   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

   // インスタンスを作成
   let instance = WorkflowInstance::new(NewWorkflowInstance {
      id: WorkflowInstanceId::new(),
      tenant_id: tenant_id.clone(),
      definition_id,
      definition_version: Version::initial(),
      display_number: DisplayNumber::new(100).unwrap(),
      title: "テスト申請".to_string(),
      form_data: json!({}),
      initiated_by: user_id.clone(),
      now,
   });
   instance_repo.insert(&instance).await.unwrap();

   // ステップを作成
   let step = WorkflowStep::new(NewWorkflowStep {
      id: WorkflowStepId::new(),
      instance_id: instance.id().clone(),
      display_number: DisplayNumber::new(1).unwrap(),
      step_id: "step1".to_string(),
      step_name: "承認".to_string(),
      step_type: "approval".to_string(),
      assigned_to: Some(user_id),
      now,
   });
   let step_id = step.id().clone();
   let v1 = step.version();
   step_repo.insert(&step).await.unwrap();

   // ステップをアクティブ化
   let active_step = step.activated(now);
   let v2 = active_step.version();
   step_repo
      .update_with_version_check(&active_step, v1)
      .await
      .unwrap();

   // ステップを完了
   let completed_step = active_step
      .completed(StepDecision::Approved, Some("承認します".to_string()), now)
      .unwrap();
   step_repo
      .update_with_version_check(&completed_step, v2)
      .await
      .unwrap();

   // 確認
   let result = step_repo.find_by_id(&step_id, &tenant_id).await;
   assert!(result.is_ok());
   let found = result.unwrap().unwrap();
   assert!(found.completed_at().is_some());
   assert_eq!(found.decision(), Some(StepDecision::Approved));
   assert_eq!(found.comment(), Some("承認します"));
}
