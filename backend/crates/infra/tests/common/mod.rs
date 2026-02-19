//! テスト共通フィクスチャ
//!
//! DB を使用する統合テストで共通利用するシードデータ定数・
//! エンティティ生成ヘルパー。 Rust の統合テスト規約に従い `tests/common/mod.rs`
//! に配置。

// 各テストファイルが独立したクレートとしてコンパイルされるため、
// 使用しない関数に dead_code 警告が出る。モジュール全体で抑制する。
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayNumber, Version},
    workflow::{
        CommentBody,
        NewWorkflowComment,
        NewWorkflowInstance,
        NewWorkflowStep,
        StepDecision,
        WorkflowComment,
        WorkflowCommentId,
        WorkflowDefinitionId,
        WorkflowInstance,
        WorkflowInstanceId,
        WorkflowInstanceStatus,
        WorkflowStep,
        WorkflowStepId,
        WorkflowStepStatus,
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
use uuid::Uuid;

// =============================================================================
// シードデータ定数
// =============================================================================

/// シードデータのテナント ID
pub fn seed_tenant_id() -> TenantId {
    TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap())
}

/// シードデータのユーザー ID
pub fn seed_user_id() -> UserId {
    UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap())
}

/// シードデータのワークフロー定義 ID
pub fn seed_definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap())
}

/// テスト用の固定日時
pub fn test_now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}

// =============================================================================
// エンティティ生成ヘルパー
// =============================================================================

/// デフォルト値で WorkflowInstance を作成
pub fn create_test_instance(display_number: i64) -> WorkflowInstance {
    WorkflowInstance::new(NewWorkflowInstance {
        id: WorkflowInstanceId::new(),
        tenant_id: seed_tenant_id(),
        definition_id: seed_definition_id(),
        definition_version: Version::initial(),
        display_number: DisplayNumber::new(display_number).unwrap(),
        title: "テスト申請".to_string(),
        form_data: json!({}),
        initiated_by: seed_user_id(),
        now: test_now(),
    })
}

/// デフォルト値で WorkflowComment を作成
pub fn create_test_comment(
    instance_id: &WorkflowInstanceId,
    posted_by: &UserId,
    body: &str,
) -> WorkflowComment {
    WorkflowComment::new(NewWorkflowComment {
        id:          WorkflowCommentId::new(),
        tenant_id:   seed_tenant_id(),
        instance_id: instance_id.clone(),
        posted_by:   posted_by.clone(),
        body:        CommentBody::new(body).unwrap(),
        now:         test_now(),
    })
}

/// デフォルト値で WorkflowStep を作成
pub fn create_test_step(instance_id: &WorkflowInstanceId, display_number: i64) -> WorkflowStep {
    WorkflowStep::new(NewWorkflowStep {
        id: WorkflowStepId::new(),
        instance_id: instance_id.clone(),
        display_number: DisplayNumber::new(display_number).unwrap(),
        step_id: "step1".to_string(),
        step_name: "承認".to_string(),
        step_type: "approval".to_string(),
        assigned_to: Some(seed_user_id()),
        now: test_now(),
    })
}

// =============================================================================
// DB セットアップヘルパー
// =============================================================================

