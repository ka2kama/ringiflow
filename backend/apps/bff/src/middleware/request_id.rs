//! # Request ID 伝播ミドルウェア
//!
//! BFF → 内部サービス間の Request ID 伝播を実現する。
//!
//! ## 仕組み
//!
//! 1. [`store_request_id`] ミドルウェアが `SetRequestIdLayer` の設定した
//!    [`RequestId`](tower_http::request_id::RequestId) を task-local に保存する
//! 2. [`inject_request_id`] ヘルパーが task-local から Request ID を取得し、
//!    reqwest の `RequestBuilder` に `X-Request-Id` ヘッダーとして付与する
//!
//! ## なぜ task-local を使用するか
//!
//! Request ID は横断的関心事であり、全ハンドラー・全クライアントメソッドに影響する。
//! 引数として明示的に渡す方法（型安全）は 34 箇所のメソッドシグネチャ変更が必要で
//! 侵襲的であるため、task-local による暗黙的な伝播を選択した。
//!
//! → ナレッジベース: [Observability > Request ID 伝播](../../docs/80_ナレッジベース/backend/observability.md)

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use ringiflow_shared::observability::REQUEST_ID_HEADER;
use tower_http::request_id::RequestId;

tokio::task_local! {
    static REQUEST_ID: String;
}

/// 現在のリクエストの Request ID を取得する
///
/// task-local に保存された Request ID を返す。
/// task-local スコープ外（テスト等）では `None` を返す。
pub fn current_request_id() -> Option<String> {
    REQUEST_ID.try_with(|id| id.clone()).ok()
}

/// Request ID を task-local に保存するミドルウェア
///
/// `SetRequestIdLayer` が設定した `RequestId` をリクエスト extensions から取得し、
/// task-local に保存する。これによりハンドラーやクライアントコードから
/// `current_request_id()` で Request ID にアクセスできる。
pub async fn store_request_id(request: Request<Body>, next: Next) -> Response {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .unwrap_or("-")
        .to_string();

    REQUEST_ID.scope(request_id, next.run(request)).await
}

/// reqwest リクエストビルダーに `X-Request-Id` ヘッダーを付与する
///
/// task-local に保存された Request ID があれば `x-request-id` ヘッダーとして付与する。
/// task-local スコープ外の場合はビルダーをそのまま返す。
pub fn inject_request_id(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match current_request_id() {
        Some(id) => builder.header(REQUEST_ID_HEADER, id),
        None => builder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_request_id_task_localスコープ外でnoneを返す() {
        assert_eq!(current_request_id(), None);
    }

    #[tokio::test]
    async fn test_inject_request_id_task_local設定時にヘッダーを付与する() {
        let client = reqwest::Client::new();

        let result = REQUEST_ID
            .scope("test-request-id-456".to_string(), async {
                let builder = inject_request_id(client.get("http://example.com"));
                builder.build().unwrap()
            })
            .await;

        let header_value = result
            .headers()
            .get("x-request-id")
            .expect("x-request-id ヘッダーが存在すること");
        assert_eq!(header_value.to_str().unwrap(), "test-request-id-456");
    }

    #[tokio::test]
    async fn test_inject_request_id_task_local未設定時にビルダーを変更しない() {
        let client = reqwest::Client::new();
        let builder = inject_request_id(client.get("http://example.com"));
        let request = builder.build().unwrap();

        assert!(
            request.headers().get("x-request-id").is_none(),
            "task-local 未設定時は x-request-id ヘッダーが含まれないこと"
        );
    }
}
