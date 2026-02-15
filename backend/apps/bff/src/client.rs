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
    CoreServiceRoleClient,
    CoreServiceTaskClient,
    CoreServiceUserClient,
    CoreServiceWorkflowClient,
    CreateRoleCoreRequest,
    CreateUserCoreRequest,
    CreateUserCoreResponse,
    CreateWorkflowRequest,
    DashboardStatsDto,
    PostCommentCoreRequest,
    ResubmitWorkflowRequest,
    RoleDetailDto,
    RoleItemDto,
    StepApproverRequest,
    SubmitWorkflowRequest,
    TaskDetailDto,
    TaskItemDto,
    TaskWorkflowSummaryDto,
    UpdateRoleCoreRequest,
    UpdateUserCoreRequest,
    UpdateUserStatusCoreRequest,
    UserItemDto,
    UserRefDto,
    UserResponse,
    UserWithPermissionsData,
    WorkflowCommentDto,
    WorkflowDefinitionDto,
    WorkflowInstanceDto,
    WorkflowStepDto,
};
