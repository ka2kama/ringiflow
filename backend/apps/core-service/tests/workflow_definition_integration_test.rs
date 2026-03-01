//! ワークフロー定義 CRUD API 統合テスト
//!
//! マルチステップのワークフローを通じて、Core Service API の
//! データ整合性を検証する。
//!
//! ## Phase 5 ハンドラテストとの違い
//!
//! - Phase 5: 個別エンドポイントのステータスコードを検証
//! - Phase 7（本テスト）: 複数操作を横断したレスポンスデータの整合性を検証
//!
//! ## テストケース
//!
//! - 定義の作成 → 取得で全フィールドが一致
//! - 定義の更新 → 取得で更新内容が反映
//! - 定義の作成 → 公開 → 一覧で Published が含まれる
//! - 公開 → アーカイブ → ステータスが Archived
//! - Draft 以外の更新・削除が拒否される
//! - バージョン競合で 409 が返る

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use ringiflow_core_service::{
    handler::{
        WorkflowDefinitionState,
        archive_definition,
        create_definition,
        delete_definition,
        get_definition,
        list_definitions,
        publish_definition,
        update_definition,
    },
    usecase::WorkflowDefinitionUseCaseImpl,
};
use ringiflow_domain::clock::FixedClock;
use ringiflow_infra::fake::FakeWorkflowDefinitionRepository;
use serde_json::{Value as JsonValue, json};
use tower::ServiceExt;
use uuid::Uuid;

// --- テストヘルパー ---

fn fixed_now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}

/// テスト用 Core Service アプリケーションを構築する
fn create_test_app() -> (Router, Uuid) {
    let tenant_id = Uuid::new_v4();
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
        .with_state(state);

    (app, tenant_id)
}

/// バリデーションが通る定義 JSON
fn valid_definition_json() -> JsonValue {
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

/// レスポンスボディを JSON として解析する
async fn parse_body(response: axum::http::Response<Body>) -> JsonValue {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// 定義を作成し、レスポンスの data を返すヘルパー
async fn create_definition_via_api(
    app: &Router,
    tenant_id: Uuid,
    name: &str,
    definition: JsonValue,
) -> JsonValue {
    let request = Request::builder()
        .method(Method::POST)
        .uri("/internal/workflow-definitions")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": name,
                "definition": definition,
                "tenant_id": tenant_id,
                "user_id": Uuid::new_v4()
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let json = parse_body(response).await;
    json["data"].clone()
}

/// 定義を公開するヘルパー
async fn publish_definition_via_api(
    app: &Router,
    def_id: &str,
    version: i32,
    tenant_id: Uuid,
) -> JsonValue {
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/internal/workflow-definitions/{}/publish", def_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"version": version, "tenant_id": tenant_id}).to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = parse_body(response).await;
    json["data"].clone()
}

// --- テストケース ---

#[tokio::test]
async fn test_作成した定義を取得すると全フィールドが一致する() {
    // Given
    let (app, tenant_id) = create_test_app();
    let definition_json = valid_definition_json();

    // When: 作成
    let create_request = Request::builder()
        .method(Method::POST)
        .uri("/internal/workflow-definitions")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "テスト定義",
                "description": "テスト用の説明",
                "definition": definition_json,
                "tenant_id": tenant_id,
                "user_id": Uuid::new_v4()
            })
            .to_string(),
        ))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created = parse_body(create_response).await;
    let created_data = &created["data"];
    let def_id = created_data["id"].as_str().unwrap();

    // When: 取得
    let get_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/internal/workflow-definitions/{}?tenant_id={}",
            def_id, tenant_id
        ))
        .body(Body::empty())
        .unwrap();

    let get_response = app.oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);
    let got = parse_body(get_response).await;
    let got_data = &got["data"];

    // Then: 全フィールドが一致
    assert_eq!(got_data["id"], created_data["id"]);
    assert_eq!(got_data["name"], "テスト定義");
    assert_eq!(got_data["description"], "テスト用の説明");
    assert_eq!(got_data["status"], "Draft");
    assert_eq!(got_data["version"], 1);
    assert_eq!(got_data["definition"], definition_json);
    assert_eq!(got_data["created_by"], created_data["created_by"]);
    assert_eq!(got_data["created_at"], created_data["created_at"]);
    assert_eq!(got_data["updated_at"], created_data["updated_at"]);
}

