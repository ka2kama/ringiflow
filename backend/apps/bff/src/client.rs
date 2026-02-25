//! # 外部 API クライアント
//!
//! Core Service、Auth Service など外部サービスとの通信を担当する。

pub mod auth_service;
pub mod core_service;

pub use auth_service::{
    AuthServiceClient,
    AuthServiceClientImpl,
    AuthServiceError,
    CreateCredentialsResponse,
    VerifyResponse,
};
pub use core_service::{
    ApproveRejectRequest,
    CoreServiceClient,
    CoreServiceClientImpl,
    CoreServiceError,
    CoreServiceFolderClient,
    CoreServiceRoleClient,
    CoreServiceTaskClient,
    CoreServiceUserClient,
    CoreServiceWorkflowClient,
    CreateDefinitionCoreRequest,
    CreateFolderCoreRequest,
    CreateRoleCoreRequest,
    CreateUserCoreRequest,
    CreateUserCoreResponse,
    CreateWorkflowRequest,
    DashboardStatsDto,
    FolderItemDto,
    PostCommentCoreRequest,
    PublishArchiveCoreRequest,
    ResubmitWorkflowRequest,
    RoleDetailDto,
    RoleItemDto,
    StepApproverRequest,
    SubmitWorkflowRequest,
    TaskDetailDto,
    TaskItemDto,
    TaskWorkflowSummaryDto,
    UpdateDefinitionCoreRequest,
    UpdateFolderCoreRequest,
    UpdateRoleCoreRequest,
    UpdateUserCoreRequest,
    UpdateUserStatusCoreRequest,
    UserItemDto,
    UserRefDto,
    UserResponse,
    UserWithPermissionsData,
    ValidateDefinitionCoreRequest,
    ValidationErrorDto,
    ValidationResultDto,
    WorkflowCommentDto,
    WorkflowDefinitionDto,
    WorkflowInstanceDto,
    WorkflowStepDto,
};
