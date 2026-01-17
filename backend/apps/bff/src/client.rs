//! # 外部 API クライアント
//!
//! Core API など外部サービスとの通信を担当する。

pub mod core_api;

pub use core_api::{
   CoreApiClient,
   CoreApiClientImpl,
   CoreApiError,
   RoleResponse,
   UserResponse,
   UserWithPermissionsResponse,
   VerifyResponse,
};
