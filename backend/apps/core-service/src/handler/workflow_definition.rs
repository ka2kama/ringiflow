//! # ワークフロー定義管理ハンドラ
//!
//! ワークフロー定義の CRUD 操作を行う内部 API。
//!
//! ## エンドポイント
//!
//! - `GET /internal/workflow-definitions` - 定義一覧（全ステータス）
//! - `GET /internal/workflow-definitions/{id}` - 定義詳細
//! - `POST /internal/workflow-definitions` - 新規作成（Draft）
//! - `PUT /internal/workflow-definitions/{id}` - 更新（Draft のみ）
//! - `DELETE /internal/workflow-definitions/{id}` - 削除（Draft のみ）
//! - `POST /internal/workflow-definitions/{id}/publish` - 公開
//! - `POST /internal/workflow-definitions/{id}/archive` - アーカイブ
//! - `POST /internal/workflow-definitions/validate` - バリデーション

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::WorkflowName,
    workflow::WorkflowDefinitionId,
};
use ringiflow_shared::ApiResponse;
use serde::Deserialize;
use uuid::Uuid;

use super::workflow::{TenantQuery, WorkflowDefinitionDto, parse_version};
use crate::{error::CoreError, usecase::WorkflowDefinitionUseCaseImpl};

/// ワークフロー定義管理 API の共有状態
pub struct WorkflowDefinitionState {
    pub usecase: WorkflowDefinitionUseCaseImpl,
}

// --- リクエスト型 ---

/// 定義作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateDefinitionRequest {
    /// ワークフロー定義名
    pub name:        String,
    /// 説明（任意）
    pub description: Option<String>,
    /// 定義 JSON
    pub definition:  serde_json::Value,
    /// テナント ID
    pub tenant_id:   Uuid,
    /// 作成者のユーザー ID
    pub user_id:     Uuid,
}

/// 定義更新リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateDefinitionRequest {
    /// ワークフロー定義名
    pub name:        String,
    /// 説明（任意）
    pub description: Option<String>,
    /// 定義 JSON
    pub definition:  serde_json::Value,
    /// 楽観的ロック用バージョン
    pub version:     i32,
    /// テナント ID
    pub tenant_id:   Uuid,
}

/// 公開/アーカイブリクエスト
#[derive(Debug, Deserialize)]
pub struct PublishArchiveRequest {
    /// 楽観的ロック用バージョン
    pub version:   i32,
    /// テナント ID
    pub tenant_id: Uuid,
}

/// バリデーションリクエスト
#[derive(Debug, Deserialize)]
pub struct ValidateDefinitionRequest {
    /// 検証対象の定義 JSON
    pub definition: serde_json::Value,
}

// --- ハンドラ ---

/// GET /internal/workflow-definitions
///
/// テナント内の全定義を取得する（ステータス問わず）。
///
/// ## レスポンス
///
/// - `200 OK`: 定義一覧
#[tracing::instrument(skip_all)]
pub async fn list_definitions(
    State(state): State<Arc<WorkflowDefinitionState>>,
    Query(query): Query<TenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    let definitions = state.usecase.list_definitions(&tenant_id).await?;

    let response = ApiResponse::new(
        definitions
            .into_iter()
            .map(WorkflowDefinitionDto::from)
            .collect::<Vec<_>>(),
    );

    Ok((StatusCode::OK, Json(response)))
}

/// GET /internal/workflow-definitions/{id}
///
/// 定義の詳細を取得する。
///
/// ## レスポンス
///
/// - `200 OK`: 定義詳細
/// - `404 Not Found`: 定義が見つからない
#[tracing::instrument(skip_all, fields(%id))]
pub async fn get_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<TenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let definition_id = WorkflowDefinitionId::from_uuid(id);
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    let definition = state
        .usecase
        .get_definition(&definition_id, &tenant_id)
        .await?;

    let response = ApiResponse::new(WorkflowDefinitionDto::from(definition));

    Ok((StatusCode::OK, Json(response)))
}

/// POST /internal/workflow-definitions
///
/// 新規定義を作成する（Draft 状態）。
///
/// ## レスポンス
///
/// - `201 Created`: 作成された定義
/// - `400 Bad Request`: 名前バリデーションエラー
#[tracing::instrument(skip_all)]
pub async fn create_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    Json(req): Json<CreateDefinitionRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let name = WorkflowName::new(&req.name).map_err(|e| CoreError::BadRequest(e.to_string()))?;

    let definition = state
        .usecase
        .create_definition(
            name,
            req.description,
            req.definition,
            TenantId::from_uuid(req.tenant_id),
            UserId::from_uuid(req.user_id),
        )
        .await?;

    let response = ApiResponse::new(WorkflowDefinitionDto::from(definition));

    Ok((StatusCode::CREATED, Json(response)))
}

