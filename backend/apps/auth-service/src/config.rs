//! # Auth Service 設定
//!
//! 環境変数から Auth Service サーバーの設定を読み込む。

use std::env;

/// Auth Service サーバーの設定
#[derive(Debug, Clone)]
pub struct AuthConfig {
   /// バインドアドレス
   pub host:         String,
   /// ポート番号
   pub port:         u16,
   /// データベース接続 URL
   pub database_url: String,
}

impl AuthConfig {
   /// 環境変数から設定を読み込む
   pub fn from_env() -> Result<Self, env::VarError> {
      Ok(Self {
         host:         env::var("AUTH_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
         port:         env::var("AUTH_PORT")
            .expect("AUTH_PORT が設定されていません（just setup-env を実行してください）")
            .parse()
            .expect("AUTH_PORT は有効なポート番号である必要があります"),
         database_url: env::var("DATABASE_URL")
            .expect("DATABASE_URL が設定されていません（just setup-env を実行してください）"),
      })
   }
}
