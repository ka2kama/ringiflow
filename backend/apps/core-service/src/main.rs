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

mod config;
mod error;
mod handler;
mod usecase;

use std::{net::SocketAddr, sync::Arc};

use axum::{
   Router,
   routing::{get, post},
};
use config::CoreConfig;
use handler::{
   DashboardState,
   TaskState,
   UserState,
   WorkflowState,
   approve_step,
   approve_step_by_display_number,
   create_workflow,
   get_dashboard_stats,
   get_task,
   get_user,
   get_user_by_email,
   get_workflow,
   get_workflow_by_display_number,
   get_workflow_definition,
   health_check,
   list_my_tasks,
   list_my_workflows,
   list_workflow_definitions,
   reject_step,
   reject_step_by_display_number,
   submit_workflow,
   submit_workflow_by_display_number,
};
use ringiflow_infra::{
   db,
   repository::{
      display_id_counter_repository::PostgresDisplayIdCounterRepository,
      user_repository::PostgresUserRepository,
      workflow_definition_repository::PostgresWorkflowDefinitionRepository,
      workflow_instance_repository::PostgresWorkflowInstanceRepository,
      workflow_step_repository::PostgresWorkflowStepRepository,
   },
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use usecase::{DashboardUseCaseImpl, TaskUseCaseImpl, WorkflowUseCaseImpl};

/// Core Service サーバーのエントリーポイント
///
/// BFF とは独立した設定（`CORE_HOST`, `CORE_PORT`）を使用する。
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

   // 依存コンポーネントを初期化
   let user_repository = PostgresUserRepository::new(pool.clone());
   let user_state = Arc::new(UserState { user_repository });

   // ワークフロー関連の依存コンポーネント
   let definition_repo = PostgresWorkflowDefinitionRepository::new(pool.clone());
   let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let step_repo = PostgresWorkflowStepRepository::new(pool.clone());
   let workflow_user_repo = PostgresUserRepository::new(pool.clone());
   let counter_repo = PostgresDisplayIdCounterRepository::new(pool.clone());
   let workflow_usecase = WorkflowUseCaseImpl::new(
      definition_repo,
      instance_repo,
      step_repo,
      workflow_user_repo,
      counter_repo,
   );
   let workflow_state = Arc::new(WorkflowState {
      usecase: workflow_usecase,
   });

   // タスク関連の依存コンポーネント
   let task_instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let task_step_repo = PostgresWorkflowStepRepository::new(pool.clone());
   let task_user_repo = PostgresUserRepository::new(pool.clone());
   let task_usecase = TaskUseCaseImpl::new(task_instance_repo, task_step_repo, task_user_repo);
   let task_state = Arc::new(TaskState {
      usecase: task_usecase,
   });

   // ダッシュボード関連の依存コンポーネント
   let dashboard_instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
   let dashboard_step_repo = PostgresWorkflowStepRepository::new(pool.clone());
   let dashboard_usecase = DashboardUseCaseImpl::new(dashboard_instance_repo, dashboard_step_repo);
   let dashboard_state = Arc::new(DashboardState {
      usecase: dashboard_usecase,
   });

   // ルーター構築
   let app =
      Router::new()
         .route("/health", get(health_check))
         .route(
            "/internal/users/by-email",
            get(get_user_by_email::<PostgresUserRepository>),
         )
         .route(
            "/internal/users/{user_id}",
            get(get_user::<PostgresUserRepository>),
         )
         .with_state(user_state)
         // ワークフロー定義 API
         .route(
            "/internal/workflow-definitions",
            get(
               list_workflow_definitions::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .route(
            "/internal/workflow-definitions/{id}",
            get(
               get_workflow_definition::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         // ワークフローインスタンス API
         .route(
            "/internal/workflows",
            get(
               list_my_workflows::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            )
            .post(
               create_workflow::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .route(
            "/internal/workflows/{id}",
            get(
               get_workflow::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .route(
            "/internal/workflows/{id}/submit",
            post(
               submit_workflow::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .route(
            "/internal/workflows/{id}/steps/{step_id}/approve",
            post(
               approve_step::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .route(
            "/internal/workflows/{id}/steps/{step_id}/reject",
            post(
               reject_step::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         // display_number 対応 API
         .route(
            "/internal/workflows/by-display-number/{display_number}",
            get(
               get_workflow_by_display_number::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .route(
            "/internal/workflows/by-display-number/{display_number}/submit",
            post(
               submit_workflow_by_display_number::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .route(
            "/internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/approve",
            post(
               approve_step_by_display_number::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .route(
            "/internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/reject",
            post(
               reject_step_by_display_number::<
                  PostgresWorkflowDefinitionRepository,
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
                  PostgresDisplayIdCounterRepository,
               >,
            ),
         )
         .with_state(workflow_state)
         // タスク API
         .route(
            "/internal/tasks/my",
            get(
               list_my_tasks::<
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
               >,
            ),
         )
         .route(
            "/internal/tasks/{id}",
            get(
               get_task::<
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
                  PostgresUserRepository,
               >,
            ),
         )
         .with_state(task_state)
         // ダッシュボード API
         .route(
            "/internal/dashboard/stats",
            get(
               get_dashboard_stats::<
                  PostgresWorkflowInstanceRepository,
                  PostgresWorkflowStepRepository,
               >,
            ),
         )
         .with_state(dashboard_state)
         .layer(TraceLayer::new_for_http());

   // サーバー起動
   let addr: SocketAddr = format!("{}:{}", config.host, config.port)
      .parse()
      .expect("アドレスのパースに失敗しました");

   let listener = TcpListener::bind(addr).await?;
   tracing::info!("Core Service サーバーが起動しました: {}", addr);

   axum::serve(listener, app).await?;

   Ok(())
}
