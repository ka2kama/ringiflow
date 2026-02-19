//! トランザクション競合テスト
//!
//! 楽観的ロック（バージョンチェック）が競合を正しく検出し、
//! トランザクションの原子性により不変条件が保持されることを検証する。
//!
//! sqlx::test は単一接続プール（savepoint ベース）のため、tokio::spawn による
//! 真の並行テストは不可能。逐次実行で同等の競合シナリオを検証する。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test transaction_concurrency_test
//! ```

mod common;

use common::{assert_workflow_invariants, create_test_instance, create_test_step, seed_tenant_id};
use ringiflow_infra::{
    InfraError,
    db::{PgTransactionManager, TransactionManager},
    repository::{
        PostgresWorkflowInstanceRepository,
        PostgresWorkflowStepRepository,
        WorkflowInstanceRepository,
        WorkflowStepRepository,
    },
};
use sqlx::PgPool;

/// 楽観的ロックにより、古いバージョンでの更新が Conflict を返す
///
/// シナリオ:
/// 1. Instance(v1), Step(v1) を DB に挿入
/// 2. TX_A: step を更新（v1→v2）+ instance を更新 → コミット成功
/// 3. TX_B: 同じ step を古いバージョン（v1）で更新 → Conflict
/// 4. 不変条件が保持されている
#[sqlx::test(migrations = "../../migrations")]
async fn test_楽観的ロックで古いバージョンの更新がconflictを返す(
    pool: PgPool,
) {
    let tenant_id = seed_tenant_id();
    let now = common::test_now();

    let instance = create_test_instance(200)
        .submitted(now)
        .unwrap()
        .with_current_step("step1".to_string(), now);
    let step = create_test_step(instance.id(), 1).activated(now);

    let instance_id = instance.id().clone();
    let initial_step_version = step.version();
    let initial_instance_version = instance.version();

    let tx_manager = PgTransactionManager::new(pool.clone());
    let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
    let step_repo = PostgresWorkflowStepRepository::new(pool.clone());

    // 初期データを挿入
    let mut tx = tx_manager.begin().await.unwrap();
    instance_repo.insert(&mut tx, &instance).await.unwrap();
    step_repo.insert(&mut tx, &step, &tenant_id).await.unwrap();
    tx.commit().await.unwrap();

    // TX_A: step を承認して更新 → コミット成功（v1→v2）
    let approved_step = step.clone().approve(None, now).unwrap();
    let mut tx_a = tx_manager.begin().await.unwrap();
    step_repo
        .update_with_version_check(&mut tx_a, &approved_step, initial_step_version, &tenant_id)
        .await
        .unwrap();

    let instance_for_update = instance_repo
        .find_by_id(&instance_id, &tenant_id)
        .await
        .unwrap()
        .unwrap();
    let completed_instance = instance_for_update.complete_with_approval(now).unwrap();
    instance_repo
        .update_with_version_check(
            &mut tx_a,
            &completed_instance,
            initial_instance_version,
            &tenant_id,
        )
        .await
        .unwrap();
    tx_a.commit().await.unwrap();

    // TX_B: 同じ step を古いバージョン（v1）で更新 → Conflict
    let rejected_step = step.reject(Some("差し戻し".to_string()), now).unwrap();
    let mut tx_b = tx_manager.begin().await.unwrap();
    let result = step_repo
        .update_with_version_check(
            &mut tx_b,
            &rejected_step,
            initial_step_version, // TX_A でコミット済みなので v2 だが、v1 を期待 → Conflict
            &tenant_id,
        )
        .await;

    assert!(
        matches!(result, Err(InfraError::Conflict { .. })),
        "古いバージョンでの更新は Conflict を返すべき: {:?}",
        result
    );
    // tx_b は drop → 自動ロールバック

    // 不変条件の検証
    assert_workflow_invariants(&pool, &instance_id, &tenant_id).await;
}

/// トランザクション原子性: 途中で Conflict が発生すると全書き込みがロールバックされる
///
/// シナリオ:
/// 1. Instance(v1), Step(v1) を DB に挿入
/// 2. TX_A: step を更新（v1→v2）→ コミット成功
/// 3. TX_B: instance を更新（成功）→ step を古いバージョンで更新 → Conflict → 全ロールバック
/// 4. instance が TX_B の書き込み前の状態に戻っていることを検証
#[sqlx::test(migrations = "../../migrations")]
async fn test_トランザクション原子性で部分更新がロールバックされる(
    pool: PgPool,
) {
    let tenant_id = seed_tenant_id();
    let now = common::test_now();

    let instance = create_test_instance(300)
        .submitted(now)
        .unwrap()
        .with_current_step("step1".to_string(), now);
    let step = create_test_step(instance.id(), 1).activated(now);

    let instance_id = instance.id().clone();
    let initial_step_version = step.version();

    let tx_manager = PgTransactionManager::new(pool.clone());
    let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
    let step_repo = PostgresWorkflowStepRepository::new(pool.clone());

    // 初期データを挿入
    let mut tx = tx_manager.begin().await.unwrap();
    instance_repo.insert(&mut tx, &instance).await.unwrap();
    step_repo.insert(&mut tx, &step, &tenant_id).await.unwrap();
    tx.commit().await.unwrap();

    // TX_A: step のみ更新 → コミット（v1→v2）
    let approved_step = step.clone().approve(None, now).unwrap();
    let mut tx_a = tx_manager.begin().await.unwrap();
    step_repo
        .update_with_version_check(&mut tx_a, &approved_step, initial_step_version, &tenant_id)
        .await
        .unwrap();
    tx_a.commit().await.unwrap();

    // TX_B: instance を更新（成功）→ step を古いバージョンで更新（Conflict）
    // → TX 全体がロールバックされ、instance の更新も取り消される
    let instance_for_update = instance_repo
        .find_by_id(&instance_id, &tenant_id)
        .await
        .unwrap()
        .unwrap();
    let instance_version_before_tx_b = instance_for_update.version();
    let rejected_instance = instance_for_update.complete_with_rejection(now).unwrap();

    let rejected_step = step.reject(Some("却下".to_string()), now).unwrap();

    let mut tx_b = tx_manager.begin().await.unwrap();

    // instance の更新はトランザクション内では成功する
    instance_repo
        .update_with_version_check(
            &mut tx_b,
            &rejected_instance,
            instance_version_before_tx_b,
            &tenant_id,
        )
        .await
        .unwrap();

    // step の更新は Conflict（TX_A がバージョンを上げた）
    let step_result = step_repo
        .update_with_version_check(
            &mut tx_b,
            &rejected_step,
            initial_step_version, // v1 を期待するが、TX_A で v2 に更新済み
            &tenant_id,
        )
        .await;

    assert!(
        matches!(step_result, Err(InfraError::Conflict { .. })),
        "古いバージョンでの更新は Conflict を返すべき: {:?}",
        step_result
    );

    // tx_b を明示的に drop（コミットしない → ロールバック）
    drop(tx_b);

    // 検証: instance は TX_B の更新前の状態に戻っている
    let instance_after = instance_repo
        .find_by_id(&instance_id, &tenant_id)
        .await
        .unwrap()
        .unwrap();

    // TX_B の instance 更新はロールバックされたので、
    // ステータスは InProgress のまま（Rejected にはなっていない）
    assert_eq!(
        instance_after.status(),
        ringiflow_domain::workflow::WorkflowInstanceStatus::InProgress,
        "ロールバックにより instance は InProgress のまま"
    );

    // 不変条件の検証
    assert_workflow_invariants(&pool, &instance_id, &tenant_id).await;
}
