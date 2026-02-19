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

mod common;

use std::collections::HashSet;

use common::{
    assert_workflow_invariants,
    create_test_instance,
    seed_tenant_id,
    seed_user_id,
    test_now,
};
use ringiflow_domain::{
    tenant::TenantId,
    value_objects::{DisplayNumber, Version},
    workflow::WorkflowInstanceId,
};
use ringiflow_infra::{
    db::{PgTransactionManager, TransactionManager},
    repository::{PostgresWorkflowInstanceRepository, WorkflowInstanceRepository},
};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_insert_で新規インスタンスを作成できる(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);

    let instance = create_test_instance(100);

    let mut tx = tx_manager.begin().await.unwrap();
    let result = sut.insert(&mut tx, &instance).await;
    tx.commit().await.unwrap();

    assert!(result.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_でインスタンスを取得できる(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool.clone());

    let instance = create_test_instance(100);
    let instance_id = instance.id().clone();
    let tenant_id = seed_tenant_id();

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    let result = sut.find_by_id(&instance_id, &tenant_id).await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id(), &instance_id);
    assert_eq!(found.title(), "テスト申請");

    assert_workflow_invariants(&pool, &instance_id, &tenant_id).await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_存在しない場合はnoneを返す(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool);
    let tenant_id = TenantId::new();
    let instance_id = WorkflowInstanceId::new();

    let result = sut.find_by_id(&instance_id, &tenant_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_tenant_テナント内の一覧を取得できる(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);
    let tenant_id = seed_tenant_id();

    let instance1 = create_test_instance(100);
    let instance2 = create_test_instance(101);

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance1).await.unwrap();
    sut.insert(&mut tx, &instance2).await.unwrap();
    tx.commit().await.unwrap();

    let result = sut.find_by_tenant(&tenant_id).await;

    assert!(result.is_ok());
    let instances = result.unwrap();
    assert!(instances.len() >= 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_tenant_別テナントのインスタンスは取得できない(
    pool: PgPool,
) {
    let sut = PostgresWorkflowInstanceRepository::new(pool);
    let other_tenant_id = TenantId::new();

    let result = sut.find_by_tenant(&other_tenant_id).await;

    assert!(result.is_ok());
    let instances = result.unwrap();
    assert!(instances.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_initiated_by_申請者によるインスタンスを取得できる(
    pool: PgPool,
) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);
    let tenant_id = seed_tenant_id();
    let user_id = seed_user_id();

    let instance = create_test_instance(100);

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    let result = sut.find_by_initiated_by(&tenant_id, &user_id).await;

    assert!(result.is_ok());
    let instances = result.unwrap();
    assert!(!instances.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン一致で更新できる(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool.clone());
    let tenant_id = seed_tenant_id();
    let now = test_now();

    let instance = create_test_instance(100);
    let instance_id = instance.id().clone();
    let expected_version = instance.version();

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    // 申請を実行（ステータス変更 + バージョンインクリメント）
    let submitted_instance = instance.submitted(now).unwrap();

    // バージョン一致で更新
    let mut tx = tx_manager.begin().await.unwrap();
    let result = sut
        .update_with_version_check(&mut tx, &submitted_instance, expected_version, &tenant_id)
        .await;
    tx.commit().await.unwrap();

    assert!(result.is_ok());

    // 更新結果を確認
    let found = sut
        .find_by_id(&instance_id, &tenant_id)
        .await
        .unwrap()
        .unwrap();
    assert!(found.submitted_at().is_some());

    assert_workflow_invariants(&pool, &instance_id, &tenant_id).await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_バージョン不一致でconflictエラーを返す(
    pool: PgPool,
) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);
    let tenant_id = seed_tenant_id();
    let now = test_now();

    let instance = create_test_instance(100);

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    // 申請を実行（バージョンインクリメント）
    let submitted_instance = instance.submitted(now).unwrap();

    // 不一致バージョン（version 2）で更新を試みる
    let wrong_version = Version::initial().next();
    let mut tx = tx_manager.begin().await.unwrap();
    let result = sut
        .update_with_version_check(&mut tx, &submitted_instance, wrong_version, &tenant_id)
        .await;
    tx.commit().await.unwrap();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ringiflow_infra::InfraError::Conflict { .. }),
        "InfraError::Conflict を期待したが {:?} が返った",
        err
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_with_version_check_別テナントのインスタンスは更新できない(
    pool: PgPool,
) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);
    let now = test_now();

    let instance = create_test_instance(100);
    let expected_version = instance.version();

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    // 申請を実行（ステータス変更 + バージョンインクリメント）
    let submitted_instance = instance.submitted(now).unwrap();

    // 別テナントの ID で更新を試みる
    let other_tenant_id = TenantId::new();
    let mut tx = tx_manager.begin().await.unwrap();
    let result = sut
        .update_with_version_check(
            &mut tx,
            &submitted_instance,
            expected_version,
            &other_tenant_id,
        )
        .await;
    tx.commit().await.unwrap();

    // tenant_id が一致しないため、rows_affected が 0 → Conflict エラー
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ringiflow_infra::InfraError::Conflict { .. }),
        "InfraError::Conflict を期待したが {:?} が返った",
        err
    );
}

