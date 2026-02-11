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

pub mod audit_log;
pub mod auth;
pub mod dashboard;
pub mod health;
pub mod role;
pub mod task;
pub mod user;
pub mod workflow;

pub use audit_log::{AuditLogState, list_audit_logs};
pub use auth::{AuthState, csrf, login, logout, me};
pub use dashboard::get_dashboard_stats;
pub use health::health_check;
pub use role::{RoleState, create_role, delete_role, get_role, list_roles, update_role};
pub use task::list_my_tasks;
pub use user::{
   UserState,
   create_user,
   get_user_detail,
   list_users,
   update_user,
   update_user_status,
};
pub use workflow::{
   WorkflowState,
   approve_step,
   create_workflow,
   get_task_by_display_numbers,
   get_workflow,
   get_workflow_definition,
   list_my_workflows,
   list_workflow_definitions,
   reject_step,
   submit_workflow,
};
