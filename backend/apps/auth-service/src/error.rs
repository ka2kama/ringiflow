//! # Auth Service エラー定義
//!
//! Auth Service 固有のエラーと、HTTP レスポンスへの変換を定義する。

use axum::{
   Json,
   http::StatusCode,
   response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// エラーレスポンス（RFC 9457 Problem Details）
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
   #[serde(rename = "type")]
   pub error_type: String,
   pub title:      String,
   pub status:     u16,
   pub detail:     String,
}

/// Auth Service で発生するエラー
#[derive(Debug, Error)]
pub enum AuthError {
   /// 認証失敗
   #[error("認証に失敗しました")]
   AuthenticationFailed,

   /// 認証情報が見つからない
   // FIXME: `#[allow(dead_code)]` を解消する
   //        （認証情報取得 API を追加するか、バリアントごと削除する）
   #[error("認証情報が見つかりません")]
   #[allow(dead_code)]
   CredentialNotFound,

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
      let (status, error_type, title, detail) = match &self {
         AuthError::AuthenticationFailed => (
            StatusCode::UNAUTHORIZED,
            "https://ringiflow.example.com/errors/authentication-failed",
            "Authentication Failed",
            "認証に失敗しました".to_string(),
         ),
         AuthError::CredentialNotFound => (
            StatusCode::NOT_FOUND,
            "https://ringiflow.example.com/errors/credential-not-found",
            "Credential Not Found",
            "認証情報が見つかりません".to_string(),
         ),
         AuthError::CredentialInactive => (
            StatusCode::UNAUTHORIZED,
            "https://ringiflow.example.com/errors/credential-inactive",
            "Credential Inactive",
            "認証情報が無効です".to_string(),
         ),
         AuthError::Database(e) => {
            tracing::error!("データベースエラー: {}", e);
            (
               StatusCode::INTERNAL_SERVER_ERROR,
               "https://ringiflow.example.com/errors/internal-error",
               "Internal Server Error",
               "内部エラーが発生しました".to_string(),
            )
         }
         AuthError::Internal(msg) => {
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
