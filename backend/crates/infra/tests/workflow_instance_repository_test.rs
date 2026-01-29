//! WorkflowInstanceRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test workflow_instance_repository_test
//! ```

use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::Version,
   workflow::{WorkflowDefinitionId, WorkflowInstance, WorkflowInstanceId},
};
use ringiflow_infra::repository::{PostgresWorkflowInstanceRepository, WorkflowInstanceRepository};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_insert_で新規インスタンスを作成できる(pool: PgPool) {
   let repo = PostgresWorkflowInstanceRepository::new(pool);
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({"field": "value"}),
      user_id,
   );

   let result = repo.insert(&instance).await;

   assert!(result.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_でインスタンスを取得できる(pool: PgPool) {
   let repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({"field": "value"}),
      user_id,
   );
   let instance_id = instance.id().clone();

   repo.insert(&instance).await.unwrap();

   let result = repo.find_by_id(&instance_id, &tenant_id).await;

   assert!(result.is_ok());
   let found = result.unwrap();
   assert!(found.is_some());
   let found = found.unwrap();
   assert_eq!(found.id(), &instance_id);
   assert_eq!(found.title(), "テスト申請");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_存在しない場合はnoneを返す(pool: PgPool) {
   let repo = PostgresWorkflowInstanceRepository::new(pool);
   let tenant_id = TenantId::new();
   let instance_id = WorkflowInstanceId::new();

   let result = repo.find_by_id(&instance_id, &tenant_id).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_tenant_テナント内の一覧を取得できる(pool: PgPool) {
   let repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   // 2つのインスタンスを作成
   let instance1 = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id.clone(),
      Version::initial(),
      "申請1".to_string(),
      json!({}),
      user_id.clone(),
   );
   let instance2 = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "申請2".to_string(),
      json!({}),
      user_id,
   );

   repo.insert(&instance1).await.unwrap();
   repo.insert(&instance2).await.unwrap();

   let result = repo.find_by_tenant(&tenant_id).await;

   assert!(result.is_ok());
   let instances = result.unwrap();
   assert!(instances.len() >= 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_tenant_別テナントのインスタンスは取得できない(
   pool: PgPool,
) {
   let repo = PostgresWorkflowInstanceRepository::new(pool);
   let other_tenant_id = TenantId::new();

   let result = repo.find_by_tenant(&other_tenant_id).await;

   assert!(result.is_ok());
   let instances = result.unwrap();
   assert!(instances.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_initiated_by_申請者によるインスタンスを取得できる(
   pool: PgPool,
) {
   let repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "自分の申請".to_string(),
      json!({}),
      user_id.clone(),
   );

   repo.insert(&instance).await.unwrap();

   let result = repo.find_by_initiated_by(&tenant_id, &user_id).await;

   assert!(result.is_ok());
   let instances = result.unwrap();
   assert!(!instances.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン一致で更新できる(pool: PgPool) {
   let repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({}),
      user_id,
   );
   let instance_id = instance.id().clone();
   let expected_version = instance.version();

   // INSERT で保存
   repo.insert(&instance).await.unwrap();

   // 申請を実行（ステータス変更 + バージョンインクリメント）
   let submitted_instance = instance.submitted().unwrap();

   // バージョン一致で更新
   let result = repo
      .update_with_version_check(&submitted_instance, expected_version)
      .await;

   assert!(result.is_ok());

   // 更新結果を確認
   let found = repo
      .find_by_id(&instance_id, &tenant_id)
      .await
      .unwrap()
      .unwrap();
   assert!(found.submitted_at().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン不一致でconflictエラーを返す(
   pool: PgPool,
) {
   let repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let tenant_id = TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let definition_id =
      WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());
   let user_id = UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap());

   let instance = WorkflowInstance::new(
      tenant_id.clone(),
      definition_id,
      Version::initial(),
      "テスト申請".to_string(),
      json!({}),
      user_id,
   );

   // INSERT で保存
   repo.insert(&instance).await.unwrap();

   // 申請を実行（バージョンインクリメント）
   let submitted_instance = instance.submitted().unwrap();

   // 不一致バージョン（version 2）で更新を試みる
   let wrong_version = Version::initial().next();
   let result = repo
      .update_with_version_check(&submitted_instance, wrong_version)
      .await;

   assert!(result.is_err());
   let err = result.unwrap_err();
   assert!(
      matches!(err, ringiflow_infra::InfraError::Conflict { .. }),
      "InfraError::Conflict を期待したが {:?} が返った",
      err
   );
}
