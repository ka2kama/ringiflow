//! # HTTP リクエストハンドラ
//!
//! axum のルートに対応するハンドラ関数を定義する。
//!
//! ## 設計方針
//!
//! - 各ハンドラはサブモジュールに配置
//! - 親モジュールで re-export し、フラットな API を提供
//! - ハンドラは薄く保ち、ビジネスロジックは usecase 層に委譲
//!
//! ## ハンドラ一覧
//!
//! - `health`: ヘルスチェック
//! - `auth`: 認証関連（verify, credentials CRUD）

pub mod auth;
pub mod health;

pub use auth::{AuthState, create_credentials, delete_credentials, verify};
pub use health::{ReadinessState, health_check, readiness_check};
