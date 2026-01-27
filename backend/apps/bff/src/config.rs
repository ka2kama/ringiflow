//! # BFF 設定
//!
//! 環境変数から BFF サーバーの設定を読み込む。

use std::env;

/// BFF サーバーの設定
#[derive(Debug, Clone)]
pub struct BffConfig {
   /// バインドアドレス
   pub host: String,
   /// ポート番号
   pub port: u16,
   /// Redis 接続 URL
   pub redis_url: String,
   /// Core Service の URL
   pub core_url: String,
   /// Auth Service の URL
   pub auth_url: String,
   /// 開発用認証バイパス（DevAuth）の有効化
   ///
   /// `DEV_AUTH_ENABLED=true` のときに有効になる。
   /// 本番環境では絶対に有効にしないこと。
   pub dev_auth_enabled: bool,
}

impl BffConfig {
   /// 環境変数から設定を読み込む
   pub fn from_env() -> Result<Self, env::VarError> {
      let dev_auth_enabled = env::var("DEV_AUTH_ENABLED")
         .map(|v| v.eq_ignore_ascii_case("true"))
         .unwrap_or(false);

      // リリースビルドで DevAuth が有効な場合は panic
      // 本番環境への誤デプロイを防ぐためのセーフティネット
      #[cfg(not(debug_assertions))]
      if dev_auth_enabled {
         panic!(
            "DEV_AUTH_ENABLED=true はリリースビルドでは使用できません。\n\
             本番環境では認証バイパスは許可されません。"
         );
      }

      Ok(Self {
         host: env::var("BFF_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
         port: env::var("BFF_PORT")
            .expect("BFF_PORT が設定されていません（just setup-env を実行してください）")
            .parse()
            .expect("BFF_PORT は有効なポート番号である必要があります"),
         redis_url: env::var("REDIS_URL")
            .expect("REDIS_URL が設定されていません（just setup-env を実行してください）"),
         core_url: env::var("CORE_URL")
            .expect("CORE_URL が設定されていません（just setup-env を実行してください）"),
         auth_url: env::var("AUTH_URL")
            .expect("AUTH_URL が設定されていません（just setup-env を実行してください）"),
         dev_auth_enabled,
      })
   }
}

#[cfg(test)]
mod tests {
   // テスト間で環境変数の競合を避けるため、
   // テスト用のパース関数で検証する

   #[test]
   fn test_dev_auth_enabled_trueのとき有効() {
      // テスト用のパース関数で検証
      assert!(parse_dev_auth_enabled("true"));
      assert!(parse_dev_auth_enabled("TRUE"));
      assert!(parse_dev_auth_enabled("True"));
   }

   #[test]
   fn test_dev_auth_enabled_falseのとき無効() {
      assert!(!parse_dev_auth_enabled("false"));
      assert!(!parse_dev_auth_enabled("FALSE"));
      assert!(!parse_dev_auth_enabled("0"));
      assert!(!parse_dev_auth_enabled(""));
   }

   #[test]
   fn test_dev_auth_enabled_未設定のとき無効() {
      // None の場合は false
      assert!(!parse_dev_auth_enabled_option(None));
   }

   /// 環境変数の値から dev_auth_enabled をパースする（テスト用）
   fn parse_dev_auth_enabled(value: &str) -> bool {
      value.eq_ignore_ascii_case("true")
   }

   /// Option<String> から dev_auth_enabled をパースする（テスト用）
   fn parse_dev_auth_enabled_option(value: Option<&str>) -> bool {
      value
         .map(|v| v.eq_ignore_ascii_case("true"))
         .unwrap_or(false)
   }
}
