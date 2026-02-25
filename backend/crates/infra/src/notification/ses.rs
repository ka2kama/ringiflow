//! SES 通知送信実装
//!
//! AWS SES v2 API を使用してメールを送信する。
//! 本番環境で使用する。

use async_trait::async_trait;
use aws_sdk_sesv2::{
    Client,
    types::{Body, Content, Destination, EmailContent, Message},
};
use ringiflow_domain::notification::{EmailMessage, NotificationError};

use super::NotificationSender;

/// SES 通知送信
///
/// `aws_sdk_sesv2::Client` をラップする。
/// 本番環境で AWS SES を通じてメールを送信する。
pub struct SesNotificationSender {
    client:       Client,
    from_address: String,
}

impl SesNotificationSender {
    /// 新しい SES 送信インスタンスを作成
    ///
    /// # 引数
    ///
    /// - `client`: AWS SES v2 クライアント
    /// - `from_address`: 送信元メールアドレス（SES で検証済みであること）
    pub fn new(client: Client, from_address: String) -> Self {
        Self {
            client,
            from_address,
        }
    }
}

#[async_trait]
impl NotificationSender for SesNotificationSender {
    async fn send_email(&self, email: &EmailMessage) -> Result<(), NotificationError> {
        let destination = Destination::builder().to_addresses(&email.to).build();

        let content = EmailContent::builder()
            .simple(
                Message::builder()
                    .subject(
                        Content::builder()
                            .data(&email.subject)
                            .build()
                            .map_err(|e| {
                                NotificationError::SendFailed(format!("件名構築失敗: {e}"))
                            })?,
                    )
                    .body(
                        Body::builder()
                            .html(Content::builder().data(&email.html_body).build().map_err(
                                |e| {
                                    NotificationError::SendFailed(format!("HTML 本文構築失敗: {e}"))
                                },
                            )?)
                            .text(Content::builder().data(&email.text_body).build().map_err(
                                |e| {
                                    NotificationError::SendFailed(format!(
                                        "テキスト本文構築失敗: {e}"
                                    ))
                                },
                            )?)
                            .build(),
                    )
                    .build(),
            )
            .build();

        self.client
            .send_email()
            .from_email_address(&self.from_address)
            .destination(destination)
            .content(content)
            .send()
            .await
            .map_err(|e| NotificationError::SendFailed(format!("SES 送信失敗: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SesNotificationSender>();
    }
}
