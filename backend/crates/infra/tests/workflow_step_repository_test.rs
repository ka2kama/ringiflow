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

use common::{
    assert_workflow_invariants,
    create_test_instance,
    create_test_step,
    seed_tenant_id,
    seed_user_id,
    test_now,
};
use ringiflow_domain::{
    tenant::TenantId,
    value_objects::{DisplayNumber, Version},
    workflow::{StepDecision, WorkflowInstance, WorkflowInstanceId, WorkflowStepId},
};
use ringiflow_infra::{
    db::{PgTransactionManager, TransactionManager},
    repository::{
        PostgresWorkflowInstanceRepository,
        PostgresWorkflowStepRepository,
        WorkflowInstanceRepository,
        WorkflowStepRepository,
    },
};
use sqlx::PgPool;

struct StepTestContext {
    pool:       PgPool,
    sut:        PostgresWorkflowStepRepository,
    instance:   WorkflowInstance,
    tenant_id:  TenantId,
    tx_manager: PgTransactionManager,
}

/// リポジトリ初期化 + インスタンス INSERT の共通セットアップ
async fn setup_repos_with_instance(pool: PgPool, display_number: i64) -> StepTestContext {
    let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
    let sut = PostgresWorkflowStepRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool.clone());
    let tenant_id = seed_tenant_id();
    let instance = create_test_instance(display_number);

    let mut tx = tx_manager.begin().await.unwrap();
    instance_repo.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    StepTestContext {
        pool,
        sut,
        instance,
        tenant_id,
        tx_manager,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_insert_で新規ステップを作成できる(pool: PgPool) {
    let ctx = setup_repos_with_instance(pool, 100).await;

    let step = create_test_step(ctx.instance.id(), 1);

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    let result = ctx.sut.insert(&mut tx, &step, &ctx.tenant_id).await;
    tx.commit().await.unwrap();

    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_でステップを取得できる(pool: PgPool) {
    let ctx = setup_repos_with_instance(pool, 100).await;

    let step = create_test_step(ctx.instance.id(), 1);
    let step_id = step.id().clone();

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let result = ctx.sut.find_by_id(&step_id, &ctx.tenant_id).await;

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
    let ctx = setup_repos_with_instance(pool, 100).await;
    let instance_id = ctx.instance.id().clone();

    let step1 = create_test_step(&instance_id, 1);
    let step2 = create_test_step(&instance_id, 2);

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step1, &ctx.tenant_id)
        .await
        .unwrap();
    ctx.sut
        .insert(&mut tx, &step2, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let result = ctx.sut.find_by_instance(&instance_id, &ctx.tenant_id).await;

    assert!(result.is_ok());
    let steps = result.unwrap();
    assert_eq!(steps.len(), 2);

    assert_workflow_invariants(&ctx.pool, ctx.instance.id(), &ctx.tenant_id).await;
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
    let ctx = setup_repos_with_instance(pool, 100).await;
    let user_id = seed_user_id();

    let step = create_test_step(ctx.instance.id(), 1);

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let result = ctx.sut.find_by_assigned_to(&ctx.tenant_id, &user_id).await;

    assert!(result.is_ok());
    let steps = result.unwrap();
    assert!(!steps.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン一致で更新できる(pool: PgPool) {
    let ctx = setup_repos_with_instance(pool, 100).await;
    let now = test_now();

    let step = create_test_step(ctx.instance.id(), 1);
    let step_id = step.id().clone();
    let expected_version = step.version();

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // アクティブ化（バージョンインクリメント）
    let activated_step = step.activated(now);

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    let result = ctx
        .sut
        .update_with_version_check(&mut tx, &activated_step, expected_version, &ctx.tenant_id)
        .await;
    tx.commit().await.unwrap();

    assert!(result.is_ok());

    let found = ctx
        .sut
        .find_by_id(&step_id, &ctx.tenant_id)
        .await
        .unwrap()
        .unwrap();
    assert!(found.started_at().is_some());

    assert_workflow_invariants(&ctx.pool, ctx.instance.id(), &ctx.tenant_id).await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン不一致でconflictエラーを返す(
    pool: PgPool,
) {
    let ctx = setup_repos_with_instance(pool, 100).await;
    let now = test_now();

    let step = create_test_step(ctx.instance.id(), 1);

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // アクティブ化（バージョンインクリメント）
    let activated_step = step.activated(now);

    // 不一致バージョン（version 2）で更新を試みる
    let wrong_version = Version::initial().next();
    let mut tx = ctx.tx_manager.begin().await.unwrap();
    let result = ctx
        .sut
        .update_with_version_check(&mut tx, &activated_step, wrong_version, &ctx.tenant_id)
        .await;
    tx.commit().await.unwrap();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err.kind(), ringiflow_infra::InfraErrorKind::Conflict { .. }),
        "InfraError::Conflict を期待したが {:?} が返った",
        err
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_別テナントのステップは更新できない(
    pool: PgPool,
) {
    let ctx = setup_repos_with_instance(pool, 100).await;
    let now = test_now();

    let step = create_test_step(ctx.instance.id(), 1);
    let expected_version = step.version();

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // アクティブ化
    let activated_step = step.activated(now);

    // 別テナントで更新を試みる → Conflict エラー
    let other_tenant_id = TenantId::new();
    let mut tx = ctx.tx_manager.begin().await.unwrap();
    let result = ctx
        .sut
        .update_with_version_check(&mut tx, &activated_step, expected_version, &other_tenant_id)
        .await;
    tx.commit().await.unwrap();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err.kind(), ringiflow_infra::InfraErrorKind::Conflict { .. }),
        "InfraError::Conflict を期待したが {:?} が返った",
        err
    );
}

// ============================================================================
// find_by_display_number テスト
// ============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_存在するdisplay_numberで検索できる(pool: PgPool) {
    let ctx = setup_repos_with_instance(pool, 100).await;
    let instance_id = ctx.instance.id().clone();

    let step = create_test_step(&instance_id, 1);
    let step_id = step.id().clone();

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let display_number = DisplayNumber::new(1).unwrap();
    let result = ctx
        .sut
        .find_by_display_number(display_number, &instance_id, &ctx.tenant_id)
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
    let ctx = setup_repos_with_instance(pool, 100).await;
    let instance_id = ctx.instance.id().clone();

    let display_number = DisplayNumber::new(999).unwrap();
    let result = ctx
        .sut
        .find_by_display_number(display_number, &instance_id, &ctx.tenant_id)
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_別のinstance_idでは見つからない(pool: PgPool) {
    let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
    let ctx = setup_repos_with_instance(pool, 100).await;
    let instance_a_id = ctx.instance.id().clone();

    let instance_b = create_test_instance(101);
    let instance_b_id = instance_b.id().clone();

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    instance_repo.insert(&mut tx, &instance_b).await.unwrap();
    tx.commit().await.unwrap();

    // インスタンス A にステップを作成
    let step = create_test_step(&instance_a_id, 1);

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // インスタンス B の display_number: 1 を検索 → 見つからないはず
    let display_number = DisplayNumber::new(1).unwrap();
    let result = ctx
        .sut
        .find_by_display_number(display_number, &instance_b_id, &ctx.tenant_id)
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ステップを完了できる(pool: PgPool) {
    let ctx = setup_repos_with_instance(pool, 100).await;
    let now = test_now();

    let step = create_test_step(ctx.instance.id(), 1);
    let step_id = step.id().clone();
    let v1 = step.version();

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .insert(&mut tx, &step, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // ステップをアクティブ化
    let active_step = step.activated(now);
    let v2 = active_step.version();

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .update_with_version_check(&mut tx, &active_step, v1, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // ステップを完了
    let completed_step = active_step
        .completed(StepDecision::Approved, Some("承認します".to_string()), now)
        .unwrap();

    let mut tx = ctx.tx_manager.begin().await.unwrap();
    ctx.sut
        .update_with_version_check(&mut tx, &completed_step, v2, &ctx.tenant_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // 確認
    let result = ctx.sut.find_by_id(&step_id, &ctx.tenant_id).await;
    assert!(result.is_ok());
    let found = result.unwrap().unwrap();
    assert!(found.completed_at().is_some());
    assert_eq!(found.decision(), Some(StepDecision::Approved));
    assert_eq!(found.comment(), Some("承認します"));

    assert_workflow_invariants(&ctx.pool, ctx.instance.id(), &ctx.tenant_id).await;
}
