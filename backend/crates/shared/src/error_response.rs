//! # エラーレスポンス（RFC 9457 Problem Details）
//!
//! 全サービスで共通のエラーレスポンス構造体を提供する。
//!
//! ## 設計
//!
//! - `ErrorResponse` は純粋なデータ構造（`Serialize` / `Deserialize` のみ）
//! - axum の `IntoResponse` 変換は各サービスの責務（shared に axum 依存を入れない）
//! - よく使うエラー種別は便利コンストラクタで提供し、URI のハードコードを排除
//! - サービス固有のエラーは `new()` で自由に作成可能

use serde::{Deserialize, Serialize};

/// error_type URI のベースパス
const ERROR_TYPE_BASE: &str = "https://ringiflow.example.com/errors";

/// エラーレスポンス（RFC 9457 Problem Details）
///
/// すべてのサービスで統一されたエラーレスポンス形式。
/// `type` フィールドは URI で問題の種類を識別する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
   #[serde(rename = "type")]
   pub error_type: String,
   pub title:      String,
   pub status:     u16,
   pub detail:     String,
}

impl ErrorResponse {
   /// 汎用コンストラクタ
   ///
   /// サービス固有のエラー種別を作成する場合に使用する。
   /// `error_type_suffix` はベース URI に付加される（例: `"credential-not-found"`）。
   pub fn new(
      error_type_suffix: &str,
      title: impl Into<String>,
      status: u16,
      detail: impl Into<String>,
   ) -> Self {
      Self {
         error_type: format!("{ERROR_TYPE_BASE}/{error_type_suffix}"),
         title: title.into(),
         status,
         detail: detail.into(),
      }
   }

   /// 400 Bad Request
   pub fn bad_request(detail: impl Into<String>) -> Self {
      Self::new("bad-request", "Bad Request", 400, detail)
   }

   /// 401 Unauthorized
   pub fn unauthorized(detail: impl Into<String>) -> Self {
      Self::new("unauthorized", "Unauthorized", 401, detail)
   }

   /// 403 Forbidden
   pub fn forbidden(detail: impl Into<String>) -> Self {
      Self::new("forbidden", "Forbidden", 403, detail)
   }

   /// 404 Not Found
   pub fn not_found(detail: impl Into<String>) -> Self {
      Self::new("not-found", "Not Found", 404, detail)
   }

   /// 409 Conflict
   pub fn conflict(detail: impl Into<String>) -> Self {
      Self::new("conflict", "Conflict", 409, detail)
   }

   /// 400 Validation Error
   pub fn validation_error(detail: impl Into<String>) -> Self {
      Self::new("validation-error", "Validation Error", 400, detail)
   }

   /// 500 Internal Server Error
   ///
   /// detail は固定値（内部情報を漏らさないため）。
   pub fn internal_error() -> Self {
      Self::new(
         "internal-error",
         "Internal Server Error",
         500,
         "内部エラーが発生しました",
      )
   }

   /// 503 Service Unavailable
   pub fn service_unavailable(detail: impl Into<String>) -> Self {
      Self::new("service-unavailable", "Service Unavailable", 503, detail)
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_new_で全フィールドが正しく設定される() {
      let error = ErrorResponse::new("custom-error", "Custom Error", 418, "カスタムエラー");

      assert_eq!(
         error.error_type,
         "https://ringiflow.example.com/errors/custom-error"
      );
      assert_eq!(error.title, "Custom Error");
      assert_eq!(error.status, 418);
      assert_eq!(error.detail, "カスタムエラー");
   }

   #[test]
   fn test_not_found_が404と正しいerror_typeを返す() {
      let error = ErrorResponse::not_found("リソースが見つかりません");

      assert_eq!(
         error.error_type,
         "https://ringiflow.example.com/errors/not-found"
      );
      assert_eq!(error.title, "Not Found");
      assert_eq!(error.status, 404);
      assert_eq!(error.detail, "リソースが見つかりません");
   }

   #[test]
   fn test_internal_error_が500と固定detailを返す() {
      let error = ErrorResponse::internal_error();

      assert_eq!(
         error.error_type,
         "https://ringiflow.example.com/errors/internal-error"
      );
      assert_eq!(error.title, "Internal Server Error");
      assert_eq!(error.status, 500);
      assert_eq!(error.detail, "内部エラーが発生しました");
   }

   #[test]
   fn test_jsonシリアライズでtypeフィールド名が正しい() {
      let error = ErrorResponse::bad_request("不正なリクエスト");
      let json = serde_json::to_value(&error).unwrap();

      // serde(rename = "type") で `error_type` → `type` に変換される
      assert_eq!(
         json["type"],
         "https://ringiflow.example.com/errors/bad-request"
      );
      assert_eq!(json["title"], "Bad Request");
      assert_eq!(json["status"], 400);
      assert_eq!(json["detail"], "不正なリクエスト");
      // `error_type` フィールドは存在しない
      assert!(json.get("error_type").is_none());
   }

   #[test]
   fn test_全便利コンストラクタのstatusが正しい() {
      assert_eq!(ErrorResponse::bad_request("").status, 400);
      assert_eq!(ErrorResponse::unauthorized("").status, 401);
      assert_eq!(ErrorResponse::forbidden("").status, 403);
      assert_eq!(ErrorResponse::not_found("").status, 404);
      assert_eq!(ErrorResponse::conflict("").status, 409);
      assert_eq!(ErrorResponse::validation_error("").status, 400);
      assert_eq!(ErrorResponse::internal_error().status, 500);
      assert_eq!(ErrorResponse::service_unavailable("").status, 503);
   }

   #[test]
   fn test_jsonデシリアライズが正しく動作する() {
      let json = r#"{
            "type": "https://ringiflow.example.com/errors/not-found",
            "title": "Not Found",
            "status": 404,
            "detail": "見つかりません"
        }"#;
      let error: ErrorResponse = serde_json::from_str(json).unwrap();

      assert_eq!(
         error.error_type,
         "https://ringiflow.example.com/errors/not-found"
      );
      assert_eq!(error.title, "Not Found");
      assert_eq!(error.status, 404);
      assert_eq!(error.detail, "見つかりません");
   }
}
