//! # 通知送信
//!
//! メール通知の送信を担当するインフラストラクチャモジュール。
//!
//! ## 設計方針
//!
//! - **trait による抽象化**: `NotificationSender` trait でメール送信を抽象化
//! - **3 つの実装**: SMTP（Mailpit 開発用）、SES（本番用）、Noop（テスト用）
//! - **環境変数切替**: `NOTIFICATION_BACKEND` でランタイム選択
//!
//! → 詳細設計: `docs/40_詳細設計書/16_通知機能設計.md`

mod noop;
mod ses;
mod smtp;

use async_trait::async_trait;
pub use noop::NoopNotificationSender;
use ringiflow_domain::notification::{EmailMessage, NotificationError};
pub use ses::SesNotificationSender;
pub use smtp::SmtpNotificationSender;

/// メール送信トレイト
///
/// 通知基盤の中核。メール送信の具体的な方法を抽象化する。
/// SMTP / SES / Noop の 3 実装を環境変数で切り替える。
#[async_trait]
pub trait NotificationSender: Send + Sync {
    /// メールを送信する
    async fn send_email(&self, email: &EmailMessage) -> Result<(), NotificationError>;
}
