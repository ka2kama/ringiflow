//! # BFF (Backend for Frontend) サーバー
//!
//! フロントエンド専用の API サーバー。
//!
//! ## 役割
//!
//! BFF はフロントエンド（Elm アプリケーション）と Core API の間に位置し、
//! 以下の責務を担う:
//!
//! - **認証・セッション管理**: JWT トークンの発行・検証
//! - **レスポンス最適化**: フロントエンドに最適な形式にデータを変換
//! - **アグリゲーション**: 複数の API 呼び出しを 1 つにまとめる
//! - **キャッシュ**: Redis を使用したレスポンスキャッシュ
//!
//! ## アーキテクチャ
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │   Browser    │────▶│     BFF      │────▶│   Core API   │
//! │   (Elm)      │     │  port: 13000 │     │  port: 13001 │
//! └──────────────┘     └──────────────┘     └──────────────┘
//!                             │
//!                             ▼
//!                      ┌──────────────┐
//!                      │    Redis     │
//!                      │   (Cache)    │
//!                      └──────────────┘
//! ```
//!
//! ## 環境変数
//!
//! ポート番号は `.env` ファイルで設定する（`just setup-env` で作成）。
//!
//! | 変数名 | 必須 | 説明 |
//! |--------|------|------|
//! | `BFF_HOST` | No | バインドアドレス（デフォルト: `0.0.0.0`） |
//! | `BFF_PORT` | **Yes** | ポート番号 |
//! | `DATABASE_URL` | **Yes** | PostgreSQL 接続 URL |
//! | `REDIS_URL` | **Yes** | Redis 接続 URL |
//!
//! ## 起動方法
//!
//! ```bash
//! # 開発環境（.env ファイルを使用）
//! cargo run --bin bff
//!
//! # 本番環境（環境変数を直接指定）
//! DATABASE_URL=postgres://... cargo run --bin bff --release
//! ```

use std::net::SocketAddr;

use axum::{Router, routing::get};
use ringiflow_api::{config::AppConfig, handler::health_check};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// BFF サーバーのエントリーポイント
///
/// 以下の順序で初期化を行う:
///
/// 1. 環境変数の読み込み（.env ファイル）
/// 2. トレーシングの初期化
/// 3. アプリケーション設定の読み込み
/// 4. ルーターの構築
/// 5. HTTP サーバーの起動
#[tokio::main]
async fn main() -> anyhow::Result<()> {
   // .env ファイルを読み込む（存在する場合）
   // 本番環境では .env ファイルは使用せず、環境変数を直接設定する
   dotenvy::dotenv().ok();

   // トレーシング初期化
   // RUST_LOG 環境変数でログレベルを制御可能
   // 例: RUST_LOG=debug,tower_http=trace
   tracing_subscriber::registry()
      .with(
         tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,ringiflow=debug".into()),
      )
      .with(tracing_subscriber::fmt::layer())
      .init();

   // 設定読み込み
   // DATABASE_URL が未設定の場合はここでパニックする
   let config = AppConfig::from_env().expect("設定の読み込みに失敗しました");

   tracing::info!(
      "BFF サーバーを起動します: {}:{}",
      config.server.host,
      config.server.port
   );

   // ルーター構築
   // TraceLayer により、すべての HTTP リクエストがトレーシングされる
   let app = Router::new()
      .route("/health", get(health_check))
      .layer(TraceLayer::new_for_http());

   // サーバー起動
   let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
      .parse()
      .expect("アドレスのパースに失敗しました");

   let listener = TcpListener::bind(addr).await?;
   tracing::info!("BFF サーバーが起動しました: {}", addr);

   // Graceful shutdown は axum::serve が自動的に処理する
   axum::serve(listener, app).await?;

   Ok(())
}
