//! # Core API 設定
//!
//! 環境変数から Core API サーバーの設定を読み込む。
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
//! | `CORE_API_HOST` | No | `0.0.0.0` | バインドアドレス |
//! | `CORE_API_PORT` | **Yes** | - | ポート番号 |
//! | `DATABASE_URL` | **Yes** | - | PostgreSQL 接続 URL |

use std::env;

/// Core API サーバーの設定
#[derive(Debug, Clone)]
pub struct CoreApiConfig {
    /// バインドアドレス（例: `0.0.0.0`, `127.0.0.1`）
    pub host: String,
    /// ポート番号（例: `3001`, `13001`）
    pub port: u16,
    /// PostgreSQL 接続 URL
    pub database_url: String,
}

impl CoreApiConfig {
    /// 環境変数から設定を読み込む
    ///
    /// # Panics
    ///
    /// 必須の環境変数が設定されていない場合はパニックする。
    /// 起動時に設定エラーを明確にするための意図的な設計。
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            host: env::var("CORE_API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("CORE_API_PORT")
                .expect(
                    "CORE_API_PORT が設定されていません（just setup-env を実行してください）",
                )
                .parse()
                .expect("CORE_API_PORT は有効なポート番号である必要があります"),
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL が設定されていません（just setup-env を実行してください）"),
        })
    }
}