/// PUT /internal/workflow-definitions/{id}
///
/// 定義を更新する（Draft のみ）。
///
/// ## レスポンス
///
/// - `200 OK`: 更新後の定義
/// - `400 Bad Request`: Draft 以外の更新、名前バリデーションエラー
/// - `404 Not Found`: 定義が見つからない
/// - `409 Conflict`: バージョン不一致
#[tracing::instrument(skip_all, fields(%id))]
pub async fn update_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDefinitionRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let definition_id = WorkflowDefinitionId::from_uuid(id);
    let name = WorkflowName::new(&req.name).map_err(|e| CoreError::BadRequest(e.to_string()))?;
    let version = parse_version(req.version)?;
    let tenant_id = TenantId::from_uuid(req.tenant_id);

    let updated = state
        .usecase
        .update_definition(
            &definition_id,
            name,
            req.description,
            req.definition,
            version,
            &tenant_id,
        )
        .await?;

    let response = ApiResponse::new(WorkflowDefinitionDto::from(updated));

    Ok((StatusCode::OK, Json(response)))
}

/// DELETE /internal/workflow-definitions/{id}
///
/// 定義を削除する（Draft のみ）。
///
/// ## レスポンス
///
/// - `204 No Content`: 削除成功
/// - `400 Bad Request`: Draft 以外の削除
/// - `404 Not Found`: 定義が見つからない
#[tracing::instrument(skip_all, fields(%id))]
pub async fn delete_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<TenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let definition_id = WorkflowDefinitionId::from_uuid(id);
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    state
        .usecase
        .delete_definition(&definition_id, &tenant_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /internal/workflow-definitions/{id}/publish
///
/// 定義を公開する（Draft → Published）。
/// バリデーション成功時のみ遷移する。
///
/// ## レスポンス
///
/// - `200 OK`: 公開後の定義
/// - `400 Bad Request`: バリデーション失敗、Draft 以外
/// - `404 Not Found`: 定義が見つからない
/// - `409 Conflict`: バージョン不一致
#[tracing::instrument(skip_all, fields(%id))]
pub async fn publish_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<PublishArchiveRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let definition_id = WorkflowDefinitionId::from_uuid(id);
    let version = parse_version(req.version)?;
    let tenant_id = TenantId::from_uuid(req.tenant_id);

    let published = state
        .usecase
        .publish_definition(&definition_id, version, &tenant_id)
        .await?;

    let response = ApiResponse::new(WorkflowDefinitionDto::from(published));

    Ok((StatusCode::OK, Json(response)))
}

/// POST /internal/workflow-definitions/{id}/archive
///
/// 定義をアーカイブする（Published → Archived）。
///
/// ## レスポンス
///
/// - `200 OK`: アーカイブ後の定義
/// - `400 Bad Request`: Published 以外
/// - `404 Not Found`: 定義が見つからない
/// - `409 Conflict`: バージョン不一致
#[tracing::instrument(skip_all, fields(%id))]
pub async fn archive_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<PublishArchiveRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let definition_id = WorkflowDefinitionId::from_uuid(id);
    let version = parse_version(req.version)?;
    let tenant_id = TenantId::from_uuid(req.tenant_id);

    let archived = state
        .usecase
        .archive_definition(&definition_id, version, &tenant_id)
        .await?;

    let response = ApiResponse::new(WorkflowDefinitionDto::from(archived));

    Ok((StatusCode::OK, Json(response)))
}

