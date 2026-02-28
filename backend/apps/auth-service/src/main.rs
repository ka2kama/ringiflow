//! # Auth Service サーバー
//!
//! 認証処理を担当する内部 API サーバー。
//!
//! ## 役割
//!
//! Auth Service は認証ドメインを専門的に担当する:
//!
//! - **パスワード認証**: credentials テーブルを使用したパスワード検証
//! - **認証情報管理**: パスワード、将来の TOTP/OIDC/SAML 認証情報の CRUD
//! - **タイミング攻撃対策**: ユーザー存在確認を防ぐためのダミー検証
//!
//! ## アクセス制御
//!
//! Auth Service は内部ネットワークからのみアクセス可能とする。
//! 外部からのリクエストは BFF を経由する必要がある。
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │   Internet   │──X──│ Auth Service │     │   Database   │
//! └──────────────┘     └──────────────┘     └──────────────┘
//!                             ↑
//!                      内部ネットワークのみ
//!                             ↓
//!                      ┌──────────────┐
//!                      │     BFF      │
//!                      └──────────────┘
//! ```
//!
//! ## 環境変数
//!
//! | 変数名 | 必須 | 説明 |
//! |--------|------|------|
//! | `AUTH_HOST` | No | バインドアドレス（デフォルト: `0.0.0.0`） |
//! | `AUTH_PORT` | **Yes** | ポート番号 |
//! | `DATABASE_URL` | **Yes** | PostgreSQL 接続 URL |
//!
//! ## 起動方法
//!
//! ```bash
//! # 開発環境
//! cargo run -p ringiflow-auth-service
//!
//! # 本番環境
//! AUTH_PORT=13002 DATABASE_URL=postgres://... cargo run -p ringiflow-auth-service --release
//! ```
//!
//! ## 設計詳細
//!
//! → [Auth Service 設計](../../../../docs/40_詳細設計書/08_AuthService設計.md)

mod config;
mod error;
mod handler;
mod usecase;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    routing::{delete, get, post},
};
use config::AuthConfig;
use handler::{
    AuthState,
    ReadinessState,
    create_credentials,
    delete_credentials,
    health_check,
    readiness_check,
    verify,
};
use ringiflow_infra::{
    Argon2PasswordChecker,
    PasswordChecker,
    db,
    repository::{CredentialsRepository, PostgresCredentialsRepository},
};
use ringiflow_shared::{
    canonical_log::CanonicalLogLineLayer,
    observability::{TracingConfig, make_request_span},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use usecase::AuthUseCaseImpl;

/// Auth Service サーバーのエントリーポイント
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env ファイルを読み込む（存在する場合）
    dotenvy::dotenv().ok();

    // トレーシング初期化
    let tracing_config = TracingConfig::from_env("auth-service");
    ringiflow_shared::observability::init_tracing(tracing_config);
    let _tracing_guard = tracing::info_span!("app", service = "auth-service").entered();

    // 設定読み込み
    let config = AuthConfig::from_env().expect("設定の読み込みに失敗しました");

    tracing::info!(
        "Auth Service サーバーを起動します: {}:{}",
        config.host,
        config.port
    );

    // データベース接続プールを作成
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("データベース接続に失敗しました");
    tracing::info!("データベースに接続しました");

    // マイグレーション実行
    db::run_migrations(&pool)
        .await
        .expect("マイグレーションの実行に失敗しました");
    tracing::info!("マイグレーションを適用しました");

    // Readiness Check 用 State（pool が move される前に clone）
    let readiness_state = Arc::new(ReadinessState { pool: pool.clone() });

    // 依存コンポーネントを初期化
    let credentials_repo: Arc<dyn CredentialsRepository> =
        Arc::new(PostgresCredentialsRepository::new(pool));
    let password_checker: Arc<dyn PasswordChecker> = Arc::new(Argon2PasswordChecker::new());
    let auth_usecase = AuthUseCaseImpl::new(credentials_repo, password_checker);
    let auth_state = Arc::new(AuthState {
        usecase: Arc::new(auth_usecase),
    });

    // ルーター構築
    let app = Router::new()
        .route("/health", get(health_check))
        .merge(
            Router::new()
                .route("/health/ready", get(readiness_check))
                .with_state(readiness_state),
        )
        .route("/internal/auth/verify", post(verify))
        .route("/internal/auth/credentials", post(create_credentials))
        .route(
            "/internal/auth/credentials/{tenant_id}/{user_id}",
            delete(delete_credentials),
        )
        .with_state(auth_state)
        .layer(CanonicalLogLineLayer)
        .layer(TraceLayer::new_for_http().make_span_with(make_request_span));

    // jscpd:ignore-start — サーバー起動パターン（意図的な重複）
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("アドレスのパースに失敗しました");

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Auth Service サーバーが起動しました: {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
    // jscpd:ignore-end
}
