//! # BFF アプリケーション構築
//!
//! DI（クライアント・State）の初期化とルーター構築を担当する。
//! `main.rs` はインフラ初期化とサーバー起動に集中する。

use std::sync::Arc;

use axum::{
    Router,
    middleware::{from_fn, from_fn_with_state},
    routing::{delete, get, patch, post, put},
};
use ringiflow_bff::{
    client::{AuthServiceClient, AuthServiceClientImpl, CoreServiceClientImpl},
    handler::{
        AuditLogState,
        AuthState,
        DocumentState,
        FolderState,
        ReadinessState,
        RoleState,
        UserState,
        WorkflowDefinitionState,
        WorkflowState,
        approve_step,
        archive_definition,
        confirm_upload,
        create_definition,
        create_folder,
        create_role,
        create_user,
        create_workflow,
        csrf,
        delete_definition,
        delete_document,
        delete_folder,
        delete_role,
        generate_download_url,
        get_dashboard_stats,
        get_role,
        get_task_by_display_numbers,
        get_user_detail,
        get_workflow,
        get_workflow_definition,
        health_check,
        list_audit_logs,
        list_comments,
        list_documents,
        list_folders,
        list_my_tasks,
        list_my_workflows,
        list_roles,
        list_users,
        list_workflow_attachments,
        list_workflow_definitions,
        login,
        logout,
        me,
        post_comment,
        publish_definition,
        readiness_check,
        reject_step,
        request_changes_step,
        request_upload_url,
        resubmit_workflow,
        submit_workflow,
        update_definition,
        update_folder,
        update_role,
        update_user,
        update_user_status,
        validate_definition,
    },
    middleware::{
        AuthzState,
        CsrfState,
        csrf_middleware,
        no_cache,
        request_id::store_request_id,
        require_permission,
    },
};
use ringiflow_infra::{SessionManager, repository::AuditLogRepository};
use ringiflow_shared::{
    canonical_log::CanonicalLogLineLayer,
    observability::{MakeRequestUuidV7, make_request_span},
};
use tower_http::{
    request_id::{PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::config::BffConfig;

/// DI コンテナの構築とルーター定義を行う
///
/// インフラ初期化済みの依存を受け取り、クライアント → State → Router の
/// 順に組み立てる。
pub(crate) fn build_app(
    config: &BffConfig,
    session_manager: Arc<dyn SessionManager>,
    readiness_state: Arc<ReadinessState>,
    audit_log_repository: Arc<dyn AuditLogRepository>,
) -> Router {
    // クライアントの初期化
    // 具象型で保持し、各 State 注入時に必要なトレイトオブジェクトへ coerce する
    let core_service_client = Arc::new(CoreServiceClientImpl::new(&config.core_url));
    let auth_service_client: Arc<dyn AuthServiceClient> =
        Arc::new(AuthServiceClientImpl::new(&config.auth_url));

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
        core_service_client:  core_service_client.clone(),
        session_manager:      session_manager.clone(),
        audit_log_repository: audit_log_repository.clone(),
    });

    // FolderState はフォルダ管理の CRUD に必要
    let folder_state = Arc::new(FolderState {
        core_service_client: core_service_client.clone(),
        session_manager:     session_manager.clone(),
    });

    // DocumentState はドキュメント管理（Upload URL 発行・確認）に必要
    let document_state = Arc::new(DocumentState {
        core_service_client,
        session_manager: session_manager.clone(),
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
    Router::new()
        .route("/health", get(health_check))
        .merge(
            Router::new()
                .route("/health/ready", get(readiness_check))
                .with_state(readiness_state),
        )
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
        // フォルダ管理 API
        .route(
            "/api/v1/folders",
            get(list_folders).post(create_folder),
        )
        .route(
            "/api/v1/folders/{folder_id}",
            put(update_folder).delete(delete_folder),
        )
        .with_state(folder_state)
        // ドキュメント管理 API
        .route(
            "/api/v1/documents",
            get(list_documents),
        )
        .route(
            "/api/v1/documents/upload-url",
            post(request_upload_url),
        )
        .route(
            "/api/v1/documents/{document_id}",
            delete(delete_document),
        )
        .route(
            "/api/v1/documents/{document_id}/confirm",
            post(confirm_upload),
        )
        .route(
            "/api/v1/documents/{document_id}/download-url",
            post(generate_download_url),
        )
        .route(
            "/api/v1/workflows/{workflow_instance_id}/attachments",
            get(list_workflow_attachments),
        )
        .with_state(document_state)
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
        // 3. CanonicalLogLineLayer: リクエスト完了時に1行サマリログを出力（スパン内）
        // 4. PropagateRequestIdLayer: レスポンスヘッダーに X-Request-Id をコピー
        // 5. store_request_id: task-local に保存し、BFF → 内部サービスのヘッダー伝播に使用
        .layer(from_fn(store_request_id))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CanonicalLogLineLayer)
        .layer(TraceLayer::new_for_http().make_span_with(make_request_span))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuidV7))
}
