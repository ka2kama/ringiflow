//! # Core Service 設定
//!
//! 環境変数から Core Service サーバーの設定を読み込む。

use std::env;

/// Core Service サーバーの設定
#[derive(Debug, Clone)]
pub struct CoreConfig {
   /// バインドアドレス
   pub host:         String,
   /// ポート番号
   pub port:         u16,
   /// データベース接続 URL
   pub database_url: String,
}

impl CoreConfig {
   /// 環境変数から設定を読み込む
   pub fn from_env() -> Result<Self, env::VarError> {
      Ok(Self {
         host:         env::var("CORE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
         port:         env::var("CORE_PORT")
            .expect("CORE_PORT が設定されていません（just setup-env を実行してください）")
            .parse()
            .expect("CORE_PORT は有効なポート番号である必要があります"),
         database_url: env::var("DATABASE_URL")
            .expect("DATABASE_URL が設定されていません（just setup-env を実行してください）"),
      })
   }
}
