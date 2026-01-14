//! # Core API サーバー
//!
//! ビジネスロジックを実行する内部 API サーバー。
//!
//! ## 役割
//!
//! Core API はビジネスロジックの実行とデータの永続化を担当する:
//!
//! - **ビジネスロジック**: ワークフロー実行、承認処理、タスク管理
//! - **データ永続化**: PostgreSQL へのエンティティ保存
//! - **ドメインイベント**: イベント駆動処理のトリガー（将来）
//!
//! ## アクセス制御
//!
//! Core API は内部ネットワークからのみアクセス可能とする。
//! 外部からのリクエストは BFF を経由する必要がある。
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │   Internet   │──X──│   Core API   │     │   Database   │
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
//! | 変数名 | デフォルト | 説明 |
//! |--------|------------|------|
//! | `CORE_API_HOST` | `0.0.0.0` | バインドアドレス |
//! | `CORE_API_PORT` | `3001` | ポート番号 |
//!
//! ## 起動方法
//!
//! ```bash
//! # 開発環境
//! cargo run --bin core_api
//!
//! # 本番環境（BFF と同時に起動）
//! cargo run --bin core_api --release
//! ```
//!
//! ## BFF との違い
//!
//! | 項目 | BFF | Core API |
//! |------|-----|----------|
//! | 目的 | フロントエンド向け API | 内部サービス向け API |
//! | 認証 | JWT セッション管理 | サービス間認証（将来） |
//! | レスポンス | UI 最適化 | 正規化されたデータ |
//! | キャッシュ | Redis キャッシュ | なし（DB 直接アクセス） |

use std::{env, net::SocketAddr};

use axum::{Router, routing::get};
use ringiflow_api::handler::health_check;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Core API サーバーのエントリーポイント
///
/// BFF とは独立した設定（`CORE_API_HOST`, `CORE_API_PORT`）を使用する。
/// 将来的にはデータベース接続やリポジトリの初期化もここで行う。
#[tokio::main]
async fn main() -> anyhow::Result<()> {
   // .env ファイルを読み込む（存在する場合）
   dotenvy::dotenv().ok();

   // トレーシング初期化
   tracing_subscriber::registry()
      .with(
         tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,ringiflow=debug".into()),
      )
      .with(tracing_subscriber::fmt::layer())
      .init();

   // Core API 専用の設定を読み込む
   // BFF とは異なるポート（デフォルト 3001）を使用
   let host = env::var("CORE_API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
   let port: u16 = env::var("CORE_API_PORT")
      .unwrap_or_else(|_| "3001".to_string())
      .parse()
      .unwrap_or(3001);

   tracing::info!("Core API サーバーを起動します: {}:{}", host, port);

   // ルーター構築
   // 将来的にはワークフロー、タスク、ドキュメント関連のルートを追加
   let app = Router::new()
      .route("/health", get(health_check))
      .layer(TraceLayer::new_for_http());

   // サーバー起動
   let addr: SocketAddr = format!("{}:{}", host, port)
      .parse()
      .expect("アドレスのパースに失敗しました");

   let listener = TcpListener::bind(addr).await?;
   tracing::info!("Core API サーバーが起動しました: {}", addr);

   axum::serve(listener, app).await?;

   Ok(())
}
