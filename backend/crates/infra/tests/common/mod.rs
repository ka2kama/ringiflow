//! テスト共通フィクスチャ
//!
//! DB を使用する統合テストで共通利用するシードデータ定数・エンティティ生成ヘルパー。
//! Rust の統合テスト規約に従い `tests/common/mod.rs` に配置。

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
        WorkflowComment,
        WorkflowCommentId,
        WorkflowDefinitionId,
        WorkflowInstance,
        WorkflowInstanceId,
        WorkflowStep,
        WorkflowStepId,
    },
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
