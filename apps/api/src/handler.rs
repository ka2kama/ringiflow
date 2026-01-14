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
//! ## モジュール構成
//!
//! ```text
//! handler.rs          # 親モジュール（re-export）
//! └── handler/
//!     └── health.rs   # ヘルスチェックハンドラ
//! ```
//!
//! ## 今後追加予定のハンドラ
//!
//! - `auth`: 認証関連（ログイン、ログアウト、トークンリフレッシュ）
//! - `workflow`: ワークフロー CRUD
//! - `task`: タスク操作
//! - `document`: ドキュメント管理
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use axum::{routing::get, Router};
//! use ringiflow_api::handler::health_check;
//!
//! let app = Router::new()
//!     .route("/health", get(health_check));
//! ```

pub mod health;

pub use health::health_check;
