//! # NotificationLogRepository
//!
//! 通知ログの永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **fire-and-forget ログ**: 送信成功・失敗どちらも記録する
//! - **テナント分離**: RLS + tenant_id で分離
//!
//! → 詳細設計: `docs/03_詳細設計書/16_通知機能設計.md`

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ringiflow_domain::{
    notification::NotificationLogId,
    tenant::TenantId,
    user::UserId,
    workflow::WorkflowInstanceId,
};
use sqlx::PgPool;

use crate::error::InfraError;

/// 通知ログ（リポジトリ INSERT 用データ型）
#[derive(Debug, Clone)]
pub struct NotificationLog {
    pub id: NotificationLogId,
    pub tenant_id: TenantId,
    pub event_type: String,
    pub workflow_instance_id: WorkflowInstanceId,
    pub workflow_title: String,
    pub workflow_display_id: String,
    pub recipient_user_id: UserId,
    pub recipient_email: String,
    pub subject: String,
    pub status: String,
    pub error_message: Option<String>,
    pub sent_at: DateTime<Utc>,
}

/// 通知ログリポジトリトレイト
#[async_trait]
pub trait NotificationLogRepository: Send + Sync {
    /// 通知ログを挿入する
    async fn insert(&self, log: &NotificationLog) -> Result<(), InfraError>;
}

/// PostgreSQL 実装の NotificationLogRepository
#[derive(Debug, Clone)]
pub struct PostgresNotificationLogRepository {
    pool: PgPool,
}

impl PostgresNotificationLogRepository {
    /// 新しいリポジトリインスタンスを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationLogRepository for PostgresNotificationLogRepository {
    #[tracing::instrument(skip_all, level = "debug")]
    async fn insert(&self, log: &NotificationLog) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            INSERT INTO notification_logs (
                id, tenant_id, event_type, workflow_instance_id,
                workflow_title, workflow_display_id,
                recipient_user_id, recipient_email,
                subject, status, error_message, sent_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            log.id.as_uuid(),
            log.tenant_id.as_uuid(),
            log.event_type,
            log.workflow_instance_id.as_uuid(),
            log.workflow_title,
            log.workflow_display_id,
            log.recipient_user_id.as_uuid(),
            log.recipient_email,
            log.subject,
            log.status,
            log.error_message,
            log.sent_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PostgresNotificationLogRepository>();
    }
}
