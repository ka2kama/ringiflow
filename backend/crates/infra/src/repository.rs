//! # リポジトリ実装
//!
//! ドメイン層で定義されたリポジトリトレイトの具体的な実装を提供する。
//!
//! ## 設計方針
//!
//! - **依存性逆転**: ドメイン層のトレイトをインフラ層で実装
//! - **データベース抽象化**: sqlx を使用し、PostgreSQL 固有の処理をカプセル化
//! - **テスタビリティ**: トレイト経由でモック可能な設計

pub mod credentials_repository;
pub mod user_repository;
pub mod workflow_definition_repository;
pub mod workflow_instance_repository;
pub mod workflow_step_repository;

pub use credentials_repository::{
   Credential,
   CredentialType,
   CredentialsRepository,
   PostgresCredentialsRepository,
};
pub use user_repository::{PostgresUserRepository, UserRepository};
pub use workflow_definition_repository::{
   PostgresWorkflowDefinitionRepository,
   WorkflowDefinitionRepository,
};
pub use workflow_instance_repository::{
   PostgresWorkflowInstanceRepository,
   WorkflowInstanceRepository,
};
pub use workflow_step_repository::{PostgresWorkflowStepRepository, WorkflowStepRepository};
