//! Noop 通知送信実装
//!
//! メールを実際に送信せず、ログ出力のみ行う。
//! テスト環境や通知無効化時に使用する。

use async_trait::async_trait;
use ringiflow_domain::notification::{EmailMessage, NotificationError};

use super::NotificationSender;

/// Noop 通知送信（ログ出力のみ）
#[derive(Debug, Clone)]
pub struct NoopNotificationSender;

#[async_trait]
impl NotificationSender for NoopNotificationSender {
    async fn send_email(&self, email: &EmailMessage) -> Result<(), NotificationError> {
        tracing::info!(
            to = %email.to,
            subject = %email.subject,
            "Noop: メール送信をスキップ"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn send_emailがエラーを返さない() {
        let sender = NoopNotificationSender;
        let email = EmailMessage {
            to:        "test@example.com".to_string(),
            subject:   "テスト件名".to_string(),
            html_body: "<p>テスト</p>".to_string(),
            text_body: "テスト".to_string(),
        };

        let result = sender.send_email(&email).await;
        assert!(result.is_ok());
    }
}
