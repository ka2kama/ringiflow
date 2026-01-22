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
   CoreServiceClient,
   CoreServiceClientImpl,
   CoreServiceError,
   GetUserByEmailResponse,
   UserResponse,
   UserWithPermissionsResponse,
};
