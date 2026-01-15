//! # Core API エラーハンドリング
//!
//! HTTP API のエラー定義と、axum レスポンスへの変換。
//!
//! 詳細: [Rust エラーハンドリング](../../../docs/05_技術ノート/Rustエラーハンドリング.md)

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// Core API 層で発生するエラー
///
/// `IntoResponse` を実装しているため、axum が自動的に HTTP レスポンスに変換する。
#[derive(Debug, Error)]
pub enum CoreApiError {
    /// リソースが見つからない（404 Not Found）
    #[error("リソースが見つかりません")]
    NotFound,

    /// バリデーションエラー（400 Bad Request）
    #[error("バリデーションエラー: {0}")]
    Validation(String),

    /// 認証エラー（401 Unauthorized）
    #[error("認証エラー")]
    Unauthorized,

    /// 権限エラー（403 Forbidden）
    #[error("権限エラー")]
    Forbidden,

    /// 競合エラー（409 Conflict）
    #[error("リソースが競合しています")]
    Conflict,

    /// 内部サーバーエラー（500 Internal Server Error）
    #[error("内部サーバーエラー")]
    Internal(#[from] anyhow::Error),
}

/// RFC 7807 準拠のエラーレスポンス
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// エラーの種類を識別する URI
    #[serde(rename = "type")]
    pub error_type: String,
    /// エラーの概要
    pub title: String,
    /// HTTP ステータスコード
    pub status: u16,
    /// エラーの詳細情報（オプション）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl IntoResponse for CoreApiError {
    fn into_response(self) -> Response {
        let (status, error_response) = match self {
            CoreApiError::NotFound => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error_type: "about:blank".to_string(),
                    title: "リソースが見つかりません".to_string(),
                    status: 404,
                    detail: None,
                },
            ),
            CoreApiError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error_type: "about:blank".to_string(),
                    title: "バリデーションエラー".to_string(),
                    status: 400,
                    detail: Some(msg),
                },
            ),
            CoreApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse {
                    error_type: "about:blank".to_string(),
                    title: "認証が必要です".to_string(),
                    status: 401,
                    detail: None,
                },
            ),
            CoreApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                ErrorResponse {
                    error_type: "about:blank".to_string(),
                    title: "アクセスが拒否されました".to_string(),
                    status: 403,
                    detail: None,
                },
            ),
            CoreApiError::Conflict => (
                StatusCode::CONFLICT,
                ErrorResponse {
                    error_type: "about:blank".to_string(),
                    title: "リソースの競合".to_string(),
                    status: 409,
                    detail: None,
                },
            ),
            CoreApiError::Internal(err) => {
                // セキュリティ: 内部エラー詳細はログのみ
                tracing::error!("内部エラー: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse {
                        error_type: "about:blank".to_string(),
                        title: "内部サーバーエラー".to_string(),
                        status: 500,
                        detail: None,
                    },
                )
            }
        };

        (status, Json(error_response)).into_response()
    }
}
