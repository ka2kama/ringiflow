//! # リクエスト追跡用の Correlation ID
//!
//! 分散システムにおいて、複数のサービスをまたぐリクエストを追跡するための識別子。
//! ログやトレーシングで使用し、問題発生時のデバッグを容易にする。
//!
//! ## 設計判断
//!
//! - **Newtype パターン**: `String` をラップすることで型安全性を確保
//! - **UUID v7 採用**: タイムスタンプを含むため時系列でソート可能
//! - **文字列表現**: HTTP ヘッダでの伝播を考慮し、内部は文字列で保持
//!
//! ## 使用例
//!
//! ```rust
//! use ringiflow_shared::CorrelationId;
//!
//! // 新規生成（UUID v7 ベース）
//! let id = CorrelationId::new();
//!
//! // 外部から受け取った値で作成（例: HTTP ヘッダから）
//! let id = CorrelationId::from_string("existing-correlation-id");
//!
//! // ログ出力
//! tracing::info!(correlation_id = %id, "リクエスト処理開始");
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// リクエスト追跡用の一意識別子
///
/// 分散システムにおいて、サービス間でリクエストを追跡するために使用する。
/// HTTP ヘッダ `X-Correlation-ID` として伝播させることを想定。
///
/// # 型安全性
///
/// Newtype パターンにより、他の文字列型と混同されることを防ぐ。
/// 例えば、`TenantId` や `UserId` と取り違えるバグをコンパイル時に検出できる。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(String);

impl CorrelationId {
   /// 新しい Correlation ID を生成する
   ///
   /// UUID v7 を使用するため、生成順にソート可能。
   /// これにより、ログの時系列分析が容易になる。
   ///
   /// # 例
   ///
   /// ```rust
   /// use ringiflow_shared::CorrelationId;
   ///
   /// let id = CorrelationId::new();
   /// assert!(!id.as_str().is_empty());
   /// ```
   pub fn new() -> Self {
      Self(Uuid::now_v7().to_string())
   }

   /// 文字列から Correlation ID を作成する
   ///
   /// 外部システムから受け取った値（HTTP ヘッダなど）を
   /// 型安全な `CorrelationId` に変換する際に使用する。
   ///
   /// # 引数
   ///
   /// * `s` - 任意の文字列型（`String`, `&str` など）
   ///
   /// # 例
   ///
   /// ```rust
   /// use ringiflow_shared::CorrelationId;
   ///
   /// // &str から作成
   /// let id1 = CorrelationId::from_string("abc-123");
   ///
   /// // String から作成
   /// let id2 = CorrelationId::from_string(String::from("def-456"));
   /// ```
   pub fn from_string(s: impl Into<String>) -> Self {
      Self(s.into())
   }

   /// 内部の文字列参照を取得する
   ///
   /// HTTP レスポンスヘッダへの設定や、ログ出力時に使用する。
   ///
   /// # 例
   ///
   /// ```rust
   /// use ringiflow_shared::CorrelationId;
   ///
   /// let id = CorrelationId::from_string("test-id");
   /// assert_eq!(id.as_str(), "test-id");
   /// ```
   pub fn as_str(&self) -> &str {
      &self.0
   }
}

impl Default for CorrelationId {
   /// デフォルトで新しい Correlation ID を生成する
   fn default() -> Self {
      Self::new()
   }
}

impl fmt::Display for CorrelationId {
   /// 人間可読な形式で出力する
   ///
   /// `tracing` マクロの `%` フォーマッタで使用される。
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
   }
}

#[cfg(test)]
mod tests {
   use pretty_assertions::{assert_eq, assert_ne};

   use super::*;

   #[test]
   fn test_correlation_id_new_generates_unique_ids() {
      // 連続して生成した ID は異なる値になること
      let id1 = CorrelationId::new();
      let id2 = CorrelationId::new();
      assert_ne!(id1, id2);
   }

   #[test]
   fn test_correlation_id_from_string_preserves_value() {
      let id = CorrelationId::from_string("test-correlation-id");
      assert_eq!(id.as_str(), "test-correlation-id");
   }

   #[test]
   fn test_correlation_id_display() {
      let id = CorrelationId::from_string("display-test");
      assert_eq!(format!("{}", id), "display-test");
   }

   #[test]
   fn test_correlation_id_equality() {
      let id1 = CorrelationId::from_string("same-id");
      let id2 = CorrelationId::from_string("same-id");
      assert_eq!(id1, id2);
   }
}
