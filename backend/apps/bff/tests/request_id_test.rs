//! # Request ID レイヤーのテスト
//!
//! BFF の Request ID レイヤー（SetRequestIdLayer + PropagateRequestIdLayer +
//! カスタム make_span_with）が正しく動作することを検証する。
//!
//! - レスポンスに `X-Request-Id` ヘッダーが含まれる
//! - クライアント提供の `X-Request-Id` がそのまま返される
//! - 自動生成の `X-Request-Id` が UUID v7 形式である

use axum::{Json, Router, routing::get};
use http::{Request, StatusCode};
use ringiflow_shared::observability::{MakeRequestUuidV7, make_request_span};
use tower::ServiceExt;
use tower_http::{
    request_id::{PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

/// テスト用の最小限ルーターを構築する
///
/// BFF の main.rs と同じレイヤー構成（Request ID 関連のみ）を再現する。
fn test_app() -> Router {
    Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok"})) }),
        )
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http().make_span_with(make_request_span))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuidV7))
}

#[tokio::test]
async fn test_レスポンスにx_request_idヘッダーが含まれる() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers().contains_key("x-request-id"),
        "レスポンスに x-request-id ヘッダーが含まれること"
    );
}

#[tokio::test]
async fn test_クライアント提供のx_request_idがそのまま返される() {
    let app = test_app();
    let custom_id = "client-provided-request-id-123";

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("x-request-id", custom_id)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap(),
        custom_id,
        "クライアント提供の Request ID がそのまま返されること"
    );
}

#[tokio::test]
async fn test_自動生成のx_request_idがuuid_v7形式である() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let request_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();

    let uuid = uuid::Uuid::parse_str(request_id)
        .unwrap_or_else(|_| panic!("有効な UUID であること: {request_id}"));
    assert_eq!(
        uuid.get_version(),
        Some(uuid::Version::SortRand),
        "UUID v7（SortRand）であること"
    );
}
