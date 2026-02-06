//! # Core Service エラー定義
//!
//! Core Service 固有のエラーと、HTTP レスポンスへの変換を定義する。

use axum::{
   Json,
   http::StatusCode,
   response::{IntoResponse, Response},
};
use ringiflow_shared::ErrorResponse;
use thiserror::Error;

/// Core Service で発生するエラー
#[derive(Debug, Error)]
pub enum CoreError {
   /// リソースが見つからない
   #[error("リソースが見つかりません: {0}")]
   NotFound(String),

   /// 不正なリクエスト
   #[error("不正なリクエスト: {0}")]
   BadRequest(String),

   /// 権限不足
   #[error("権限がありません: {0}")]
   Forbidden(String),

   /// 競合（楽観的ロック失敗）
   #[error("競合が発生しました: {0}")]
   Conflict(String),

   /// データベースエラー
   #[error("データベースエラー: {0}")]
   Database(#[from] ringiflow_infra::InfraError),

   /// 内部エラー
   #[error("内部エラー: {0}")]
   Internal(String),
}

impl IntoResponse for CoreError {
   fn into_response(self) -> Response {
      let (status, error_response) = match &self {
         CoreError::NotFound(msg) => (StatusCode::NOT_FOUND, ErrorResponse::not_found(msg.clone())),
         CoreError::BadRequest(msg) => (
            StatusCode::BAD_REQUEST,
            ErrorResponse::bad_request(msg.clone()),
         ),
         CoreError::Forbidden(msg) => {
            (StatusCode::FORBIDDEN, ErrorResponse::forbidden(msg.clone()))
         }
         CoreError::Conflict(msg) => (StatusCode::CONFLICT, ErrorResponse::conflict(msg.clone())),
         CoreError::Database(e) => {
            tracing::error!("データベースエラー: {}", e);
            (
               StatusCode::INTERNAL_SERVER_ERROR,
               ErrorResponse::internal_error(),
            )
         }
         CoreError::Internal(msg) => {
            tracing::error!("内部エラー: {}", msg);
            (
               StatusCode::INTERNAL_SERVER_ERROR,
               ErrorResponse::internal_error(),
            )
         }
      };

      (status, Json(error_response)).into_response()
   }
}