#[tokio::test]
async fn test_更新した定義を取得すると更新内容が反映されている() {
    // Given
    let (app, tenant_id) = create_test_app();
    let created =
        create_definition_via_api(&app, tenant_id, "元の名前", json!({"steps": []})).await;
    let def_id = created["id"].as_str().unwrap();

    // When: 更新
    let updated_definition = json!({"steps": [{"id": "new_step"}]});
    let update_request = Request::builder()
        .method(Method::PUT)
        .uri(format!("/internal/workflow-definitions/{}", def_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "更新後の名前",
                "description": "更新された説明",
                "definition": updated_definition,
                "version": 1,
                "tenant_id": tenant_id
            })
            .to_string(),
        ))
        .unwrap();

    let update_response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(update_response.status(), StatusCode::OK);

    // When: 取得
    let get_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/internal/workflow-definitions/{}?tenant_id={}",
            def_id, tenant_id
        ))
        .body(Body::empty())
        .unwrap();

    let get_response = app.oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);
    let got = parse_body(get_response).await;
    let got_data = &got["data"];

    // Then: 更新内容が反映されている
    assert_eq!(got_data["name"], "更新後の名前");
    assert_eq!(got_data["description"], "更新された説明");
    assert_eq!(got_data["definition"], updated_definition);
    assert_eq!(got_data["version"], 2);
    assert_eq!(got_data["status"], "Draft");
}

#[tokio::test]
async fn test_作成して公開すると一覧でpublished状態で取得できる() {
    // Given
    let (app, tenant_id) = create_test_app();
    let created =
        create_definition_via_api(&app, tenant_id, "公開予定", valid_definition_json()).await;
    let def_id = created["id"].as_str().unwrap();

    // When: 公開
    publish_definition_via_api(&app, def_id, 1, tenant_id).await;

    // When: 一覧取得
    let list_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/internal/workflow-definitions?tenant_id={}",
            tenant_id
        ))
        .body(Body::empty())
        .unwrap();

    let list_response = app.oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list = parse_body(list_response).await;
    let definitions = list["data"].as_array().unwrap();

    // Then: Published 状態の定義が1件
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0]["id"].as_str().unwrap(), def_id);
    assert_eq!(definitions[0]["status"], "Published");
    assert_eq!(definitions[0]["version"], 2);
}

#[tokio::test]
async fn test_公開後にアーカイブするとarchived状態になる() {
    // Given
    let (app, tenant_id) = create_test_app();
    let created =
        create_definition_via_api(&app, tenant_id, "アーカイブ予定", valid_definition_json()).await;
    let def_id = created["id"].as_str().unwrap();

    // 公開（version 1 → 2）
    publish_definition_via_api(&app, def_id, 1, tenant_id).await;

    // When: アーカイブ（version 2 → 3）
    let archive_request = Request::builder()
        .method(Method::POST)
        .uri(format!("/internal/workflow-definitions/{}/archive", def_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"version": 2, "tenant_id": tenant_id}).to_string(),
        ))
        .unwrap();

    let archive_response = app.clone().oneshot(archive_request).await.unwrap();
    assert_eq!(archive_response.status(), StatusCode::OK);

    // When: 取得して確認
    let get_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/internal/workflow-definitions/{}?tenant_id={}",
            def_id, tenant_id
        ))
        .body(Body::empty())
        .unwrap();

    let get_response = app.oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);
    let got = parse_body(get_response).await;

    // Then: Archived 状態、バージョンは 3
    assert_eq!(got["data"]["status"], "Archived");
    assert_eq!(got["data"]["version"], 3);
}

#[tokio::test]
async fn test_published定義の更新と削除が拒否される() {
    // Given
    let (app, tenant_id) = create_test_app();
    let created =
        create_definition_via_api(&app, tenant_id, "公開済み", valid_definition_json()).await;
    let def_id = created["id"].as_str().unwrap();

    // 公開
    publish_definition_via_api(&app, def_id, 1, tenant_id).await;

    // When: Published 定義を更新
    let update_request = Request::builder()
        .method(Method::PUT)
        .uri(format!("/internal/workflow-definitions/{}", def_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "更新",
                "definition": {},
                "version": 2,
                "tenant_id": tenant_id
            })
            .to_string(),
        ))
        .unwrap();

    let update_response = app.clone().oneshot(update_request).await.unwrap();

    // Then: 400 Bad Request
    assert_eq!(update_response.status(), StatusCode::BAD_REQUEST);

    // When: Published 定義を削除
    let delete_request = Request::builder()
        .method(Method::DELETE)
        .uri(format!(
            "/internal/workflow-definitions/{}?tenant_id={}",
            def_id, tenant_id
        ))
        .body(Body::empty())
        .unwrap();

    let delete_response = app.oneshot(delete_request).await.unwrap();

    // Then: 400 Bad Request
    assert_eq!(delete_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_バージョン競合で409が返る() {
    // Given
    let (app, tenant_id) = create_test_app();
    let created = create_definition_via_api(&app, tenant_id, "テスト", json!({"steps": []})).await;
    let def_id = created["id"].as_str().unwrap();

    // When: 不正なバージョン（999）で更新
    let update_request = Request::builder()
        .method(Method::PUT)
        .uri(format!("/internal/workflow-definitions/{}", def_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "更新",
                "definition": {},
                "version": 999,
                "tenant_id": tenant_id
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(update_request).await.unwrap();

    // Then: 409 Conflict
    assert_eq!(response.status(), StatusCode::CONFLICT);
}
