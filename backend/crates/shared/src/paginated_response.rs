//! # ページネーション付きレスポンス
//!
//! カーソルベースのページネーションに対応した API レスポンス型。

use serde::{Deserialize, Serialize};

/// ページネーション付きレスポンス
///
/// `ApiResponse<T>` が単一データ用であるのに対し、
/// `PaginatedResponse<T>` はリスト + カーソルのページネーション形式。
///
/// ## JSON 形式
///
/// ```json
/// {
///   "data": [...],
///   "next_cursor": "opaque-cursor-string"
/// }
/// ```
///
/// `next_cursor` が `null` の場合は最後のページを意味する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
   pub data:        Vec<T>,
   pub next_cursor: Option<String>,
}
