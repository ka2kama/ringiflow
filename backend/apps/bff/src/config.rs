//! # BFF 設定
//!
//! 環境変数から BFF サーバーの設定を読み込む。

use std::env;

/// BFF サーバーの設定
#[derive(Debug, Clone)]
pub struct BffConfig {
   /// バインドアドレス
   pub host:         String,
   /// ポート番号
   pub port:         u16,
   /// Redis 接続 URL
   pub redis_url:    String,
   /// Core API の URL
   pub core_api_url: String,
}

impl BffConfig {
   /// 環境変数から設定を読み込む
   pub fn from_env() -> Result<Self, env::VarError> {
      Ok(Self {
         host:         env::var("BFF_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
         port:         env::var("BFF_PORT")
            .expect("BFF_PORT が設定されていません（just setup-env を実行してください）")
            .parse()
            .expect("BFF_PORT は有効なポート番号である必要があります"),
         redis_url:    env::var("REDIS_URL")
            .expect("REDIS_URL が設定されていません（just setup-env を実行してください）"),
         core_api_url: env::var("CORE_API_URL")
            .expect("CORE_API_URL が設定されていません（just setup-env を実行してください）"),
      })
   }
}
