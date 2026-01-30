//! # API レスポンスエンベロープ
//!
//! 公開 API の統一レスポンス形式 `{ "data": T }` を提供する。

use serde::{Deserialize, Serialize};

/// 公開 API の統一レスポンス型
///
/// すべての公開 API エンドポイントは `{ "data": T }` 形式でレスポンスを返す。
/// この型は以下の場所で使用される:
/// - Core Service ハンドラ（Serialize でレスポンスを返す）
/// - BFF ハンドラ（Serialize でクライアントにレスポンスを返す）
/// - BFF クライアント（Deserialize で Core Service のレスポンスを受け取る）
///
/// ## 使用例
///
/// ```
/// use ringiflow_shared::ApiResponse;
///
/// let response = ApiResponse::new("hello");
/// assert_eq!(response.data, "hello");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiResponse<T> {
   pub data: T,
}

impl<T> ApiResponse<T> {
   /// 新しい `ApiResponse` を作成する
   pub fn new(data: T) -> Self {
      Self { data }
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn serialize_を正しいjson形状にする() {
      let response = ApiResponse::new("hello");
      let json = serde_json::to_value(&response).unwrap();

      assert_eq!(json, serde_json::json!({ "data": "hello" }));
   }

   #[test]
   fn deserialize_でjsonからオブジェクトに変換する() {
      let json = r#"{"data": "world"}"#;
      let response: ApiResponse<String> = serde_json::from_str(json).unwrap();

      assert_eq!(response.data, "world");
   }

   #[test]
   fn serialize_deserialize_のラウンドトリップ() {
      let original = ApiResponse::new(42);
      let json = serde_json::to_string(&original).unwrap();
      let deserialized: ApiResponse<i32> = serde_json::from_str(&json).unwrap();

      assert_eq!(original, deserialized);
   }

   #[test]
   fn vec_ペイロードをシリアライズする() {
      let response = ApiResponse::new(vec!["a", "b", "c"]);
      let json = serde_json::to_value(&response).unwrap();

      assert_eq!(json, serde_json::json!({ "data": ["a", "b", "c"] }));
   }
}
