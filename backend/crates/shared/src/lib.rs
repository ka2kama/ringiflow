//! # RingiFlow 共有ユーティリティ
//!
//! このクレートは、RingiFlow
//! プロジェクト全体で使用される共通ユーティリティを提供する。
//!
//! ## 設計方針
//!
//! - 他のすべてのクレート（domain, infra, api）から依存される
//! - ビジネスロジックを含まない純粋なユーティリティのみを配置
//! - 外部クレートへの依存は最小限に抑える

pub mod api_response;
pub mod error_response;
pub mod health;
pub mod observability;
pub mod paginated_response;

pub use api_response::ApiResponse;
pub use error_response::ErrorResponse;
pub use health::HealthResponse;
pub use paginated_response::PaginatedResponse;
