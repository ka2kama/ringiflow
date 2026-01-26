//! # BFF (Backend for Frontend) サーバー
//!
//! フロントエンド専用の API サーバー。
//!
//! ## 役割
//!
//! BFF はフロントエンド（Elm アプリケーション）と Core API の間に位置し、
//! 以下の責務を担う:
//!
//! - **認証・セッション管理**: HTTPOnly Cookie によるセッション管理
//! - **CSRF 防御**: 状態変更リクエストの保護
//! - **レスポンス最適化**: フロントエンドに最適な形式にデータを変換
//! - **アグリゲーション**: 複数の API 呼び出しを 1 つにまとめる
//! - **キャッシュ**: Redis を使用したレスポンスキャッシュ
//!
//! ## アーキテクチャ
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │   Browser    │────▶│     BFF      │────▶│Core Service  │
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
//! | `REDIS_URL` | **Yes** | Redis 接続 URL |
//! | `CORE_URL` | **Yes** | Core Service の URL |
//! | `AUTH_URL` | **Yes** | Auth Service の URL |
//! | `DEV_AUTH_ENABLED` | No | 開発用認証バイパスの有効化（`true` で有効） |
//!
//! ## 起動方法
//!
//! ```bash
//! # 開発環境（.env ファイルを使用）
//! cargo run -p ringiflow-bff
//!
//! # 本番環境（環境変数を直接指定）
//! BFF_PORT=3000 REDIS_URL=redis://... cargo run -p ringiflow-bff --release
//! ```

pub mod client;
mod config;
mod dev_auth;
mod error;
pub mod handler;
pub mod middleware;

use std::{net::SocketAddr, sync::Arc};

use axum::{
   Router,
   middleware::from_fn_with_state,
   routing::{get, post},
};
use client::{AuthServiceClientImpl, CoreServiceClientImpl};
use config::BffConfig;
use handler::{
   AuthState,
   WorkflowState,
   create_workflow,
   csrf,
   health_check,
   login,
   logout,
   me,
   submit_workflow,
};
use middleware::{CsrfState, csrf_middleware};
use ringiflow_infra::RedisSessionManager;
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
   let config = BffConfig::from_env().expect("設定の読み込みに失敗しました");

   tracing::info!("BFF サーバーを起動します: {}:{}", config.host, config.port);

   // 依存関係の初期化
   let session_manager = RedisSessionManager::new(&config.redis_url)
      .await
      .expect("Redis への接続に失敗しました");
   let core_service_client = CoreServiceClientImpl::new(&config.core_url);
   let auth_service_client = AuthServiceClientImpl::new(&config.auth_url);

   // DevAuth の初期化（開発環境のみ）
   if config.dev_auth_enabled {
      tracing::warn!("========================================");
      tracing::warn!("⚠️  DevAuth が有効です！");
      tracing::warn!("   本番環境では絶対に有効にしないでください");
      tracing::warn!("========================================");

      match dev_auth::setup_dev_session(&session_manager).await {
         Ok(csrf_token) => {
            tracing::info!("DevAuth: 開発用セッションを作成しました");
            tracing::info!("  Tenant ID: {}", dev_auth::DEV_TENANT_ID);
            tracing::info!("  User ID: {}", dev_auth::DEV_USER_ID);
            tracing::info!("  Session ID: {}", dev_auth::DEV_SESSION_ID);
            tracing::info!("  CSRF Token: {}", csrf_token);
         }
         Err(e) => {
            tracing::error!("DevAuth: セッション作成に失敗しました: {}", e);
         }
      }
   }

   // CSRF ミドルウェア用の状態
   let csrf_state = CsrfState {
      session_manager: session_manager.clone(),
   };

   let auth_state = Arc::new(AuthState {
      core_service_client: core_service_client.clone(),
      auth_service_client,
      session_manager: session_manager.clone(),
   });

   let workflow_state = Arc::new(WorkflowState {
      core_service_client,
      session_manager,
   });

   // ルーター構築
   // TraceLayer により、すべての HTTP リクエストがトレーシングされる
   // CSRF ミドルウェアは POST/PUT/PATCH/DELETE リクエストを検証する
   let app = Router::new()
      .route("/health", get(health_check))
      .route(
         "/auth/login",
         post(login::<CoreServiceClientImpl, AuthServiceClientImpl, RedisSessionManager>),
      )
      .route(
         "/auth/logout",
         post(logout::<CoreServiceClientImpl, AuthServiceClientImpl, RedisSessionManager>),
      )
      .route(
         "/auth/me",
         get(me::<CoreServiceClientImpl, AuthServiceClientImpl, RedisSessionManager>),
      )
      .route(
         "/auth/csrf",
         get(csrf::<CoreServiceClientImpl, AuthServiceClientImpl, RedisSessionManager>),
      )
      .with_state(auth_state)
      .route(
         "/api/v1/workflows",
         post(create_workflow::<CoreServiceClientImpl, RedisSessionManager>),
      )
      .route(
         "/api/v1/workflows/:id/submit",
         post(submit_workflow::<CoreServiceClientImpl, RedisSessionManager>),
      )
      .with_state(workflow_state)
      .layer(from_fn_with_state(
         csrf_state,
         csrf_middleware::<RedisSessionManager>,
      ))
      .layer(TraceLayer::new_for_http());

   // サーバー起動
   let addr: SocketAddr = format!("{}:{}", config.host, config.port)
      .parse()
      .expect("アドレスのパースに失敗しました");

   let listener = TcpListener::bind(addr).await?;
   tracing::info!("BFF サーバーが起動しました: {}", addr);

   // Graceful shutdown は axum::serve が自動的に処理する
   axum::serve(listener, app).await?;

   Ok(())
}
