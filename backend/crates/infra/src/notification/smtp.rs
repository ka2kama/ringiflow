//! SMTP 通知送信実装
//!
//! lettre の `AsyncSmtpTransport` を使用してメールを送信する。
//! 開発環境では Mailpit（ローカル SMTP サーバー）に接続する。

use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport,
    AsyncTransport,
    Tokio1Executor,
    message::{Message, MultiPart, SinglePart, header::ContentType},
};
use ringiflow_domain::notification::{EmailMessage, NotificationError};

use super::NotificationSender;

/// SMTP 通知送信
///
/// `lettre::AsyncSmtpTransport<Tokio1Executor>` をラップする。
/// Mailpit（開発）や SMTP リレー（テスト環境）で使用する。
pub struct SmtpNotificationSender {
    transport:    AsyncSmtpTransport<Tokio1Executor>,
    from_address: String,
}

impl SmtpNotificationSender {
    /// 新しい SMTP 送信インスタンスを作成
    ///
    /// # 引数
    ///
    /// - `host`: SMTP サーバーのホスト名（例: "localhost"）
    /// - `port`: SMTP サーバーのポート番号（例: 1025 for Mailpit）
    /// - `from_address`: 送信元メールアドレス
    pub fn new(host: &str, port: u16, from_address: String) -> Self {
        // builder_dangerous: TLS なしで接続（Mailpit 等のローカル SMTP 向け）
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
            .build();

        Self {
            transport,
            from_address,
        }
    }
}

#[async_trait]
impl NotificationSender for SmtpNotificationSender {
    async fn send_email(&self, email: &EmailMessage) -> Result<(), NotificationError> {
        let message =
            Message::builder()
                .from(self.from_address.parse().map_err(|e| {
                    NotificationError::SendFailed(format!("送信元アドレス不正: {e}"))
                })?)
                .to(email
                    .to
                    .parse()
                    .map_err(|e| NotificationError::SendFailed(format!("宛先アドレス不正: {e}")))?)
                .subject(&email.subject)
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(email.text_body.clone()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(email.html_body.clone()),
                        ),
                )
                .map_err(|e| NotificationError::SendFailed(format!("メッセージ構築失敗: {e}")))?;

        self.transport
            .send(message)
            .await
            .map_err(|e| NotificationError::SendFailed(format!("SMTP 送信失敗: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SmtpNotificationSender>();
    }
}
