//! # アプリケーション設定
//!
//! 環境変数からアプリケーション設定を読み込む。
//!
//! ## 設計方針
//!
//! [12-Factor App](https://12factor.net/ja/config) の原則に従い、
//! すべての設定を環境変数から読み込む。これにより:
//!
//! - 環境ごとの設定を変更せずにデプロイ可能
//! - シークレット（DB パスワードなど）をコードに含めない
//! - コンテナ環境での設定注入が容易
//!
//! ## 環境変数一覧
//!
//! | 変数名 | 必須 | デフォルト | 説明 |
//! |--------|------|------------|------|
//! | `BFF_HOST` | No | `0.0.0.0` | BFF サーバーのバインドアドレス |
//! | `BFF_PORT` | No | `3000` | BFF サーバーのポート番号 |
//! | `CORE_API_HOST` | No | `0.0.0.0` | Core API サーバーのバインドアドレス |
//! | `CORE_API_PORT` | No | `3001` | Core API サーバーのポート番号 |
//! | `DATABASE_URL` | **Yes** | - | PostgreSQL 接続 URL |
//! | `REDIS_URL` | No | `redis://localhost:6379` | Redis 接続 URL |
//! | `ENVIRONMENT` | No | `development` | 実行環境（development/staging/production） |
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_api::config::AppConfig;
//!
//! // .env ファイルから読み込み（開発環境）
//! dotenvy::dotenv().ok();
//!
//! // 設定を構築
//! let config = AppConfig::from_env()
//!     .expect("DATABASE_URL が設定されていません");
//!
//! println!("サーバー: {}:{}", config.server.host, config.server.port);
//! ```

use std::env;

/// HTTP サーバー設定
///
/// サーバーのバインドアドレスとポート番号を保持する。
#[derive(Debug, Clone)]
pub struct ServerConfig {
   /// バインドアドレス（例: `0.0.0.0`, `127.0.0.1`）
   pub host: String,
   /// ポート番号（例: `3000`, `8080`）
   pub port: u16,
}

/// データベース接続設定
///
/// PostgreSQL への接続に必要な情報を保持する。
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
   /// 接続 URL（例: `postgres://user:pass@localhost/ringiflow`）
   pub url: String,
}

/// Redis 接続設定
///
/// Redis キャッシュサーバーへの接続に必要な情報を保持する。
#[derive(Debug, Clone)]
pub struct RedisConfig {
   /// 接続 URL（例: `redis://localhost:6379`）
   pub url: String,
}

/// アプリケーション全体の設定
///
/// すべての設定をまとめた構造体。
/// アプリケーション起動時に一度だけ構築し、各コンポーネントに渡す。
///
/// # 設計判断
///
/// - `Clone` 実装: 複数のタスクで設定を共有するため
/// - `Debug` 実装: ログ出力でのデバッグ用（本番ではシークレットに注意）
#[derive(Debug, Clone)]
pub struct AppConfig {
   /// HTTP サーバー設定
   pub server:      ServerConfig,
   /// データベース接続設定
   pub database:    DatabaseConfig,
   /// Redis 接続設定
   pub redis:       RedisConfig,
   /// 実行環境（`development`, `staging`, `production`）
   pub environment: String,
}

impl AppConfig {
   /// 環境変数から設定を読み込む
   ///
   /// 必須の環境変数が設定されていない場合はエラーを返す。
   /// オプションの環境変数はデフォルト値を使用する。
   ///
   /// # 戻り値
   ///
   /// - `Ok(AppConfig)`: 設定の読み込み成功
   /// - `Err(VarError)`: 必須の環境変数が未設定
   ///
   /// # 必須環境変数
   ///
   /// - `DATABASE_URL`: PostgreSQL 接続 URL
   ///
   /// # 例
   ///
   /// ```rust,ignore
   /// // 開発環境: .env ファイルから読み込み
   /// dotenvy::dotenv().ok();
   ///
   /// match AppConfig::from_env() {
   ///     Ok(config) => println!("設定読み込み成功"),
   ///     Err(e) => eprintln!("設定エラー: {}", e),
   /// }
   /// ```
   ///
   /// # 本番環境での注意
   ///
   /// 本番環境では、必須環境変数が設定されていない場合は
   /// アプリケーションを起動せず、明確なエラーメッセージを表示すべきである。
   pub fn from_env() -> Result<Self, env::VarError> {
      Ok(Self {
         server:      ServerConfig {
            host: env::var("BFF_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("BFF_PORT")
               .unwrap_or_else(|_| "3000".to_string())
               .parse()
               .unwrap_or(3000),
         },
         database:    DatabaseConfig {
            url: env::var("DATABASE_URL")?,
         },
         redis:       RedisConfig {
            url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
         },
         environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
      })
   }
}
