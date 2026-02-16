//! # WorkflowDefinitionRepository
//!
//! ワークフロー定義の永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **テナント分離**: すべてのクエリでテナント ID を考慮
//! - **公開済みのみ取得**: status = 'published' の定義のみを返す
//! - **型安全なクエリ**: sqlx のコンパイル時検証を活用
//!
//! 詳細: [データベース設計](../../../../docs/03_詳細設計書/02_データベース設計.md)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::{Version, WorkflowName},
    workflow::{
        WorkflowDefinition,
        WorkflowDefinitionId,
        WorkflowDefinitionRecord,
        WorkflowDefinitionStatus,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::InfraError;

/// ワークフロー定義リポジトリトレイト
///
/// ワークフロー定義の永続化操作を定義する。
#[async_trait]
pub trait WorkflowDefinitionRepository: Send + Sync {
    /// 公開されている定義の一覧を取得（テナント内）
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    ///
    /// # 戻り値
    ///
    /// - `Ok(Vec<WorkflowDefinition>)`: 定義一覧
    /// - `Err(_)`: データベースエラー
    async fn find_published_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowDefinition>, InfraError>;

    /// ID で定義を取得
    ///
    /// # 引数
    ///
    /// - `id`: ワークフロー定義 ID
    /// - `tenant_id`: テナント ID
    ///
    /// # 戻り値
    ///
    /// - `Ok(Some(definition))`: 定義が見つかった場合
    /// - `Ok(None)`: 定義が見つからない場合
    /// - `Err(_)`: データベースエラー
    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowDefinition>, InfraError>;
}

/// DB の workflow_definitions テーブルの行を表す中間構造体
///
/// `query_as!` マクロが SQL 結果を直接マッピングする対象。
/// `TryFrom` で `WorkflowDefinition` への変換ロジックを一箇所に集約する。
struct WorkflowDefinitionRow {
    id:          Uuid,
    tenant_id:   Uuid,
    name:        String,
    description: Option<String>,
    version:     i32,
    definition:  serde_json::Value,
    status:      String,
    created_by:  Uuid,
    created_at:  DateTime<Utc>,
    updated_at:  DateTime<Utc>,
}

impl TryFrom<WorkflowDefinitionRow> for WorkflowDefinition {
    type Error = InfraError;

    fn try_from(row: WorkflowDefinitionRow) -> Result<Self, Self::Error> {
        Ok(WorkflowDefinition::from_db(WorkflowDefinitionRecord {
            id:          WorkflowDefinitionId::from_uuid(row.id),
            tenant_id:   TenantId::from_uuid(row.tenant_id),
            name:        WorkflowName::new(&row.name)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            description: row.description,
            version:     Version::new(row.version as u32)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            definition:  row.definition,
            status:      row
                .status
                .parse::<WorkflowDefinitionStatus>()
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            created_by:  UserId::from_uuid(row.created_by),
            created_at:  row.created_at,
            updated_at:  row.updated_at,
        }))
    }
}

/// PostgreSQL 実装の WorkflowDefinitionRepository
#[derive(Debug, Clone)]
pub struct PostgresWorkflowDefinitionRepository {
    pool: PgPool,
}

impl PostgresWorkflowDefinitionRepository {
    /// 新しいリポジトリインスタンスを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WorkflowDefinitionRepository for PostgresWorkflowDefinitionRepository {
    async fn find_published_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowDefinition>, InfraError> {
        let rows = sqlx::query_as!(
            WorkflowDefinitionRow,
            r#"
            SELECT
                id,
                tenant_id,
                name,
                description,
                version,
                definition,
                status,
                created_by,
                created_at,
                updated_at
            FROM workflow_definitions
            WHERE tenant_id = $1 AND status = 'published'
            ORDER BY created_at DESC
            "#,
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(WorkflowDefinition::try_from).collect()
    }

    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowDefinition>, InfraError> {
        let row = sqlx::query_as!(
            WorkflowDefinitionRow,
            r#"
            SELECT
                id,
                tenant_id,
                name,
                description,
                version,
                definition,
                status,
                created_by,
                created_at,
                updated_at
            FROM workflow_definitions
            WHERE id = $1 AND tenant_id = $2
            "#,
            id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(WorkflowDefinition::try_from).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// トレイトオブジェクトとして使用できることを確認
    #[test]
    fn test_トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn WorkflowDefinitionRepository>>();
    }
}
