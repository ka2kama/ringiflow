//! # ユースケース層
//!
//! Core Service のビジネスロジックを実装する。
//!
//! ## 設計方針
//!
//! - **依存性注入**: リポジトリを `Arc<dyn Trait>` で外部から注入
//! - **薄いハンドラ**: ハンドラは薄く保ち、ロジックはユースケースに集約
//!
//! ## モジュール構成
//!
//! - `workflow`: ワークフロー関連のユースケース

pub(crate) mod helpers;

pub mod dashboard;
pub mod role;
pub mod task;
pub mod user;
pub mod workflow;

use std::collections::HashMap;

pub use dashboard::DashboardUseCaseImpl;
use ringiflow_domain::user::UserId;
use ringiflow_infra::repository::UserRepository;
pub use role::RoleUseCaseImpl;
pub use task::TaskUseCaseImpl;
pub use user::UserUseCaseImpl;
pub use workflow::{
    ApproveRejectInput,
    CreateWorkflowInput,
    PostCommentInput,
    ResubmitWorkflowInput,
    StepApprover,
    SubmitWorkflowInput,
    WorkflowUseCaseImpl,
    WorkflowWithSteps,
};

use crate::error::CoreError;

/// ユーザー ID のリストからユーザー名を一括解決する
///
/// 返り値は `UserId → ユーザー名` の HashMap。
/// 空の ID リストを渡した場合は空の HashMap を返す。
pub(crate) async fn resolve_user_names(
    user_repo: &dyn UserRepository,
    user_ids: &[UserId],
) -> Result<HashMap<UserId, String>, CoreError> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let users = user_repo
        .find_by_ids(user_ids)
        .await
        .map_err(|e| CoreError::Internal(e.to_string()))?;

    Ok(users
        .into_iter()
        .map(|user| (user.id().clone(), user.name().as_str().to_string()))
        .collect())
}
