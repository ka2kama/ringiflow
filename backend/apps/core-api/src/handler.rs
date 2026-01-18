//! # HTTP リクエストハンドラ
//!
//! axum のルートに対応するハンドラ関数を定義する。
//!
//! ## 設計方針
//!
//! - 各ハンドラはサブモジュールに配置
//! - 親モジュール（この `handler.rs`）で re-export し、フラットな API を提供
//! - ハンドラは薄く保ち、ビジネスロジックはドメイン層に委譲
//!
//! ## 今後追加予定のハンドラ
//!
//! - `workflow`: ワークフロー CRUD
//! - `task`: タスク操作

pub mod auth;
pub mod health;

pub use auth::{AuthState, get_user, verify};
pub use health::health_check;
