//! # HTTP リクエストハンドラ
//!
//! axum のルートに対応するハンドラ関数を定義する。
//!
//! ## 設計方針
//!
//! - 各ハンドラはサブモジュールに配置
//! - 親モジュールで re-export し、フラットな API を提供
//! - ハンドラは薄く保ち、ビジネスロジックは Core API に委譲
//!
//! ## ハンドラ一覧
//!
//! - `health`: ヘルスチェック
//! - `auth`: 認証関連（ログイン、ログアウト）
//! - `workflow`: ワークフロー関連（作成、申請）

pub mod auth;
pub mod health;
pub mod workflow;

pub use auth::{AuthState, csrf, login, logout, me};
pub use health::health_check;
pub use workflow::{
   WorkflowState,
   approve_step,
   create_workflow,
   get_workflow,
   get_workflow_definition,
   list_my_workflows,
   list_workflow_definitions,
   reject_step,
   submit_workflow,
};
