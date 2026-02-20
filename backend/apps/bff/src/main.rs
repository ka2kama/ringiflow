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

mod config;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    middleware::{from_fn, from_fn_with_state},
    routing::{get, patch, post, put},
};
use client::{AuthServiceClient, AuthServiceClientImpl, CoreServiceClientImpl};
use config::BffConfig;
use handler::{
    AuditLogState,
    AuthState,
    RoleState,
    UserState,
    WorkflowDefinitionState,
    WorkflowState,
    approve_step,
    archive_definition,
    create_definition,
    create_role,
    create_user,
    create_workflow,
    csrf,
    delete_definition,
    delete_role,
    get_dashboard_stats,
    get_role,
    get_task_by_display_numbers,
    get_user_detail,
    get_workflow,
    get_workflow_definition,
    health_check,
    list_audit_logs,
    list_comments,
    list_my_tasks,
    list_my_workflows,
    list_roles,
    list_users,
    list_workflow_definitions,
    login,
    logout,
    me,
    post_comment,
    publish_definition,
    reject_step,
    request_changes_step,
    resubmit_workflow,
    submit_workflow,
    update_definition,
    update_role,
    update_user,
    update_user_status,
    validate_definition,
};
use middleware::{
    AuthzState,
    CsrfState,
    csrf_middleware,
    no_cache,
    request_id::store_request_id,
    require_permission,
};
#[cfg(feature = "dev-auth")]
use ringiflow_bff::dev_auth;
use ringiflow_bff::{client, handler, middleware};
use ringiflow_infra::{
    RedisSessionManager,
    SessionManager,
    dynamodb,
    repository::DynamoDbAuditLogRepository,
};
use ringiflow_shared::observability::{MakeRequestUuidV7, TracingConfig, make_request_span};
use tokio::net::TcpListener;
use tower_http::{
    request_id::{PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

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
    }

    // 依存関係の初期化
    // 具象型で保持し、各 State 注入時に必要なトレイトオブジェクトへ coerce する
    let session_manager: Arc<dyn SessionManager> = Arc::new(redis_session_manager);
    let core_service_client = Arc::new(CoreServiceClientImpl::new(&config.core_url));
    let auth_service_client: Arc<dyn AuthServiceClient> =
        Arc::new(AuthServiceClientImpl::new(&config.auth_url));

    // DynamoDB クライアントの初期化
    let dynamodb_client = dynamodb::create_client(&config.dynamodb_endpoint).await;
    dynamodb::ensure_audit_log_table(&dynamodb_client, "audit_logs")
        .await
        .expect("DynamoDB 監査ログテーブルのセットアップに失敗しました");
    let audit_log_repository = Arc::new(DynamoDbAuditLogRepository::new(
        dynamodb_client,
        "audit_logs".to_string(),
    ));

    // CSRF ミドルウェア用の状態
    let csrf_state = CsrfState {
        session_manager: session_manager.clone(),
    };

    // AuthState は CoreServiceUserClient のみ必要（ISP:
    // 認証に不要なメソッドを公開しない）
    let auth_state = Arc::new(AuthState {
        core_service_client: core_service_client.clone(),
        auth_service_client: auth_service_client.clone(),
        session_manager:     session_manager.clone(),
    });

    // WorkflowState は全サブトレイト（CoreServiceClient）が必要
    let workflow_state = Arc::new(WorkflowState {
        core_service_client: core_service_client.clone(),
        session_manager:     session_manager.clone(),
    });

    // UserState はユーザー管理の CRUD に必要（Core Service + Auth Service）
    let user_state = Arc::new(UserState {
        core_service_client: core_service_client.clone(),
        auth_service_client,
        session_manager: session_manager.clone(),
        audit_log_repository: audit_log_repository.clone(),
    });

    // WorkflowDefinitionState はワークフロー定義管理の CRUD に必要
    let workflow_definition_state = Arc::new(WorkflowDefinitionState {
        core_service_client: core_service_client.clone(),
        session_manager:     session_manager.clone(),
    });

    // RoleState はロール管理の CRUD に必要
    let role_state = Arc::new(RoleState {
        core_service_client,
        session_manager: session_manager.clone(),
        audit_log_repository: audit_log_repository.clone(),
    });

    // 認可ミドルウェア用の状態（権限別ルートグループ）
    // ユーザー管理とロール管理は同じ user:* 権限を共有する
    let user_read_authz = AuthzState {
        session_manager:     session_manager.clone(),
        required_permission: "user:read".to_string(),
    };
    let user_create_authz = AuthzState {
        session_manager:     session_manager.clone(),
        required_permission: "user:create".to_string(),
    };
    let user_update_authz = AuthzState {
        session_manager:     session_manager.clone(),
        required_permission: "user:update".to_string(),
    };

    // ロール管理 API 用の認可状態（ユーザー管理と同じ user:* 権限を使用）
    let role_read_authz = AuthzState {
        session_manager:     session_manager.clone(),
        required_permission: "user:read".to_string(),
    };
    let role_create_authz = AuthzState {
        session_manager:     session_manager.clone(),
        required_permission: "user:create".to_string(),
    };
    let role_update_authz = AuthzState {
        session_manager:     session_manager.clone(),
        required_permission: "user:update".to_string(),
    };

    // ワークフロー定義管理 API 用の認可状態
    let definition_manage_authz = AuthzState {
        session_manager:     session_manager.clone(),
        required_permission: "workflow_definition:manage".to_string(),
    };

    // 監査ログ閲覧 API 用の状態と認可
    let audit_log_state = Arc::new(AuditLogState {
        audit_log_repository,
        session_manager: session_manager.clone(),
    });
    let audit_log_read_authz = AuthzState {
        session_manager,
        required_permission: "user:read".to_string(),
    };

    // ルーター構築
    // Request ID + TraceLayer により、すべての HTTP リクエストに request_id が付与されログに自動注入される
    // CSRF ミドルウェアは POST/PUT/PATCH/DELETE リクエストを検証する
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/auth/csrf", get(csrf))
        .with_state(auth_state)
        // ワークフロー定義 API
        .route(
            "/api/v1/workflow-definitions",
            get(list_workflow_definitions),
        )
        .route(
            "/api/v1/workflow-definitions/{id}",
            get(get_workflow_definition),
        )
        // ワークフローインスタンス API
        .route(
            "/api/v1/workflows",
            get(list_my_workflows).post(create_workflow),
        )
        .route("/api/v1/workflows/{display_number}", get(get_workflow))
        .route(
            "/api/v1/workflows/{display_number}/submit",
            post(submit_workflow),
        )
        .route(
            "/api/v1/workflows/{display_number}/steps/{step_display_number}/approve",
            post(approve_step),
        )
        .route(
            "/api/v1/workflows/{display_number}/steps/{step_display_number}/reject",
            post(reject_step),
        )
        .route(
            "/api/v1/workflows/{display_number}/steps/{step_display_number}/request-changes",
            post(request_changes_step),
        )
        .route(
            "/api/v1/workflows/{display_number}/resubmit",
            post(resubmit_workflow),
        )
        // コメント API
        .route(
            "/api/v1/workflows/{display_number}/comments",
            get(list_comments).post(post_comment),
        )
        // タスク API
        .route("/api/v1/tasks/my", get(list_my_tasks))
        .route(
            "/api/v1/workflows/{display_number}/tasks/{step_display_number}",
            get(get_task_by_display_numbers),
        )
        // ダッシュボード API
        .route("/api/v1/dashboard/stats", get(get_dashboard_stats))
        .with_state(workflow_state.clone())
        // 管理者 API（認可ミドルウェア適用、権限別ルートグループ）
        .merge(
            Router::new()
                .route("/api/v1/users", get(list_users))
                .route("/api/v1/users/{display_number}", get(get_user_detail))
                .layer(from_fn_with_state(user_read_authz, require_permission))
                .with_state(user_state.clone()),
        )
        .merge(
            Router::new()
                .route("/api/v1/users", post(create_user))
                .layer(from_fn_with_state(user_create_authz, require_permission))
                .with_state(user_state.clone()),
        )
        .merge(
            Router::new()
                .route("/api/v1/users/{display_number}", patch(update_user))
                .route(
                    "/api/v1/users/{display_number}/status",
                    patch(update_user_status),
                )
                .layer(from_fn_with_state(user_update_authz, require_permission))
                .with_state(user_state),
        )
        // ロール管理 API（認可ミドルウェア適用、user:* 権限）
        .merge(
            Router::new()
                .route("/api/v1/roles", get(list_roles))
                .route("/api/v1/roles/{role_id}", get(get_role))
                .layer(from_fn_with_state(role_read_authz, require_permission))
                .with_state(role_state.clone()),
        )
        .merge(
            Router::new()
                .route("/api/v1/roles", post(create_role))
                .layer(from_fn_with_state(role_create_authz, require_permission))
                .with_state(role_state.clone()),
        )
        .merge(
            Router::new()
                .route(
                    "/api/v1/roles/{role_id}",
                    patch(update_role).delete(delete_role),
                )
                .layer(from_fn_with_state(role_update_authz, require_permission))
                .with_state(role_state),
        )
        // ワークフロー定義管理 API（認可ミドルウェア適用、workflow_definition:manage 権限）
        .merge(
            Router::new()
                .route(
                    "/api/v1/workflow-definitions",
                    post(create_definition),
                )
                .route(
                    "/api/v1/workflow-definitions/{id}",
                    put(update_definition).delete(delete_definition),
                )
                .route(
                    "/api/v1/workflow-definitions/{id}/publish",
                    post(publish_definition),
                )
                .route(
                    "/api/v1/workflow-definitions/{id}/archive",
                    post(archive_definition),
                )
                .route(
                    "/api/v1/workflow-definitions/validate",
                    post(validate_definition),
                )
                .layer(from_fn_with_state(definition_manage_authz, require_permission))
                .with_state(workflow_definition_state),
        )
        // 監査ログ閲覧 API（認可ミドルウェア適用、user:read 権限）
        .merge(
            Router::new()
                .route("/api/v1/audit-logs", get(list_audit_logs))
                .layer(from_fn_with_state(audit_log_read_authz, require_permission))
                .with_state(audit_log_state),
        )
        .layer(from_fn_with_state(csrf_state, csrf_middleware))
        // キャッシュ制御: 動的 API レスポンスがブラウザにキャッシュされないようにする
        .layer(from_fn(no_cache))
        // Request ID レイヤー（レイヤー順序が重要: 下に書いたものが外側）
        // 1. SetRequestIdLayer（最外）: リクエスト受信時に UUID v7 を生成（またはクライアント提供値を使用）
        // 2. TraceLayer: カスタムスパンに request_id を含め、全ログに自動注入
        // 3. PropagateRequestIdLayer: レスポンスヘッダーに X-Request-Id をコピー
        // 4. store_request_id: task-local に保存し、BFF → 内部サービスのヘッダー伝播に使用
        .layer(from_fn(store_request_id))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http().make_span_with(make_request_span))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuidV7));

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
