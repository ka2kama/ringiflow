//! # BFF 設定
//!
//! 環境変数から BFF サーバーの設定を読み込む。
//!
//! ## 設計方針
//!
//! [12-Factor App](https://12factor.net/ja/config) の原則に従い、
//! すべての設定を環境変数から読み込む。
//!
//! ## 環境変数一覧
//!
//! | 変数名 | 必須 | デフォルト | 説明 |
//! |--------|------|------------|------|
//! | `BFF_HOST` | No | `0.0.0.0` | バインドアドレス |
//! | `BFF_PORT` | **Yes** | - | ポート番号 |
//! | `REDIS_URL` | **Yes** | - | Redis 接続 URL |
//! | `CORE_API_URL` | **Yes** | - | Core API の URL |

use std::env;

/// BFF サーバーの設定
#[derive(Debug, Clone)]
pub struct BffConfig {
    /// バインドアドレス（例: `0.0.0.0`, `127.0.0.1`）
    pub host: String,
    /// ポート番号（例: `3000`, `13000`）
    pub port: u16,
    /// Redis 接続 URL
    pub redis_url: String,
    /// Core API の URL
    pub core_api_url: String,
}

impl BffConfig {
    /// 環境変数から設定を読み込む
    ///
    /// # Panics
    ///
    /// 必須の環境変数が設定されていない場合はパニックする。
    /// 起動時に設定エラーを明確にするための意図的な設計。
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            host: env::var("BFF_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("BFF_PORT")
                .expect("BFF_PORT が設定されていません（just setup-env を実行してください）")
                .parse()
                .expect("BFF_PORT は有効なポート番号である必要があります"),
            redis_url: env::var("REDIS_URL")
                .expect("REDIS_URL が設定されていません（just setup-env を実行してください）"),
            core_api_url: env::var("CORE_API_URL")
                .expect("CORE_API_URL が設定されていません（just setup-env を実行してください）"),
        })
    }
}
