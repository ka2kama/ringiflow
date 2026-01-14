//! # API エラーハンドリング
//!
//! HTTP API のエラー定義と、axum レスポンスへの変換を行う。
//!
//! ## 設計方針
//!
//! - **RFC 7807 準拠**: Problem Details for HTTP APIs 仕様に従う
//! - **thiserror + anyhow**: 型安全なエラー定義とエラーチェインの両立
//! - **IntoResponse 実装**: axum との統合による自動レスポンス変換
//!
//! ## RFC 7807 (Problem Details) とは
//!
//! HTTP API のエラーレスポンス形式を標準化した仕様。
//! 以下のフィールドを持つ JSON オブジェクトを返す:
//!
//! ```json
//! {
//!   "type": "https://example.com/problems/not-found",
//!   "title": "リソースが見つかりません",
//!   "status": 404,
//!   "detail": "ID 'abc123' のワークフローは存在しません"
//! }
//! ```
//!
//! ## エラーの階層
//!
//! ```text
//! ドメイン層エラー (DomainError)
//!        ↓ 変換
//! API エラー (ApiError)
//!        ↓ IntoResponse
//! HTTP レスポンス (StatusCode + JSON)
//! ```
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_api::error::ApiError;
//!
//! async fn get_workflow(id: String) -> Result<Json<Workflow>, ApiError> {
//!     let workflow = repository.find(&id)
//!         .await
//!         .map_err(|e| ApiError::Internal(e.into()))?
//!         .ok_or(ApiError::NotFound)?;
//!
//!     Ok(Json(workflow))
//! }
//! ```

use axum::{
   Json,
   http::StatusCode,
   response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// API 層で発生するエラー
///
/// ハンドラから返されるエラー型。`IntoResponse` を実装しているため、
/// axum が自動的に HTTP レスポンスに変換する。
///
/// # 設計判断
///
/// ## thiserror vs 手動実装
///
/// `thiserror` を使用することで、`std::error::Error` トレイトと
/// `Display` トレイトの実装を自動生成。ボイラープレートを削減。
///
/// ## anyhow との連携
///
/// `Internal` バリアントは `#[from] anyhow::Error` を持つため、
/// `?` 演算子で任意のエラーを `ApiError::Internal` に変換可能。
///
/// # 使用例
///
/// ```rust,ignore
/// async fn handler() -> Result<(), ApiError> {
///     // anyhow::Error から自動変換
///     some_fallible_operation().await?;
///
///     // 明示的なエラー生成
///     if !authorized {
///         return Err(ApiError::Forbidden);
///     }
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Error)]
pub enum ApiError {
   /// リソースが見つからない（404 Not Found）
   ///
   /// 指定された ID のリソースがデータベースに存在しない場合に使用。
   #[error("リソースが見つかりません")]
   NotFound,

   /// バリデーションエラー（400 Bad Request）
   ///
   /// リクエストボディやパラメータが不正な場合に使用。
   /// メッセージには具体的な検証エラーの内容を含める。
   #[error("バリデーションエラー: {0}")]
   Validation(String),

   /// 認証エラー（401 Unauthorized）
   ///
   /// ユーザーが認証されていない（ログインしていない）場合に使用。
   /// 認可エラー（`Forbidden`）とは異なる点に注意。
   #[error("認証エラー")]
   Unauthorized,

   /// 権限エラー（403 Forbidden）
   ///
   /// ユーザーは認証されているが、リソースへのアクセス権限がない場合に使用。
   #[error("権限エラー")]
   Forbidden,

   /// 内部サーバーエラー（500 Internal Server Error）
   ///
   /// 予期しないエラーが発生した場合に使用。
   /// `#[from]` により、`anyhow::Error` から自動変換される。
   ///
   /// # セキュリティ注意
   ///
   /// 内部エラーの詳細はクライアントに返さない。
   /// エラー内容はサーバーサイドのログにのみ出力する。
   #[error("内部サーバーエラー")]
   Internal(#[from] anyhow::Error),
}

/// RFC 7807 準拠のエラーレスポンス
///
/// HTTP API のエラーを標準化された形式で返すための構造体。
/// [RFC 7807](https://datatracker.ietf.org/doc/html/rfc7807) に準拠。
///
/// # フィールド
///
/// - `type`: エラーの種類を識別する URI（現在は `about:blank` を使用）
/// - `title`: エラーの概要（人間可読）
/// - `status`: HTTP ステータスコード
/// - `detail`: エラーの詳細情報（オプション）
///
/// # 将来の拡張
///
/// - `instance`: 問題が発生した具体的なリソースの URI
/// - `errors`: バリデーションエラーの詳細配列
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
   /// エラーの種類を識別する URI
   #[serde(rename = "type")]
   pub error_type: String,
   /// エラーの概要
   pub title:      String,
   /// HTTP ステータスコード
   pub status:     u16,
   /// エラーの詳細情報（オプション）
   #[serde(skip_serializing_if = "Option::is_none")]
   pub detail:     Option<String>,
}

impl IntoResponse for ApiError {
   /// `ApiError` を axum の HTTP レスポンスに変換する
   ///
   /// 各エラーバリアントを適切な HTTP ステータスコードと
   /// RFC 7807 形式の JSON レスポンスにマッピングする。
   ///
   /// # マッピング
   ///
   /// | ApiError | HTTP Status |
   /// |----------|-------------|
   /// | NotFound | 404 |
   /// | Validation | 400 |
   /// | Unauthorized | 401 |
   /// | Forbidden | 403 |
   /// | Internal | 500 |
   fn into_response(self) -> Response {
      let (status, error_response) = match self {
         ApiError::NotFound => (
            StatusCode::NOT_FOUND,
            ErrorResponse {
               error_type: "about:blank".to_string(),
               title:      "リソースが見つかりません".to_string(),
               status:     404,
               detail:     None,
            },
         ),
         ApiError::Validation(msg) => (
            StatusCode::BAD_REQUEST,
            ErrorResponse {
               error_type: "about:blank".to_string(),
               title:      "バリデーションエラー".to_string(),
               status:     400,
               detail:     Some(msg),
            },
         ),
         ApiError::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            ErrorResponse {
               error_type: "about:blank".to_string(),
               title:      "認証が必要です".to_string(),
               status:     401,
               detail:     None,
            },
         ),
         ApiError::Forbidden => (
            StatusCode::FORBIDDEN,
            ErrorResponse {
               error_type: "about:blank".to_string(),
               title:      "アクセスが拒否されました".to_string(),
               status:     403,
               detail:     None,
            },
         ),
         ApiError::Internal(err) => {
            // 内部エラーの詳細はログにのみ出力（セキュリティ考慮）
            tracing::error!("内部エラー: {:?}", err);
            (
               StatusCode::INTERNAL_SERVER_ERROR,
               ErrorResponse {
                  error_type: "about:blank".to_string(),
                  title:      "内部サーバーエラー".to_string(),
                  status:     500,
                  detail:     None,
               },
            )
         }
      };

      (status, Json(error_response)).into_response()
   }
}
