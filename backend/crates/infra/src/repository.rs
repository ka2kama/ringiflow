//! # リポジトリ実装
//!
//! ドメイン層で定義されたリポジトリトレイトの具体的な実装を提供する。
//!
//! ## 設計方針
//!
//! - **依存性逆転**: ドメイン層のトレイトをインフラ層で実装
//! - **データベース抽象化**: sqlx を使用し、PostgreSQL 固有の処理をカプセル化
//! - **テスタビリティ**: トレイト経由でモック可能な設計

pub mod audit_log_repository;
pub mod credentials_repository;
pub mod display_id_counter_repository;
pub mod folder_repository;
pub mod role_repository;
pub mod tenant_repository;
pub mod user_repository;
pub mod workflow_comment_repository;
pub mod workflow_definition_repository;
pub mod workflow_instance_repository;
pub mod workflow_step_repository;

pub use audit_log_repository::{
    AuditLogFilter,
    AuditLogPage,
    AuditLogRepository,
    DynamoDbAuditLogRepository,
};
pub use credentials_repository::{
    Credential,
    CredentialType,
    CredentialsRepository,
    PostgresCredentialsRepository,
};
pub use display_id_counter_repository::{
    DisplayIdCounterRepository,
    PostgresDisplayIdCounterRepository,
};
pub use folder_repository::{FolderRepository, PostgresFolderRepository};
pub use role_repository::{PostgresRoleRepository, RoleRepository};
pub use tenant_repository::{PostgresTenantRepository, TenantRepository};
pub use user_repository::{PostgresUserRepository, UserRepository};
pub use workflow_comment_repository::{
    PostgresWorkflowCommentRepository,
    WorkflowCommentRepository,
};
pub use workflow_definition_repository::{
    PostgresWorkflowDefinitionRepository,
    WorkflowDefinitionRepository,
};
#[cfg(any(test, feature = "test-utils"))]
pub use workflow_instance_repository::WorkflowInstanceRepositoryTestExt;
pub use workflow_instance_repository::{
    PostgresWorkflowInstanceRepository,
    WorkflowInstanceRepository,
};
#[cfg(any(test, feature = "test-utils"))]
pub use workflow_step_repository::WorkflowStepRepositoryTestExt;
pub use workflow_step_repository::{PostgresWorkflowStepRepository, WorkflowStepRepository};
