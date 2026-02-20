//! # ワークフロー
//!
//! ワークフロー定義、インスタンス、ステップを管理する。
//!
//! ## 概念モデル
//!
//! - **WorkflowDefinition**: ワークフローのテンプレート（再利用可能）
//! - **WorkflowInstance**: 定義から生成された実行中の案件
//! - **WorkflowStep**: インスタンス内の各承認ステップ
//!
//! ## 使用例
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ringiflow_domain::workflow::{
//!     NewWorkflowDefinition, WorkflowDefinition, WorkflowDefinitionId,
//!     WorkflowDefinitionStatus,
//! };
//! use ringiflow_domain::{tenant::TenantId, user::UserId, value_objects::WorkflowName};
//! use serde_json::json;
//!
//! // ワークフロー定義の作成
//! let definition = WorkflowDefinition::new(NewWorkflowDefinition {
//!     id: WorkflowDefinitionId::new(),
//!     tenant_id: TenantId::new(),
//!     name: WorkflowName::new("汎用申請")?,
//!     description: Some("シンプルな1段階承認".to_string()),
//!     definition: json!({"steps": []}),
//!     created_by: UserId::new(),
//!     now: chrono::Utc::now(),
//! });
//! assert_eq!(definition.status(), WorkflowDefinitionStatus::Draft);
//! # Ok(())
//! # }
//! ```

mod comment;
mod definition;
mod definition_validator;
mod instance;
mod step;

pub use comment::*;
pub use definition::*;
pub use definition_validator::*;
pub use instance::*;
pub use step::*;
