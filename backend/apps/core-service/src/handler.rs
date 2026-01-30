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
//! - `task`: タスク操作

pub mod auth;
pub mod dashboard;
pub mod health;
pub mod task;
pub mod workflow;

pub use auth::{UserState, get_user, get_user_by_email};
pub use dashboard::{DashboardState, get_dashboard_stats};
pub use health::health_check;
pub use task::{TaskState, get_task, list_my_tasks};
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
