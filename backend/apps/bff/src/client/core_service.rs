//! # Core Service クライアント
//!
//! BFF から Core Service への通信を担当する。
//!
//! ## 構成
//!
//! ISP（Interface Segregation Principle）に基づき、5 つのサブトレイトに分割:
//!
//! - [`CoreServiceUserClient`] — ユーザー関連
//! - [`CoreServiceWorkflowClient`] — ワークフロー関連
//! - [`CoreServiceTaskClient`] — タスク・ダッシュボード関連
//! - [`CoreServiceRoleClient`] — ロール管理関連
//! - [`CoreServiceFolderClient`] — フォルダ管理関連
//!
//! [`CoreServiceClient`] はスーパートレイトとして 5 つを束ね、
//! `dyn CoreServiceClient` は引き続き使用可能。
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/40_詳細設計書/08_AuthService設計.md)

mod client_impl;
mod document_client;
mod error;
mod folder_client;
mod response;
mod role_client;
mod task_client;
mod types;
mod user_client;
mod workflow_client;

pub use client_impl::*;
pub use document_client::*;
pub use error::*;
pub use folder_client::*;
pub use role_client::*;
pub use task_client::*;
pub use types::*;
pub use user_client::*;
pub use workflow_client::*;
