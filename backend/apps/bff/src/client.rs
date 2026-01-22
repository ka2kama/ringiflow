//! # 外部 API クライアント
//!
//! Core API、Auth Service など外部サービスとの通信を担当する。

pub mod auth_service;
pub mod core_api;

pub use auth_service::{
   AuthServiceClient,
   AuthServiceClientImpl,
   AuthServiceError,
   VerifyResponse,
};
pub use core_api::{
   CoreApiClient,
   CoreApiClientImpl,
   CoreApiError,
   GetUserByEmailResponse,
   UserResponse,
   UserWithPermissionsResponse,
};
