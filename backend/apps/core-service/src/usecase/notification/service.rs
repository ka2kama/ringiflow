//! # 通知サービス
//!
//! テンプレートレンダリング → メール送信 → ログ記録を統合するサービス。
//!
//! ## 設計方針
//!
//! - **fire-and-forget**: `notify()` は送信失敗してもエラーを返さない
//! - **ログ記録**: 成功・失敗どちらも `notification_logs` テーブルに記録
//! - **依存性注入**: `NotificationSender` と `NotificationLogRepository` は trait で抽象化

use std::sync::Arc;

use chrono::Utc;
use ringiflow_domain::{
    notification::{NotificationLogId, WorkflowNotification},
    tenant::TenantId,
    workflow::WorkflowInstanceId,
};
use ringiflow_infra::{
    notification::NotificationSender,
    repository::{NotificationLog, NotificationLogRepository},
};
use ringiflow_shared::{event_log::event, log_business_event};

use super::TemplateRenderer;

/// 通知サービス
///
/// ワークフロー操作に伴うメール通知の全体フローを統合する。
/// `notify()` は fire-and-forget で、送信失敗してもエラーを返さない。
pub struct NotificationService {
    sender: Arc<dyn NotificationSender>,
    template_renderer: TemplateRenderer,
    log_repo: Arc<dyn NotificationLogRepository>,
    base_url: String,
}

impl NotificationService {
    pub fn new(
        sender: Arc<dyn NotificationSender>,
        template_renderer: TemplateRenderer,
        log_repo: Arc<dyn NotificationLogRepository>,
        base_url: String,
    ) -> Self {
        Self {
            sender,
            template_renderer,
            log_repo,
            base_url,
        }
    }

    /// 通知を送信する（fire-and-forget）
    ///
    /// テンプレートレンダリング → メール送信 → ログ記録を行う。
    /// いずれのステップで失敗してもエラーを返さない（ログ出力のみ）。
    pub async fn notify(
        &self,
        notification: WorkflowNotification,
        tenant_id: &TenantId,
        workflow_instance_id: &WorkflowInstanceId,
    ) {
        let event_type = notification.event_type();
        let event_type_str: &str = event_type.into();
        let workflow_title = notification.workflow_title().to_string();
        let workflow_display_id = notification.workflow_display_id().to_string();
        let recipient_user_id = notification.recipient_user_id().clone();
        let recipient_email = notification.recipient_email().to_string();

        // テンプレートレンダリング
        let email = match self.template_renderer.render(&notification, &self.base_url) {
            Ok(email) => email,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    event_type = event_type_str,
                    "通知テンプレートのレンダリングに失敗"
                );
                return;
            }
        };

        let subject = email.subject.clone();

        // メール送信
        let (status, error_message) = match self.sender.send_email(&email).await {
            Ok(()) => {
                log_business_event!(
                    event.category = event::category::NOTIFICATION,
                    event.action = event::action::NOTIFICATION_SENT,
                    event.tenant_id = %tenant_id,
                    event.entity_type = event::entity_type::NOTIFICATION_LOG,
                    event.result = event::result::SUCCESS,
                    notification.event_type = event_type_str,
                    notification.recipient = %recipient_email,
                    "通知メール送信成功"
                );
                ("sent".to_string(), None)
            }
            Err(e) => {
                log_business_event!(
                    event.category = event::category::NOTIFICATION,
                    event.action = event::action::NOTIFICATION_FAILED,
                    event.tenant_id = %tenant_id,
                    event.entity_type = event::entity_type::NOTIFICATION_LOG,
                    event.result = event::result::FAILURE,
                    notification.event_type = event_type_str,
                    notification.recipient = %recipient_email,
                    error = %e,
                    "通知メール送信失敗"
                );
                ("failed".to_string(), Some(e.to_string()))
            }
        };

        // 通知ログ記録
        let log = NotificationLog {
            id: NotificationLogId::new(),
            tenant_id: tenant_id.clone(),
            event_type: event_type_str.to_string(),
            workflow_instance_id: workflow_instance_id.clone(),
            workflow_title,
            workflow_display_id,
            recipient_user_id,
            recipient_email,
            subject,
            status,
            error_message,
            sent_at: Utc::now(),
        };

        if let Err(e) = self.log_repo.insert(&log).await {
            tracing::error!(
                error = %e,
                "通知ログの記録に失敗"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use ringiflow_domain::user::UserId;
    use ringiflow_infra::mock::{MockNotificationLogRepository, MockNotificationSender};

    use super::*;

    fn make_service(
        sender: MockNotificationSender,
        log_repo: MockNotificationLogRepository,
    ) -> NotificationService {
        let template_renderer = TemplateRenderer::new().unwrap();
        NotificationService::new(
            Arc::new(sender),
            template_renderer,
            Arc::new(log_repo),
            "http://localhost:5173".to_string(),
        )
    }

    fn make_notification() -> WorkflowNotification {
        WorkflowNotification::ApprovalRequest {
            workflow_title:      "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            applicant_name:      "田中太郎".to_string(),
            step_name:           "上長承認".to_string(),
            approver_email:      "suzuki@example.com".to_string(),
            approver_user_id:    UserId::new(),
        }
    }

    #[tokio::test]
    async fn 送信成功時にlog_repoにstatus_sentで記録する() {
        let sender = MockNotificationSender::new();
        let log_repo = MockNotificationLogRepository::new();
        let service = make_service(sender.clone(), log_repo.clone());

        let tenant_id = TenantId::new();
        let instance_id = WorkflowInstanceId::new();

        service
            .notify(make_notification(), &tenant_id, &instance_id)
            .await;

        let logs = log_repo.logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].status, "sent");
        assert!(logs[0].error_message.is_none());
        assert_eq!(logs[0].event_type, "approval_request");
        assert_eq!(logs[0].recipient_email, "suzuki@example.com");
    }

    #[tokio::test]
    async fn 送信失敗してもエラーを返さない() {
        // MockNotificationSender は常に成功するため、このテストは
        // service.notify() が Result ではなく () を返すことの確認
        let sender = MockNotificationSender::new();
        let log_repo = MockNotificationLogRepository::new();
        let service = make_service(sender, log_repo);

        let tenant_id = TenantId::new();
        let instance_id = WorkflowInstanceId::new();

        // notify() は () を返す（コンパイル時検証）
        service
            .notify(make_notification(), &tenant_id, &instance_id)
            .await;
    }

    #[tokio::test]
    async fn mock_notification_senderが送信メッセージを記録する() {
        let sender = MockNotificationSender::new();
        let log_repo = MockNotificationLogRepository::new();
        let service = make_service(sender.clone(), log_repo);

        let tenant_id = TenantId::new();
        let instance_id = WorkflowInstanceId::new();

        service
            .notify(make_notification(), &tenant_id, &instance_id)
            .await;

        let sent = sender.sent_emails();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "suzuki@example.com");
        assert_eq!(
            sent[0].subject,
            "[RingiFlow] 承認依頼: 経費精算申請 WF-0042"
        );
    }
}