/// POST /internal/workflow-definitions/validate
///
/// 定義 JSON のバリデーションのみ実行する。
/// 保存は行わない。
///
/// ## レスポンス
///
/// - `200 OK`: バリデーション結果
#[tracing::instrument(skip_all)]
pub async fn validate_definition(
    State(state): State<Arc<WorkflowDefinitionState>>,
    Json(req): Json<ValidateDefinitionRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let result = state.usecase.validate_definition_json(&req.definition);

    let response = ApiResponse::new(result);

    Ok((StatusCode::OK, Json(response)))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        Router,
        body::Body,
        http::Request,
        routing::{get, post},
    };
    use chrono::{DateTime, Utc};
    use ringiflow_domain::{
        clock::FixedClock,
        value_objects::WorkflowName,
        workflow::{NewWorkflowDefinition, WorkflowDefinition, WorkflowDefinitionId},
    };
    use ringiflow_infra::fake::FakeWorkflowDefinitionRepository;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;
    use crate::usecase::WorkflowDefinitionUseCaseImpl;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn tenant_id() -> TenantId {
        TenantId::new()
    }

    fn create_test_app() -> (Router, TenantId) {
        let tid = tenant_id();
        let repo = Arc::new(FakeWorkflowDefinitionRepository::new());
        let clock = Arc::new(FixedClock::new(fixed_now()));
        let usecase = WorkflowDefinitionUseCaseImpl::new(repo, clock);
        let state = Arc::new(WorkflowDefinitionState { usecase });

        let app = Router::new()
            .route(
                "/internal/workflow-definitions",
                get(list_definitions).post(create_definition),
            )
            .route(
                "/internal/workflow-definitions/{id}",
                get(get_definition)
                    .put(update_definition)
                    .delete(delete_definition),
            )
            .route(
                "/internal/workflow-definitions/{id}/publish",
                post(publish_definition),
            )
            .route(
                "/internal/workflow-definitions/{id}/archive",
                post(archive_definition),
            )
            .route(
                "/internal/workflow-definitions/validate",
                post(validate_definition),
            )
            .with_state(state);

        (app, tid)
    }

    fn create_test_app_with_published() -> (Router, TenantId, WorkflowDefinitionId) {
        let tid = tenant_id();
        let repo = Arc::new(FakeWorkflowDefinitionRepository::new());

        // Published 定義を直接セットアップ
        let def = WorkflowDefinition::new(NewWorkflowDefinition {
            id:          WorkflowDefinitionId::new(),
            tenant_id:   tid.clone(),
            name:        WorkflowName::new("公開済み").unwrap(),
            description: None,
            definition:  valid_definition_json(),
            created_by:  UserId::new(),
            now:         fixed_now(),
        });
        let published = def.published(fixed_now()).unwrap();
        let def_id = published.id().clone();
        repo.add_definition(published);

        let clock = Arc::new(FixedClock::new(fixed_now()));
        let usecase = WorkflowDefinitionUseCaseImpl::new(repo, clock);
        let state = Arc::new(WorkflowDefinitionState { usecase });

        let app = Router::new()
            .route(
                "/internal/workflow-definitions/{id}",
                get(get_definition)
                    .put(update_definition)
                    .delete(delete_definition),
            )
            .route(
                "/internal/workflow-definitions/{id}/publish",
                post(publish_definition),
            )
            .route(
                "/internal/workflow-definitions/{id}/archive",
                post(archive_definition),
            )
            .with_state(state);

        (app, tid, def_id)
    }

    fn valid_definition_json() -> serde_json::Value {
        json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        })
    }

    // --- テストケース ---

    #[tokio::test]
    async fn test_post_定義作成が201を返す() {
        // Given
        let (sut, tid) = create_test_app();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/workflow-definitions")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "テスト定義",
                    "description": "テスト用の定義",
                    "definition": {"steps": []},
                    "tenant_id": tid.as_uuid(),
                    "user_id": Uuid::new_v4()
                })
                .to_string(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_put_定義更新が200を返す() {
        // Given
        let (sut, tid) = create_test_app();
        let user_id = Uuid::new_v4();

        // まず作成
        let create_request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/workflow-definitions")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "元の名前",
                    "definition": {"steps": []},
                    "tenant_id": tid.as_uuid(),
                    "user_id": user_id
                })
                .to_string(),
            ))
            .unwrap();

        let create_response = sut.clone().oneshot(create_request).await.unwrap();
        assert_eq!(create_response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let def_id = created["data"]["id"].as_str().unwrap();

        // When: 更新
        let update_request = Request::builder()
            .method(axum::http::Method::PUT)
            .uri(format!("/internal/workflow-definitions/{}", def_id))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "更新後の名前",
                    "description": "説明追加",
                    "definition": {"steps": [{"id": "s1"}]},
                    "version": 1,
                    "tenant_id": tid.as_uuid()
                })
                .to_string(),
            ))
            .unwrap();

        let response = sut.oneshot(update_request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_delete_定義削除が204を返す() {
        // Given
        let (sut, tid) = create_test_app();
        let user_id = Uuid::new_v4();

        // まず作成
        let create_request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/workflow-definitions")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "削除対象",
                    "definition": {"steps": []},
                    "tenant_id": tid.as_uuid(),
                    "user_id": user_id
                })
                .to_string(),
            ))
            .unwrap();

        let create_response = sut.clone().oneshot(create_request).await.unwrap();
        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let def_id = created["data"]["id"].as_str().unwrap();

        // When
        let delete_request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/workflow-definitions/{}?tenant_id={}",
                def_id,
                tid.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        let response = sut.oneshot(delete_request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_publish_バリデーション成功で200を返す() {
        // Given
        let (sut, tid) = create_test_app();

        // バリデーションに通る定義を作成
        let create_request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/workflow-definitions")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "公開予定",
                    "definition": valid_definition_json(),
                    "tenant_id": tid.as_uuid(),
                    "user_id": Uuid::new_v4()
                })
                .to_string(),
            ))
            .unwrap();

        let create_response = sut.clone().oneshot(create_request).await.unwrap();
        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let def_id = created["data"]["id"].as_str().unwrap();

        // When
        let publish_request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!("/internal/workflow-definitions/{}/publish", def_id))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "version": 1,
                    "tenant_id": tid.as_uuid()
                })
                .to_string(),
            ))
            .unwrap();

        let response = sut.oneshot(publish_request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_publish_バリデーション失敗で400を返す() {
        // Given
        let (sut, tid) = create_test_app();

        // バリデーションに失敗する定義を作成（steps が空）
        let create_request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/workflow-definitions")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "不正な定義",
                    "definition": {"steps": [], "transitions": []},
                    "tenant_id": tid.as_uuid(),
                    "user_id": Uuid::new_v4()
                })
                .to_string(),
            ))
            .unwrap();

        let create_response = sut.clone().oneshot(create_request).await.unwrap();
        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let def_id = created["data"]["id"].as_str().unwrap();

        // When
        let publish_request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!("/internal/workflow-definitions/{}/publish", def_id))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "version": 1,
                    "tenant_id": tid.as_uuid()
                })
                .to_string(),
            ))
            .unwrap();

        let response = sut.oneshot(publish_request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_archive_published定義のアーカイブが200を返す() {
        // Given
        let (sut, tid, def_id) = create_test_app_with_published();

        // When
        let archive_request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!(
                "/internal/workflow-definitions/{}/archive",
                def_id.as_uuid()
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "version": 2,
                    "tenant_id": tid.as_uuid()
                })
                .to_string(),
            ))
            .unwrap();

        let response = sut.oneshot(archive_request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_put_published定義の更新が400を返す() {
        // Given
        let (sut, tid, def_id) = create_test_app_with_published();

        // When
        let update_request = Request::builder()
            .method(axum::http::Method::PUT)
            .uri(format!(
                "/internal/workflow-definitions/{}",
                def_id.as_uuid()
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "更新",
                    "definition": {},
                    "version": 2,
                    "tenant_id": tid.as_uuid()
                })
                .to_string(),
            ))
            .unwrap();

        let response = sut.oneshot(update_request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_published定義の削除が400を返す() {
        // Given
        let (sut, tid, def_id) = create_test_app_with_published();

        // When
        let delete_request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/workflow-definitions/{}?tenant_id={}",
                def_id.as_uuid(),
                tid.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        let response = sut.oneshot(delete_request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_put_バージョン不一致で409を返す() {
        // Given
        let (sut, tid) = create_test_app();

        // 作成
        let create_request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/workflow-definitions")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "テスト",
                    "definition": {"steps": []},
                    "tenant_id": tid.as_uuid(),
                    "user_id": Uuid::new_v4()
                })
                .to_string(),
            ))
            .unwrap();

        let create_response = sut.clone().oneshot(create_request).await.unwrap();
        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let def_id = created["data"]["id"].as_str().unwrap();

        // When: 不正なバージョンで更新
        let update_request = Request::builder()
            .method(axum::http::Method::PUT)
            .uri(format!("/internal/workflow-definitions/{}", def_id))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "name": "更新",
                    "definition": {},
                    "version": 999,
                    "tenant_id": tid.as_uuid()
                })
                .to_string(),
            ))
            .unwrap();

        let response = sut.oneshot(update_request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
