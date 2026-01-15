//! # Core API 設定
//!
//! 環境変数から Core API サーバーの設定を読み込む。

use std::env;

/// Core API サーバーの設定
#[derive(Debug, Clone)]
pub struct CoreApiConfig {
   /// バインドアドレス
   pub host: String,
   /// ポート番号
   pub port: u16,
}

impl CoreApiConfig {
   /// 環境変数から設定を読み込む
   pub fn from_env() -> Result<Self, env::VarError> {
      Ok(Self {
         host: env::var("CORE_API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
         port: env::var("CORE_API_PORT")
            .expect("CORE_API_PORT が設定されていません（just setup-env を実行してください）")
            .parse()
            .expect("CORE_API_PORT は有効なポート番号である必要があります"),
      })
   }
}
