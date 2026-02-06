//! # Auth Service エラー定義
//!
//! Auth Service 固有のエラーと、HTTP レスポンスへの変換を定義する。

use axum::{
   Json,
   http::StatusCode,
   response::{IntoResponse, Response},
};
use ringiflow_shared::ErrorResponse;
use thiserror::Error;

/// Auth Service で発生するエラー
#[derive(Debug, Error)]
pub enum AuthError {
   /// 認証失敗
   #[error("認証に失敗しました")]
   AuthenticationFailed,

   /// 認証情報が無効
   #[error("認証情報が無効です")]
   CredentialInactive,

   /// データベースエラー
   #[error("データベースエラー: {0}")]
   Database(#[from] ringiflow_infra::InfraError),

   /// 内部エラー
   #[error("内部エラー: {0}")]
   Internal(String),
}

impl IntoResponse for AuthError {
   fn into_response(self) -> Response {
      let (status, error_response) = match &self {
         AuthError::AuthenticationFailed => (
            StatusCode::UNAUTHORIZED,
            ErrorResponse::new(
               "authentication-failed",
               "Authentication Failed",
               401,
               "認証に失敗しました",
            ),
         ),
         AuthError::CredentialInactive => (
            StatusCode::UNAUTHORIZED,
            ErrorResponse::new(
               "credential-inactive",
               "Credential Inactive",
               401,
               "認証情報が無効です",
            ),
         ),
         AuthError::Database(e) => {
            tracing::error!("データベースエラー: {}", e);
            (
               StatusCode::INTERNAL_SERVER_ERROR,
               ErrorResponse::internal_error(),
            )
         }
         AuthError::Internal(msg) => {
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