// ===== find_by_ids テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_ids_空のvecを渡すと空のvecが返る(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool);
    let tenant_id = seed_tenant_id();

    let result = sut.find_by_ids(&[], &tenant_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_ids_存在するidを渡すとインスタンスが返る(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);

    let instance1 = create_test_instance(100);
    let instance2 = create_test_instance(101);
    let id1 = instance1.id().clone();
    let id2 = instance2.id().clone();
    let tenant_id = seed_tenant_id();

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance1).await.unwrap();
    sut.insert(&mut tx, &instance2).await.unwrap();
    tx.commit().await.unwrap();

    let result = sut
        .find_by_ids(&[id1.clone(), id2.clone()], &tenant_id)
        .await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert_eq!(found.len(), 2);

    let found_ids: HashSet<String> = found.iter().map(|i| i.id().to_string()).collect();
    assert!(found_ids.contains(&id1.to_string()));
    assert!(found_ids.contains(&id2.to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_ids_存在しないidを含んでも存在するもののみ返る(
    pool: PgPool,
) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);
    let tenant_id = seed_tenant_id();

    let instance = create_test_instance(100);
    let existing_id = instance.id().clone();
    let nonexistent_id = WorkflowInstanceId::new();

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    let result = sut
        .find_by_ids(&[existing_id.clone(), nonexistent_id], &tenant_id)
        .await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id(), &existing_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_ids_テナントidでフィルタされる(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);
    let other_tenant_id = TenantId::new();

    let instance = create_test_instance(100);
    let instance_id = instance.id().clone();

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    // 別のテナント ID で検索
    let result = sut.find_by_ids(&[instance_id], &other_tenant_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

// ===== find_by_display_number テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_存在するdisplay_numberで検索できる(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);
    let tenant_id = seed_tenant_id();
    let display_number = DisplayNumber::new(42).unwrap();

    let instance = create_test_instance(42);
    let instance_id = instance.id().clone();

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    let result = sut.find_by_display_number(display_number, &tenant_id).await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id(), &instance_id);
    assert_eq!(found.display_number(), display_number);
    assert_eq!(found.title(), "テスト申請");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_存在しない場合はnoneを返す(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool);
    let tenant_id = seed_tenant_id();
    let nonexistent_display_number = DisplayNumber::new(99999).unwrap();

    let result = sut
        .find_by_display_number(nonexistent_display_number, &tenant_id)
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_display_number_別テナントでは見つからない(pool: PgPool) {
    let sut = PostgresWorkflowInstanceRepository::new(pool.clone());
    let tx_manager = PgTransactionManager::new(pool);
    let other_tenant_id = TenantId::new();
    let display_number = DisplayNumber::new(42).unwrap();

    let instance = create_test_instance(42);

    let mut tx = tx_manager.begin().await.unwrap();
    sut.insert(&mut tx, &instance).await.unwrap();
    tx.commit().await.unwrap();

    // 別のテナント ID で検索
    let result = sut
        .find_by_display_number(display_number, &other_tenant_id)
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}
