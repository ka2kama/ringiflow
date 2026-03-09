//! # ページネーション付きレスポンス
//!
//! カーソルベースのページネーションに対応した API レスポンス型。

use serde::{Deserialize, Serialize};

/// ページネーション付きレスポンス
///
/// リスト + カーソルのページネーション形式。
///
/// ## JSON 形式
///
/// ```json
/// {
///   "items": [...],
///   "next_cursor": "opaque-cursor-string"
/// }
/// ```
///
/// `next_cursor` が `null` の場合は最後のページを意味する。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PaginatedResponse<T> {
    pub items:       Vec<T>,
    pub next_cursor: Option<String>,
}
