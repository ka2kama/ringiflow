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

mod app_builder;
mod config;

use std::{net::SocketAddr, sync::Arc};

use config::BffConfig;
#[cfg(feature = "dev-auth")]
use ringiflow_bff::dev_auth;
use ringiflow_bff::handler::ReadinessState;
use ringiflow_infra::{
    RedisSessionManager,
    SessionManager,
    dynamodb,
    redis,
    repository::DynamoDbAuditLogRepository,
};
use ringiflow_shared::observability::TracingConfig;
use tokio::net::TcpListener;

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
    let tracing_config = TracingConfig::from_env("bff");
    ringiflow_shared::observability::init_tracing(tracing_config);
    let _tracing_guard = tracing::info_span!("app", service = "bff").entered();

    // 設定読み込み
    let config = BffConfig::from_env().expect("設定の読み込みに失敗しました");

    tracing::info!("BFF サーバーを起動します: {}:{}", config.host, config.port);

    // 依存関係の初期化
    let redis_session_manager = RedisSessionManager::new(&config.redis_url)
        .await
        .expect("Redis への接続に失敗しました");

    // Readiness Check 用の Redis 接続（SessionManager とは別の接続）
    let readiness_redis_conn = redis::create_connection_manager(&config.redis_url)
        .await
        .expect("Redis への接続に失敗しました（readiness check 用）");
    let readiness_state = Arc::new(ReadinessState {
        redis_conn:       readiness_redis_conn,
        core_service_url: config.core_url.clone(),
        http_client:      reqwest::Client::new(),
    });

    // DevAuth の初期化（dev-auth feature 有効時のみコンパイルされる）
    #[cfg(feature = "dev-auth")]
    if config.dev_auth_enabled {
        tracing::warn!("========================================");
        tracing::warn!("⚠️  DevAuth が有効です！");
        tracing::warn!("   本番環境では絶対に有効にしないでください");
        tracing::warn!("========================================");

        match dev_auth::setup_dev_session(&redis_session_manager).await {
            Ok(csrf_token) => {
                tracing::info!("DevAuth: 開発用セッションを作成しました");
                tracing::info!("  Tenant ID: {}", dev_auth::DEV_TENANT_ID);
                tracing::info!("  User ID: {}", dev_auth::DEV_USER_ID);
                tracing::info!("  Session ID: {}", dev_auth::DEV_SESSION_ID);
                tracing::info!("  CSRF Token: {}...", &csrf_token[..8]);
            }
            Err(e) => {
                tracing::error!("DevAuth: セッション作成に失敗しました: {}", e);
            }
        }

        // セッション TTL 経過後もデモ環境で認証が維持されるよう、定期更新タスクを起動
        dev_auth::spawn_dev_session_refresh(redis_session_manager.clone());
    }

    let session_manager: Arc<dyn SessionManager> = Arc::new(redis_session_manager);

    // DynamoDB クライアントの初期化
    let dynamodb_client = dynamodb::create_client(&config.dynamodb_endpoint).await;
    dynamodb::ensure_audit_log_table(&dynamodb_client, "audit_logs")
        .await
        .expect("DynamoDB 監査ログテーブルのセットアップに失敗しました");
    let audit_log_repository = Arc::new(DynamoDbAuditLogRepository::new(
        dynamodb_client,
        "audit_logs".to_string(),
    ));

    // アプリケーション構築（DI + ルーター）
    let app = app_builder::build_app(
        &config,
        session_manager,
        readiness_state,
        audit_log_repository,
    );

    // jscpd:ignore-start — サーバー起動パターン（意図的な重複）
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("アドレスのパースに失敗しました");

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("BFF サーバーが起動しました: {}", addr);

    // Graceful shutdown は axum::serve が自動的に処理する
    axum::serve(listener, app).await?;

    Ok(())
    // jscpd:ignore-end
}
