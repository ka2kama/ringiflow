//! # Core Service エラー定義
//!
//! Core Service 固有のエラーと、HTTP レスポンスへの変換を定義する。

use axum::{
   Json,
   http::StatusCode,
   response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// エラーレスポンス（RFC 7807 Problem Details）
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
   #[serde(rename = "type")]
   pub error_type: String,
   pub title:      String,
   pub status:     u16,
   pub detail:     String,
}

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
      let (status, error_type, title, detail) = match &self {
         CoreError::NotFound(msg) => (
            StatusCode::NOT_FOUND,
            "https://ringiflow.example.com/errors/not-found",
            "Not Found",
            msg.clone(),
         ),
         CoreError::BadRequest(msg) => (
            StatusCode::BAD_REQUEST,
            "https://ringiflow.example.com/errors/bad-request",
            "Bad Request",
            msg.clone(),
         ),
         CoreError::Forbidden(msg) => (
            StatusCode::FORBIDDEN,
            "https://ringiflow.example.com/errors/forbidden",
            "Forbidden",
            msg.clone(),
         ),
         CoreError::Conflict(msg) => (
            StatusCode::CONFLICT,
            "https://ringiflow.example.com/errors/conflict",
            "Conflict",
            msg.clone(),
         ),
         CoreError::Database(e) => {
            tracing::error!("データベースエラー: {}", e);
            (
               StatusCode::INTERNAL_SERVER_ERROR,
               "https://ringiflow.example.com/errors/internal-error",
               "Internal Server Error",
               "内部エラーが発生しました".to_string(),
            )
         }
         CoreError::Internal(msg) => {
            tracing::error!("内部エラー: {}", msg);
            (
               StatusCode::INTERNAL_SERVER_ERROR,
               "https://ringiflow.example.com/errors/internal-error",
               "Internal Server Error",
               "内部エラーが発生しました".to_string(),
            )
         }
      };

      (
         status,
         Json(ErrorResponse {
            error_type: error_type.to_string(),
            title: title.to_string(),
            status: status.as_u16(),
            detail,
         }),
      )
         .into_response()
   }
}
