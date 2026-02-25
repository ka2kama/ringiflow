//! # Core Service 設定
//!
//! 環境変数から Core Service サーバーの設定を読み込む。

use std::env;

/// Core Service サーバーの設定
#[derive(Debug, Clone)]
pub struct CoreConfig {
    /// バインドアドレス
    pub host: String,
    /// ポート番号
    pub port: u16,
    /// データベース接続 URL
    pub database_url: String,
    /// S3 エンドポイント URL（MinIO 使用時に設定、未設定で AWS S3 デフォルト）
    pub s3_endpoint_url: Option<String>,
    /// S3 バケット名
    pub s3_bucket_name: String,
    /// 通知設定
    pub notification: NotificationConfig,
}

/// 通知機能の設定
///
/// `NOTIFICATION_BACKEND` 環境変数で送信バックエンドを切り替える:
/// - `smtp`: Mailpit（開発）/ SMTP サーバー経由で送信
/// - `ses`: Amazon SES v2 経由で送信（本番）
/// - `noop`: 送信しない（ログ出力のみ）
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    /// 送信バックエンド（"smtp" | "ses" | "noop"）
    pub backend:      String,
    /// SMTP ホスト（backend=smtp の場合に使用）
    pub smtp_host:    String,
    /// SMTP ポート（backend=smtp の場合に使用）
    pub smtp_port:    u16,
    /// 送信元メールアドレス
    pub from_address: String,
    /// フロントエンド URL（メール内リンク用）
    pub base_url:     String,
}

impl CoreConfig {
    /// 環境変数から設定を読み込む
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            host: env::var("CORE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("CORE_PORT")
                .expect("CORE_PORT が設定されていません（just setup-env を実行してください）")
                .parse()
                .expect("CORE_PORT は有効なポート番号である必要があります"),
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL が設定されていません（just setup-env を実行してください）"),
            s3_endpoint_url: env::var("S3_ENDPOINT_URL").ok(),
            s3_bucket_name: env::var("S3_BUCKET_NAME")
                .expect("S3_BUCKET_NAME が設定されていません（just setup-env を実行してください）"),
            notification: NotificationConfig::from_env(),
        })
    }
}

impl NotificationConfig {
    /// 環境変数から通知設定を読み込む
    fn from_env() -> Self {
        Self {
            backend:      env::var("NOTIFICATION_BACKEND").unwrap_or_else(|_| "noop".to_string()),
            smtp_host:    env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
            smtp_port:    env::var("SMTP_PORT")
                .unwrap_or_else(|_| "1025".to_string())
                .parse()
                .expect("SMTP_PORT は有効なポート番号である必要があります"),
            from_address: env::var("NOTIFICATION_FROM_ADDRESS")
                .unwrap_or_else(|_| "noreply@ringiflow.example.com".to_string()),
            base_url:     env::var("NOTIFICATION_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
        }
    }
}
