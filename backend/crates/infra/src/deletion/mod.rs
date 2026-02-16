//! # テナントデータ削除基盤
//!
//! テナント退会時のデータ削除を安全に実行するための基盤モジュール。
//!
//! ## 概要
//!
//! 各データストア（PostgreSQL, DynamoDB, Redis）に対応する `TenantDeleter`
//! 実装を `DeletionRegistry` に登録し、一括削除・件数確認を行う。
//!
//! 詳細: [テナント退会時データ削除設計](../../../../docs/03_詳細設計書/06_テナント退会時データ削除設計.md)

mod auth_credentials;
mod dynamodb_audit_log;
mod postgres_simple;
mod postgres_workflow;
mod redis_session;
mod registry;

use async_trait::async_trait;
pub use auth_credentials::AuthCredentialsDeleter;
pub use dynamodb_audit_log::DynamoDbAuditLogDeleter;
pub use postgres_simple::{
    PostgresDisplayIdCounterDeleter,
    PostgresRoleDeleter,
    PostgresUserDeleter,
};
pub use postgres_workflow::PostgresWorkflowDeleter;
pub use redis_session::RedisSessionDeleter;
pub use registry::DeletionRegistry;
use ringiflow_domain::tenant::TenantId;

use crate::error::InfraError;

/// テナントデータの削除結果
#[derive(Debug, Clone)]
pub struct DeletionResult {
    /// 削除された件数
    pub deleted_count: u64,
}

/// テナントデータ削除トレイト
///
/// 各データストアがこのトレイトを実装し、テナント退会時のデータ削除を提供する。
#[async_trait]
pub trait TenantDeleter: Send + Sync {
    /// この Deleter の名前（例: `"postgres:users"`）
    fn name(&self) -> &'static str;

    /// 指定テナントのデータを削除する
    async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError>;

    /// 指定テナントのデータ件数を返す
    async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError>;
}
