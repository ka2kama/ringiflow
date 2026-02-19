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
    routing::{get, patch, post},
};
use config::CoreConfig;
use handler::{
    DashboardState,
    RoleState,
    TaskState,
    UserState,
    WorkflowState,
    approve_step,
    approve_step_by_display_number,
    create_role,
    create_user,
    create_workflow,
    delete_role,
    get_dashboard_stats,
    get_role,
    get_task,
    get_task_by_display_numbers,
    get_user,
    get_user_by_display_number,
    get_user_by_email,
    get_workflow,
    get_workflow_by_display_number,
    get_workflow_definition,
    health_check,
    list_comments,
    list_my_tasks,
    list_my_workflows,
    list_roles,
    list_users,
    list_workflow_definitions,
    post_comment,
    reject_step,
    reject_step_by_display_number,
    request_changes_step,
    request_changes_step_by_display_number,
    resubmit_workflow,
    resubmit_workflow_by_display_number,
    submit_workflow,
    submit_workflow_by_display_number,
    update_role,
    update_user,
    update_user_status,
};
use ringiflow_domain::clock::SystemClock;
use ringiflow_infra::{
    PgTransactionManager,
    db,
    repository::{
        DisplayIdCounterRepository,
        RoleRepository,
        TenantRepository,
        UserRepository,
        WorkflowCommentRepository,
        WorkflowDefinitionRepository,
        WorkflowInstanceRepository,
        WorkflowStepRepository,
        display_id_counter_repository::PostgresDisplayIdCounterRepository,
        role_repository::PostgresRoleRepository,
        tenant_repository::PostgresTenantRepository,
        user_repository::PostgresUserRepository,
        workflow_comment_repository::PostgresWorkflowCommentRepository,
        workflow_definition_repository::PostgresWorkflowDefinitionRepository,
        workflow_instance_repository::PostgresWorkflowInstanceRepository,
        workflow_step_repository::PostgresWorkflowStepRepository,
    },
};
use ringiflow_shared::observability::{TracingConfig, make_request_span};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use usecase::{
    DashboardUseCaseImpl,
    RoleUseCaseImpl,
    TaskUseCaseImpl,
    UserUseCaseImpl,
    WorkflowUseCaseImpl,
};

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

    // 共有リポジトリインスタンスを初期化
    let user_repo: Arc<dyn UserRepository> = Arc::new(PostgresUserRepository::new(pool.clone()));
    let tenant_repo: Arc<dyn TenantRepository> =
        Arc::new(PostgresTenantRepository::new(pool.clone()));
    let definition_repo: Arc<dyn WorkflowDefinitionRepository> =
        Arc::new(PostgresWorkflowDefinitionRepository::new(pool.clone()));
    let instance_repo: Arc<dyn WorkflowInstanceRepository> =
        Arc::new(PostgresWorkflowInstanceRepository::new(pool.clone()));
    let step_repo: Arc<dyn WorkflowStepRepository> =
        Arc::new(PostgresWorkflowStepRepository::new(pool.clone()));
    let comment_repo: Arc<dyn WorkflowCommentRepository> =
        Arc::new(PostgresWorkflowCommentRepository::new(pool.clone()));
    let counter_repo: Arc<dyn DisplayIdCounterRepository> =
        Arc::new(PostgresDisplayIdCounterRepository::new(pool.clone()));

    let role_repo: Arc<dyn RoleRepository> = Arc::new(PostgresRoleRepository::new(pool.clone()));

    // Clock（複数ユースケースで共有）
    let clock: Arc<dyn ringiflow_domain::clock::Clock> = Arc::new(SystemClock);

    // ユーザー UseCase + State
    let user_usecase = UserUseCaseImpl::new(user_repo.clone(), counter_repo.clone(), clock.clone());
    let user_state = Arc::new(UserState {
        user_repository:   user_repo.clone(),
        tenant_repository: tenant_repo,
        usecase:           user_usecase,
    });

    // ロール UseCase + State
    let role_usecase = RoleUseCaseImpl::new(role_repo.clone(), clock.clone());
    let role_state = Arc::new(RoleState {
        role_repository: role_repo,
        usecase:         role_usecase,
    });

    // ワークフロー UseCase
    let tx_manager = Arc::new(PgTransactionManager::new(pool.clone()));
    let workflow_usecase = WorkflowUseCaseImpl::new(
        definition_repo,
        instance_repo.clone(),
        step_repo.clone(),
        comment_repo,
        user_repo.clone(),
        counter_repo,
        clock,
        tx_manager,
    );
    let workflow_state = Arc::new(WorkflowState {
        usecase: workflow_usecase,
    });

    // タスク UseCase
    let task_usecase = TaskUseCaseImpl::new(instance_repo.clone(), step_repo.clone(), user_repo);
    let task_state = Arc::new(TaskState {
        usecase: task_usecase,
    });

    // ダッシュボード UseCase
    let dashboard_usecase = DashboardUseCaseImpl::new(instance_repo, step_repo);
    let dashboard_state = Arc::new(DashboardState {
        usecase: dashboard_usecase,
    });

    // ルーター構築
    let app = Router::new()
      .route("/health", get(health_check))
      .route("/internal/users", get(list_users).post(create_user))
      .route("/internal/users/by-email", get(get_user_by_email))
      .route(
         "/internal/users/{user_id}",
         get(get_user).patch(update_user),
      )
      .route(
         "/internal/users/{user_id}/status",
         patch(update_user_status),
      )
      .route(
         "/internal/users/by-display-number/{display_number}",
         get(get_user_by_display_number),
      )
      .with_state(user_state)
      // ロール管理 API
      .route(
         "/internal/roles",
         get(list_roles).post(create_role),
      )
      .route(
         "/internal/roles/{role_id}",
         get(get_role).patch(update_role).delete(delete_role),
      )
      .with_state(role_state)
      // ワークフロー定義 API
      .route("/internal/workflow-definitions", get(list_workflow_definitions))
      .route(
         "/internal/workflow-definitions/{id}",
         get(get_workflow_definition),
      )
      // ワークフローインスタンス API
      .route(
         "/internal/workflows",
         get(list_my_workflows).post(create_workflow),
      )
      .route("/internal/workflows/{id}", get(get_workflow))
      .route("/internal/workflows/{id}/submit", post(submit_workflow))
      .route(
         "/internal/workflows/{id}/steps/{step_id}/approve",
         post(approve_step),
      )
      .route(
         "/internal/workflows/{id}/steps/{step_id}/reject",
         post(reject_step),
      )
      .route(
         "/internal/workflows/{id}/steps/{step_id}/request-changes",
         post(request_changes_step),
      )
      .route(
         "/internal/workflows/{id}/resubmit",
         post(resubmit_workflow),
      )
      // display_number 対応 API
      .route(
         "/internal/workflows/by-display-number/{display_number}",
         get(get_workflow_by_display_number),
      )
      .route(
         "/internal/workflows/by-display-number/{display_number}/submit",
         post(submit_workflow_by_display_number),
      )
      .route(
         "/internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/approve",
         post(approve_step_by_display_number),
      )
      .route(
         "/internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/reject",
         post(reject_step_by_display_number),
      )
      .route(
         "/internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/request-changes",
         post(request_changes_step_by_display_number),
      )
      .route(
         "/internal/workflows/by-display-number/{display_number}/resubmit",
         post(resubmit_workflow_by_display_number),
      )
      .route(
         "/internal/workflows/by-display-number/{display_number}/comments",
         get(list_comments).post(post_comment),
      )
      .with_state(workflow_state)
      // タスク API
      .route("/internal/tasks/my", get(list_my_tasks))
      .route("/internal/tasks/{id}", get(get_task))
      .route(
         "/internal/workflows/by-display-number/{workflow_display_number}/tasks/{step_display_number}",
         get(get_task_by_display_numbers),
      )
      .with_state(task_state)
      // ダッシュボード API
      .route("/internal/dashboard/stats", get(get_dashboard_stats))
      .with_state(dashboard_state)
      .layer(TraceLayer::new_for_http().make_span_with(make_request_span));

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
