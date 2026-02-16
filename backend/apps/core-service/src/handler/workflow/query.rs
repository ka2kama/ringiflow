//! ワークフローハンドラの読み取り操作

use std::{collections::HashSet, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    workflow::{WorkflowDefinitionId, WorkflowInstanceId},
};
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
    TenantQuery,
    UserQuery,
    WorkflowCommentDto,
    WorkflowDefinitionDto,
    WorkflowInstanceDto,
    WorkflowState,
    parse_display_number,
};
use crate::error::CoreError;

/// ワークフロー定義一覧を取得する
///
/// ## エンドポイント
/// GET /internal/workflow-definitions?tenant_id={tenant_id}
///
/// ## 処理フロー
/// 1. クエリパラメータからテナント ID を取得
/// 2. ユースケースを呼び出し
/// 3. レスポンスを返す
pub async fn list_workflow_definitions(
    State(state): State<Arc<WorkflowState>>,
    Query(query): Query<TenantQuery>,
) -> Result<Response, CoreError> {
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    let definitions = state.usecase.list_workflow_definitions(tenant_id).await?;

    let response = ApiResponse::new(
        definitions
            .into_iter()
            .map(WorkflowDefinitionDto::from)
            .collect::<Vec<_>>(),
    );

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// ワークフロー定義の詳細を取得する
///
/// ## エンドポイント
/// GET /internal/workflow-definitions/{id}?tenant_id={tenant_id}
///
/// ## 処理フロー
/// 1. パスパラメータから ID を取得
/// 2. クエリパラメータからテナント ID を取得
/// 3. ユースケースを呼び出し
/// 4. レスポンスを返す
pub async fn get_workflow_definition(
    State(state): State<Arc<WorkflowState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<TenantQuery>,
) -> Result<Response, CoreError> {
    let definition_id = WorkflowDefinitionId::from_uuid(id);
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    let definition = state
        .usecase
        .get_workflow_definition(definition_id, tenant_id)
        .await?;

    let response = ApiResponse::new(WorkflowDefinitionDto::from(definition));

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// 自分のワークフロー一覧を取得する
///
/// ## エンドポイント
/// GET /internal/workflows?tenant_id={tenant_id}&user_id={user_id}
///
/// ## 処理フロー
/// 1. クエリパラメータからテナント ID とユーザー ID を取得
/// 2. ユースケースを呼び出し
/// 3. レスポンスを返す
pub async fn list_my_workflows(
    State(state): State<Arc<WorkflowState>>,
    Query(query): Query<UserQuery>,
) -> Result<Response, CoreError> {
    let tenant_id = TenantId::from_uuid(query.tenant_id);
    let user_id = UserId::from_uuid(query.user_id);

    let workflows = state.usecase.list_my_workflows(tenant_id, user_id).await?;

    // 全ワークフローの initiated_by を収集してユーザー名を一括解決
    let all_user_ids: Vec<UserId> = workflows
        .iter()
        .map(|w| w.initiated_by().clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let user_names = state.usecase.resolve_user_names(&all_user_ids).await?;

    let response = ApiResponse::new(
        workflows
            .iter()
            .map(|w| WorkflowInstanceDto::from_instance(w, &user_names))
            .collect::<Vec<_>>(),
    );

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// ワークフローの詳細を取得する
///
/// ## エンドポイント
/// GET /internal/workflows/{id}?tenant_id={tenant_id}
///
/// ## 処理フロー
/// 1. パスパラメータから ID を取得
/// 2. クエリパラメータからテナント ID を取得
/// 3. ユースケースを呼び出し
/// 4. レスポンスを返す
pub async fn get_workflow(
    State(state): State<Arc<WorkflowState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<TenantQuery>,
) -> Result<Response, CoreError> {
    let instance_id = WorkflowInstanceId::from_uuid(id);
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    let workflow_with_steps = state.usecase.get_workflow(instance_id, tenant_id).await?;

    // ユーザー名を解決
    let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(
        &workflow_with_steps.instance,
        &workflow_with_steps.steps,
    );
    let user_names = state.usecase.resolve_user_names(&user_ids).await?;

    let response = ApiResponse::new(WorkflowInstanceDto::from_workflow_with_steps(
        &workflow_with_steps,
        &user_names,
    ));

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// display_number でワークフローの詳細を取得する
///
/// ## エンドポイント
/// GET /internal/workflows/by-display-number/{display_number}?
/// tenant_id={tenant_id}
///
/// ## 処理フロー
/// 1. パスパラメータから display_number を取得
/// 2. クエリパラメータからテナント ID を取得
/// 3. ユースケースを呼び出し
/// 4. 200 OK + ワークフロー詳細を返す
pub async fn get_workflow_by_display_number(
    State(state): State<Arc<WorkflowState>>,
    Path(display_number): Path<i64>,
    Query(query): Query<TenantQuery>,
) -> Result<Response, CoreError> {
    let display_number = parse_display_number(display_number, "display_number")?;
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    let workflow_with_steps = state
        .usecase
        .get_workflow_by_display_number(display_number, tenant_id)
        .await?;

    // ユーザー名を解決
    let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(
        &workflow_with_steps.instance,
        &workflow_with_steps.steps,
    );
    let user_names = state.usecase.resolve_user_names(&user_ids).await?;

    let response = ApiResponse::new(WorkflowInstanceDto::from_workflow_with_steps(
        &workflow_with_steps,
        &user_names,
    ));

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// ワークフローのコメント一覧を取得する
///
/// ## エンドポイント
/// GET /internal/workflows/by-display-number/{display_number}/comments?
/// tenant_id={tenant_id}
///
/// ## 処理フロー
/// 1. パスパラメータから display_number を取得
/// 2. クエリパラメータからテナント ID を取得
/// 3. ユースケースを呼び出し
/// 4. 200 OK + コメント一覧を返す
pub async fn list_comments(
    State(state): State<Arc<WorkflowState>>,
    Path(display_number): Path<i64>,
    Query(query): Query<TenantQuery>,
) -> Result<Response, CoreError> {
    let display_number = parse_display_number(display_number, "display_number")?;
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    let comments = state
        .usecase
        .list_comments(display_number, tenant_id)
        .await?;

    // コメント投稿者のユーザー名を一括解決
    let all_user_ids: Vec<UserId> = comments
        .iter()
        .map(|c| c.posted_by().clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let user_names = state.usecase.resolve_user_names(&all_user_ids).await?;

    let response = ApiResponse::new(
        comments
            .iter()
            .map(|c| WorkflowCommentDto::from_comment(c, &user_names))
            .collect::<Vec<_>>(),
    );

    Ok((StatusCode::OK, Json(response)).into_response())
}
