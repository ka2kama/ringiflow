//! Core Service レスポンスの共通ハンドリング

use ringiflow_shared::ApiResponse;
use serde::de::DeserializeOwned;

use super::error::CoreServiceError;

/// Core Service レスポンスの共通ハンドリング
///
/// 成功時はレスポンスボディを `ApiResponse<T>` にデシリアライズし、
/// エラー時はステータスコードに応じた `CoreServiceError` を返す。
///
/// # 引数
///
/// - `response`: Core Service からの HTTP レスポンス
/// - `not_found_error`: 404 レスポンス時に返すエラー。`None` の場合は
///   `Unexpected` にフォールスルー
pub(super) async fn handle_response<T: DeserializeOwned>(
    response: reqwest::Response,
    not_found_error: Option<CoreServiceError>,
) -> Result<ApiResponse<T>, CoreServiceError> {
    let status = response.status();

    if status.is_success() {
        let body = response.json::<ApiResponse<T>>().await?;
        return Ok(body);
    }

    if status == reqwest::StatusCode::NOT_FOUND
        && let Some(err) = not_found_error
    {
        return Err(err);
    }

    let body = response.text().await.unwrap_or_default();

    let error = match status {
        reqwest::StatusCode::BAD_REQUEST => CoreServiceError::ValidationError(body),
        reqwest::StatusCode::FORBIDDEN => CoreServiceError::Forbidden(body),
        reqwest::StatusCode::CONFLICT => CoreServiceError::Conflict(body),
        _ => CoreServiceError::Unexpected(format!("予期しないステータス {}: {}", status, body)),
    };

    Err(error)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    /// テスト用のレスポンスデータ型
    #[derive(Debug, Deserialize, PartialEq)]
    struct TestData {
        value: String,
    }

    /// テスト用の HTTP レスポンスを構築する
    fn make_response(status: u16, body: &str) -> reqwest::Response {
        let http_resp = http::Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(body.to_string())
            .unwrap();
        reqwest::Response::from(http_resp)
    }

    #[tokio::test]
    async fn test_成功レスポンスをデシリアライズする() {
        let response = make_response(200, r#"{"data": {"value": "hello"}}"#);

        let result: Result<ApiResponse<TestData>, _> = handle_response(response, None).await;

        let api_response = result.unwrap();
        assert_eq!(
            api_response.data,
            TestData {
                value: "hello".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn test_404でnot_found_errorありのとき指定エラーを返す() {
        let response = make_response(404, "");

        let result: Result<ApiResponse<TestData>, _> =
            handle_response(response, Some(CoreServiceError::UserNotFound)).await;

        assert!(matches!(result, Err(CoreServiceError::UserNotFound)));
    }

    #[tokio::test]
    async fn test_404でnot_found_errorなしのときunexpectedを返す() {
        let response = make_response(404, "not found");

        let result: Result<ApiResponse<TestData>, _> = handle_response(response, None).await;

        match result {
            Err(CoreServiceError::Unexpected(msg)) => {
                assert!(
                    msg.contains("404"),
                    "メッセージにステータスコードが含まれること: {msg}"
                );
            }
            other => panic!("Unexpected を期待したが {other:?} を受け取った"),
        }
    }

    #[tokio::test]
    async fn test_400でvalidation_errorを返す() {
        let response = make_response(400, "invalid input");

        let result: Result<ApiResponse<TestData>, _> = handle_response(response, None).await;

        assert!(matches!(
            result,
            Err(CoreServiceError::ValidationError(body)) if body == "invalid input"
        ));
    }

    #[tokio::test]
    async fn test_403でforbiddenを返す() {
        let response = make_response(403, "access denied");

        let result: Result<ApiResponse<TestData>, _> = handle_response(response, None).await;

        assert!(matches!(
            result,
            Err(CoreServiceError::Forbidden(body)) if body == "access denied"
        ));
    }

    #[tokio::test]
    async fn test_409でconflictを返す() {
        let response = make_response(409, "conflict occurred");

        let result: Result<ApiResponse<TestData>, _> = handle_response(response, None).await;

        assert!(matches!(
            result,
            Err(CoreServiceError::Conflict(body)) if body == "conflict occurred"
        ));
    }

    #[tokio::test]
    async fn test_500でunexpectedを返す() {
        let response = make_response(500, "server error");

        let result: Result<ApiResponse<TestData>, _> = handle_response(response, None).await;

        match result {
            Err(CoreServiceError::Unexpected(msg)) => {
                assert!(
                    msg.contains("500"),
                    "メッセージにステータスコードが含まれること: {msg}"
                );
                assert!(
                    msg.contains("server error"),
                    "メッセージにボディが含まれること: {msg}"
                );
            }
            other => panic!("Unexpected を期待したが {other:?} を受け取った"),
        }
    }

    #[tokio::test]
    async fn test_成功だが不正なjsonでnetworkエラーを返す() {
        let response = make_response(200, "not json");

        let result: Result<ApiResponse<TestData>, _> = handle_response(response, None).await;

        assert!(matches!(result, Err(CoreServiceError::Network(_))));
    }
}
