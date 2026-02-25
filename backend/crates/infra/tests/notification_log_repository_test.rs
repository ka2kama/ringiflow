//! NotificationLogRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test notification_log_repository_test
//! ```

mod common;

use chrono::Utc;
use common::setup_test_data;
use ringiflow_domain::{
    notification::NotificationLogId,
    tenant::TenantId,
    user::UserId,
    workflow::WorkflowInstanceId,
};
use ringiflow_infra::repository::{
    NotificationLog,
    NotificationLogRepository,
    PostgresNotificationLogRepository,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// テスト用のワークフローインスタンスを DB に作成する
///
/// notification_logs は workflow_instances への FK を持つため、
/// 先にワークフロー定義とインスタンスを作成する必要がある。
async fn create_test_workflow_instance(
    pool: &PgPool,
    tenant_id: &TenantId,
    user_id: &UserId,
) -> WorkflowInstanceId {
    let definition_id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO workflow_definitions (id, tenant_id, name, definition, status, created_by)
        VALUES ($1, $2, 'テスト定義', $3, 'published', $4)
        "#,
        definition_id,
        tenant_id.as_uuid(),
        json!({"steps": []}),
        user_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("ワークフロー定義作成に失敗");

    let instance_id = WorkflowInstanceId::from_uuid(Uuid::now_v7());
    sqlx::query!(
        r#"
        INSERT INTO workflow_instances (id, tenant_id, definition_id, definition_version, title, status, initiated_by, display_number)
        VALUES ($1, $2, $3, 1, 'テスト申請', 'draft', $4, 1)
        "#,
        instance_id.as_uuid(),
        tenant_id.as_uuid(),
        definition_id,
        user_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("ワークフローインスタンス作成に失敗");

    instance_id
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_通知ログを挿入できる(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let instance_id = create_test_workflow_instance(&pool, &tenant_id, &user_id).await;
    let sut = PostgresNotificationLogRepository::new(pool.clone());

    let log = NotificationLog {
        id: NotificationLogId::new(),
        tenant_id: tenant_id.clone(),
        event_type: "approval_request".to_string(),
        workflow_instance_id: instance_id,
        workflow_title: "テスト申請".to_string(),
        workflow_display_id: "WF-0001".to_string(),
        recipient_user_id: user_id,
        recipient_email: "test@example.com".to_string(),
        subject: "[RingiFlow] 承認依頼: テスト申請 WF-0001".to_string(),
        status: "sent".to_string(),
        error_message: None,
        sent_at: Utc::now(),
    };

    let result = sut.insert(&log).await;
    assert!(result.is_ok());

    // 挿入されたデータを直接 SQL で検証
    let row = sqlx::query!(
        r#"SELECT event_type, status, recipient_email FROM notification_logs WHERE id = $1"#,
        log.id.as_uuid()
    )
    .fetch_one(&pool)
    .await
    .expect("挿入されたログが見つからない");

    assert_eq!(row.event_type, "approval_request");
    assert_eq!(row.status, "sent");
    assert_eq!(row.recipient_email, "test@example.com");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_エラーメッセージ付きの通知ログを挿入できる(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let instance_id = create_test_workflow_instance(&pool, &tenant_id, &user_id).await;
    let sut = PostgresNotificationLogRepository::new(pool.clone());

    let log = NotificationLog {
        id: NotificationLogId::new(),
        tenant_id,
        event_type: "approval_request".to_string(),
        workflow_instance_id: instance_id,
        workflow_title: "テスト申請".to_string(),
        workflow_display_id: "WF-0002".to_string(),
        recipient_user_id: user_id,
        recipient_email: "test@example.com".to_string(),
        subject: "[RingiFlow] 承認依頼: テスト申請 WF-0002".to_string(),
        status: "failed".to_string(),
        error_message: Some("SMTP connection refused".to_string()),
        sent_at: Utc::now(),
    };

    let result = sut.insert(&log).await;
    assert!(result.is_ok());

    // エラーメッセージが正しく保存されていることを検証
    let row = sqlx::query!(
        r#"SELECT status, error_message FROM notification_logs WHERE id = $1"#,
        log.id.as_uuid()
    )
    .fetch_one(&pool)
    .await
    .expect("挿入されたログが見つからない");

    assert_eq!(row.status, "failed");
    assert_eq!(
        row.error_message,
        Some("SMTP connection refused".to_string())
    );
}
