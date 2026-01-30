//! # 外部 API クライアント
//!
//! Core Service、Auth Service など外部サービスとの通信を担当する。

pub mod auth_service;
pub mod core_service;

pub use auth_service::{
   AuthServiceClient,
   AuthServiceClientImpl,
   AuthServiceError,
   VerifyResponse,
};
pub use core_service::{
   ApproveRejectRequest,
   CoreServiceClient,
   CoreServiceClientImpl,
   CoreServiceError,
   CreateWorkflowRequest,
   DashboardStatsDto,
   DashboardStatsResponse,
   GetUserByEmailResponse,
   SubmitWorkflowRequest,
   TaskDetailDto,
   TaskDetailResponse,
   TaskItemDto,
   TaskListResponse,
   TaskWorkflowSummaryDto,
   UserResponse,
   UserWithPermissionsResponse,
   WorkflowDefinitionDto,
   WorkflowDefinitionListResponse,
   WorkflowDefinitionResponse,
   WorkflowInstanceDto,
   WorkflowListResponse,
   WorkflowResponse,
   WorkflowStepDto,
};
