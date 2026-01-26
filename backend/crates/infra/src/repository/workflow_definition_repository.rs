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
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::{Version, WorkflowName},
   workflow::{WorkflowDefinition, WorkflowDefinitionId, WorkflowDefinitionStatus},
};
use sqlx::PgPool;

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
      let rows = sqlx::query!(
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

      let definitions = rows
         .into_iter()
         .map(|row| -> Result<WorkflowDefinition, InfraError> {
            Ok(WorkflowDefinition::from_db(
               WorkflowDefinitionId::from_uuid(row.id),
               TenantId::from_uuid(row.tenant_id),
               WorkflowName::new(&row.name).map_err(|e| InfraError::Unexpected(e.to_string()))?,
               row.description,
               Version::new(row.version as u32)
                  .map_err(|e| InfraError::Unexpected(e.to_string()))?,
               row.definition,
               row.status
                  .parse::<WorkflowDefinitionStatus>()
                  .map_err(|e| InfraError::Unexpected(e.to_string()))?,
               UserId::from_uuid(row.created_by),
               row.created_at,
               row.updated_at,
            ))
         })
         .collect::<Result<Vec<_>, InfraError>>()?;

      Ok(definitions)
   }

   async fn find_by_id(
      &self,
      id: &WorkflowDefinitionId,
      tenant_id: &TenantId,
   ) -> Result<Option<WorkflowDefinition>, InfraError> {
      let row = sqlx::query!(
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

      let Some(row) = row else {
         return Ok(None);
      };

      let definition = WorkflowDefinition::from_db(
         WorkflowDefinitionId::from_uuid(row.id),
         TenantId::from_uuid(row.tenant_id),
         WorkflowName::new(&row.name).map_err(|e| InfraError::Unexpected(e.to_string()))?,
         row.description,
         Version::new(row.version as u32).map_err(|e| InfraError::Unexpected(e.to_string()))?,
         row.definition,
         row.status
            .parse::<WorkflowDefinitionStatus>()
            .map_err(|e| InfraError::Unexpected(e.to_string()))?,
         UserId::from_uuid(row.created_by),
         row.created_at,
         row.updated_at,
      );

      Ok(Some(definition))
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
