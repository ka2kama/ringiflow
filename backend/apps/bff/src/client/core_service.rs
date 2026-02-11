//! # Core Service クライアント
//!
//! BFF から Core Service への通信を担当する。
//!
//! ## 構成
//!
//! ISP（Interface Segregation Principle）に基づき、3 つのサブトレイトに分割:
//!
//! - [`CoreServiceUserClient`] — ユーザー関連（3 メソッド）
//! - [`CoreServiceWorkflowClient`] — ワークフロー関連（12 メソッド）
//! - [`CoreServiceTaskClient`] — タスク・ダッシュボード関連（4 メソッド）
//!
//! [`CoreServiceClient`] はスーパートレイトとして 3 つを束ね、
//! `dyn CoreServiceClient` は引き続き使用可能。
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/08_AuthService設計.md)

mod client_impl;
mod error;
mod response;
mod task_client;
mod types;
mod user_client;
mod workflow_client;

pub use client_impl::*;
pub use error::*;
pub use task_client::*;
pub use types::*;
pub use user_client::*;
pub use workflow_client::*;
