//! # Core Service サーバー
//!
//! ビジネスロジックを実行する内部サービス。
//!
//! ## 役割
//!
//! Core Service はビジネスロジックの実行とデータの永続化を担当する:
//!
//! - **ビジネスロジック**: ワークフロー実行、承認処理、タスク管理
//! - **データ永続化**: PostgreSQL へのエンティティ保存
//! - **ドメインイベント**: イベント駆動処理のトリガー（将来）
//!
//! ## アクセス制御
//!
//! Core Service は内部ネットワークからのみアクセス可能とする。
//! 外部からのリクエストは BFF を経由する必要がある。
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │   Internet   │──X──│Core Service  │     │   Database   │
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
//! | `CORE_HOST` | No | バインドアドレス（デフォルト: `0.0.0.0`） |
//! | `CORE_PORT` | **Yes** | ポート番号 |
//! | `DATABASE_URL` | **Yes** | PostgreSQL 接続 URL |
//!
//! ## 起動方法
//!
//! ```bash
//! # 開発環境
//! cargo run -p ringiflow-core-service
//!
//! # 本番環境
//! CORE_PORT=3001 DATABASE_URL=postgres://... cargo run -p ringiflow-core-service --release
//! ```
//!
//! ## BFF との違い
//!
//! | 項目 | BFF | Core Service |
//! |------|-----|--------------|
//! | 目的 | フロントエンド向け API | 内部サービス向け API |
//! | 認証 | セッション管理 | サービス間認証（将来） |
//! | レスポンス | UI 最適化 | 正規化されたデータ |
//! | キャッシュ | Redis キャッシュ | なし（DB 直接アクセス） |

mod app_builder;
mod config;
mod error;
mod handler;
mod usecase;

use std::{net::SocketAddr, sync::Arc};

use config::CoreConfig;
use ringiflow_infra::db;
use ringiflow_shared::observability::TracingConfig;
use tokio::net::TcpListener;

/// Core Service サーバーのエントリーポイント
///
/// BFF とは独立した設定（`CORE_HOST`, `CORE_PORT`）を使用する。
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env ファイルを読み込む（存在する場合）
    dotenvy::dotenv().ok();

    // トレーシング初期化
    let tracing_config = TracingConfig::from_env("core-service");
    ringiflow_shared::observability::init_tracing(tracing_config);
    let _tracing_guard = tracing::info_span!("app", service = "core-service").entered();

    // 設定読み込み
    let config = CoreConfig::from_env().expect("設定の読み込みに失敗しました");

    tracing::info!(
        "Core Service サーバーを起動します: {}:{}",
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

    // S3 クライアントの初期化
    let s3_client_inner =
        ringiflow_infra::s3::create_client(config.s3_endpoint_url.as_deref()).await;
    let s3_client: Arc<dyn ringiflow_infra::S3Client> = Arc::new(
        ringiflow_infra::AwsS3Client::new(s3_client_inner, config.s3_bucket_name.clone()),
    );
    tracing::info!("S3 クライアントを初期化しました");

    // アプリケーション構築（DI + ルーター）
    let app = app_builder::build_app(pool, s3_client, &config);

    // jscpd:ignore-start — サーバー起動パターン（意図的な重複）
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("アドレスのパースに失敗しました");

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Core Service サーバーが起動しました: {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
    // jscpd:ignore-end
}
