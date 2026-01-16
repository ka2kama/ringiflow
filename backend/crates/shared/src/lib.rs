//! # RingiFlow 共有ユーティリティ
//!
//! このクレートは、RingiFlow プロジェクト全体で使用される共通ユーティリティを提供する。
//!
//! ## 設計方針
//!
//! - 他のすべてのクレート（domain, infra, api）から依存される
//! - ビジネスロジックを含まない純粋なユーティリティのみを配置
//! - 外部クレートへの依存は最小限に抑える
//!
//! ## モジュール構成
//!
//! - [`correlation_id`] - リクエスト追跡用の一意識別子
//! - [`datetime`] - 日時操作のユーティリティ
//!
//! ## 使用例
//!
//! ```rust
//! use ringiflow_shared::CorrelationId;
//!
//! let id = CorrelationId::new();
//! println!("Correlation ID: {}", id);
//! ```

pub mod correlation_id;
pub mod datetime;

pub use correlation_id::CorrelationId;
