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
pub mod role;
pub mod task;
pub mod workflow;

pub use auth::{
   UserState,
   create_user,
   get_user,
   get_user_by_display_number,
   get_user_by_email,
   list_users,
   update_user,
   update_user_status,
};
pub use dashboard::{DashboardState, get_dashboard_stats};
pub use health::health_check;
pub use role::{RoleState, create_role, delete_role, get_role, list_roles, update_role};
pub use task::{TaskState, get_task, get_task_by_display_numbers, list_my_tasks};
pub use workflow::{
   WorkflowState,
   approve_step,
   approve_step_by_display_number,
   create_workflow,
   get_workflow,
   get_workflow_by_display_number,
   get_workflow_definition,
   list_comments,
   list_my_workflows,
   list_workflow_definitions,
   post_comment,
   reject_step,
   reject_step_by_display_number,
   request_changes_step,
   request_changes_step_by_display_number,
   resubmit_workflow,
   resubmit_workflow_by_display_number,
   submit_workflow,
   submit_workflow_by_display_number,
};
