//! WorkflowCommentRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test workflow_comment_repository_test
//! ```

mod common;

use common::{create_test_comment, create_test_instance, seed_tenant_id, seed_user_id};
use ringiflow_domain::{tenant::TenantId, workflow::WorkflowInstanceId};
use ringiflow_infra::repository::{
   PostgresWorkflowCommentRepository,
   PostgresWorkflowInstanceRepository,
   WorkflowCommentRepository,
   WorkflowInstanceRepository,
};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_insert_で新規コメントを作成できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowCommentRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   instance_repo.insert(&instance).await.unwrap();

   let comment = create_test_comment(instance.id(), &seed_user_id(), "テストコメント");

   let result = sut.insert(&comment, &tenant_id).await;

   assert!(result.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_instance_でコメント一覧を取得できる(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowCommentRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   let comment1 = create_test_comment(&instance_id, &seed_user_id(), "コメント1");
   let comment2 = create_test_comment(&instance_id, &seed_user_id(), "コメント2");
   sut.insert(&comment1, &tenant_id).await.unwrap();
   sut.insert(&comment2, &tenant_id).await.unwrap();

   let result = sut.find_by_instance(&instance_id, &tenant_id).await;

   assert!(result.is_ok());
   let comments = result.unwrap();
   assert_eq!(comments.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_instance_存在しないインスタンスは空ベクターを返す(
   pool: PgPool,
) {
   let sut = PostgresWorkflowCommentRepository::new(pool);
   let tenant_id = seed_tenant_id();
   let nonexistent_id = WorkflowInstanceId::new();

   let result = sut.find_by_instance(&nonexistent_id, &tenant_id).await;

   assert!(result.is_ok());
   let comments = result.unwrap();
   assert!(comments.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_instance_複数コメントが時系列昇順で返る(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowCommentRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   let comment1 = create_test_comment(&instance_id, &seed_user_id(), "最初のコメント");
   let comment2 = create_test_comment(&instance_id, &seed_user_id(), "2番目のコメント");
   let comment3 = create_test_comment(&instance_id, &seed_user_id(), "3番目のコメント");
   sut.insert(&comment1, &tenant_id).await.unwrap();
   sut.insert(&comment2, &tenant_id).await.unwrap();
   sut.insert(&comment3, &tenant_id).await.unwrap();

   let result = sut.find_by_instance(&instance_id, &tenant_id).await;

   let comments = result.unwrap();
   assert_eq!(comments.len(), 3);
   // created_at が ASC 順であることを確認
   assert!(comments[0].created_at() <= comments[1].created_at());
   assert!(comments[1].created_at() <= comments[2].created_at());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_テナント分離_別テナントのコメントは取得できない(pool: PgPool) {
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let sut = PostgresWorkflowCommentRepository::new(pool);
   let tenant_id = seed_tenant_id();

   let instance = create_test_instance(100);
   let instance_id = instance.id().clone();
   instance_repo.insert(&instance).await.unwrap();

   let comment = create_test_comment(&instance_id, &seed_user_id(), "テストコメント");
   sut.insert(&comment, &tenant_id).await.unwrap();

   // 別テナントで検索
   let other_tenant_id = TenantId::new();
   let result = sut.find_by_instance(&instance_id, &other_tenant_id).await;

   assert!(result.is_ok());
   let comments = result.unwrap();
   assert!(comments.is_empty());
}