/// テスト用のテナントとユーザーを DB に作成
pub async fn setup_test_data(pool: &PgPool) -> (TenantId, UserId) {
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let user_id = UserId::from_uuid(Uuid::now_v7());

    // テナント作成
    sqlx::query!(
        r#"
        INSERT INTO tenants (id, name, subdomain, plan, status)
        VALUES ($1, 'Test Tenant', 'test', 'free', 'active')
        "#,
        tenant_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("テナント作成に失敗");

    // ユーザー作成
    sqlx::query!(
        r#"
        INSERT INTO users (id, tenant_id, display_number, email, name, status)
        VALUES ($1, $2, 1, 'test@example.com', 'Test User', 'active')
        "#,
        user_id.as_uuid(),
        tenant_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("ユーザー作成に失敗");

    (tenant_id, user_id)
}

/// 別テナントを作成（テナント分離テスト用）
pub async fn create_other_tenant(pool: &PgPool) -> TenantId {
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    sqlx::query!(
        r#"
        INSERT INTO tenants (id, name, subdomain, plan, status)
        VALUES ($1, 'Other Tenant', 'other', 'free', 'active')
        "#,
        tenant_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("別テナント作成に失敗");
    tenant_id
}

/// テスト用ユーザーを直接 SQL で挿入（リポジトリを経由せずにシードデータを作成する場合）
pub async fn insert_user_raw(
    pool: &PgPool,
    tenant_id: &TenantId,
    display_number: i64,
    email: &str,
    name: &str,
    status: &str,
) -> UserId {
    let user_id = UserId::from_uuid(Uuid::now_v7());
    sqlx::query!(
        r#"
        INSERT INTO users (id, tenant_id, display_number, email, name, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id.as_uuid(),
        tenant_id.as_uuid(),
        display_number,
        email,
        name,
        status,
    )
    .execute(pool)
    .await
    .expect("ユーザー挿入に失敗");
    user_id
}

/// ロールをユーザーに割り当て
pub async fn assign_role(pool: &PgPool, user_id: &UserId, tenant_id: &TenantId) {
    sqlx::query!(
        r#"
        INSERT INTO user_roles (user_id, role_id, tenant_id)
        SELECT $1, id, $2 FROM roles WHERE name = 'user' AND is_system = true
        "#,
        user_id.as_uuid(),
        tenant_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("ロール割り当てに失敗");
}

// =============================================================================
// 不変条件チェック
// =============================================================================

/// ワークフローの不変条件を検証する。
///
/// 統合テストの末尾で呼び出すことで、全ユースケースの不変条件遵守を保証する。
/// 新しいユースケースのテストでもこの関数を呼び出すだけで不変条件チェックが追加される。
/// 不変条件が増えた場合、この関数に追加するだけで全テストに反映される。
///
/// 検証する不変条件:
/// - INV-I1〜I4: WorkflowInstance の不変条件
/// - INV-S1〜S4: WorkflowStep の不変条件
/// - INV-X1〜X3: クロスエンティティ不変条件
///
/// 参照: `docs/03_詳細設計書/エンティティ影響マップ/`
pub async fn assert_workflow_invariants(
    pool: &PgPool,
    instance_id: &WorkflowInstanceId,
    tenant_id: &TenantId,
) {
    let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
    let step_repo = PostgresWorkflowStepRepository::new(pool.clone());

    let instance = instance_repo
        .find_by_id(instance_id, tenant_id)
        .await
        .expect("不変条件チェック: DB エラー")
        .expect("不変条件チェック: Instance が見つからない");

    let steps = step_repo
        .find_by_instance(instance_id, tenant_id)
        .await
        .expect("不変条件チェック: DB エラー");

    // === Instance 不変条件 ===

    // INV-I1: status=Approved ⇒ completed_at IS NOT NULL
    if instance.status() == WorkflowInstanceStatus::Approved {
        assert!(
            instance.completed_at().is_some(),
            "INV-I1 violated: Approved instance must have completed_at"
        );
    }

    // INV-I2: status=Rejected ⇒ completed_at IS NOT NULL
    if instance.status() == WorkflowInstanceStatus::Rejected {
        assert!(
            instance.completed_at().is_some(),
            "INV-I2 violated: Rejected instance must have completed_at"
        );
    }

    // INV-I3: status=InProgress ⇒ current_step_id IS NOT NULL
    if instance.status() == WorkflowInstanceStatus::InProgress {
        assert!(
            instance.current_step_id().is_some(),
            "INV-I3 violated: InProgress instance must have current_step_id"
        );
    }

    // INV-I4: status=Draft ⇒ submitted_at IS NULL
    if instance.status() == WorkflowInstanceStatus::Draft {
        assert!(
            instance.submitted_at().is_none(),
            "INV-I4 violated: Draft instance must not have submitted_at"
        );
    }

    // === Step 不変条件 ===

    // INV-S1: 同一 Instance 内で Active なステップは最大1つ
    let active_count = steps
        .iter()
        .filter(|s| s.status() == WorkflowStepStatus::Active)
        .count();
    assert!(
        active_count <= 1,
        "INV-S1 violated: found {} Active steps, expected at most 1",
        active_count
    );

    for step in &steps {
        // INV-S2: status=Completed ⇒ decision IS NOT NULL
        if step.status() == WorkflowStepStatus::Completed {
            assert!(
                step.decision().is_some(),
                "INV-S2 violated: Completed step {} must have decision",
                step.id()
            );
        }

        // INV-S3: status=Completed ⇒ completed_at IS NOT NULL
        if step.status() == WorkflowStepStatus::Completed {
            assert!(
                step.completed_at().is_some(),
                "INV-S3 violated: Completed step {} must have completed_at",
                step.id()
            );
        }

        // INV-S4: status=Active ⇒ started_at IS NOT NULL
        if step.status() == WorkflowStepStatus::Active {
            assert!(
                step.started_at().is_some(),
                "INV-S4 violated: Active step {} must have started_at",
                step.id()
            );
        }
    }

    // === クロスエンティティ不変条件 ===

    // INV-X1: Instance.status=Approved ⇒ 最終 Completed ステップの decision=Approved
    if instance.status() == WorkflowInstanceStatus::Approved && !steps.is_empty() {
        let last_completed = steps
            .iter()
            .rfind(|s| s.status() == WorkflowStepStatus::Completed);
        if let Some(last) = last_completed {
            assert_eq!(
                last.decision(),
                Some(StepDecision::Approved),
                "INV-X1 violated: last completed step of Approved instance must have Approved decision"
            );
        }
    }

    // INV-X2: Instance.status=Rejected ⇒ いずれかのステップの decision=Rejected
    if instance.status() == WorkflowInstanceStatus::Rejected && !steps.is_empty() {
        let has_rejected = steps
            .iter()
            .any(|s| s.decision() == Some(StepDecision::Rejected));
        assert!(
            has_rejected,
            "INV-X2 violated: Rejected instance must have at least one Rejected step"
        );
    }

    // INV-X3: Instance.status=InProgress ⇒ Steps が1つ以上存在
    if instance.status() == WorkflowInstanceStatus::InProgress {
        assert!(
            !steps.is_empty(),
            "INV-X3 violated: InProgress instance must have at least one step"
        );
    }
}
