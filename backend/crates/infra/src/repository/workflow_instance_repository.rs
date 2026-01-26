//! # WorkflowInstanceRepository
//!
//! ワークフローインスタンスの永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **テナント分離**: すべてのクエリでテナント ID を考慮
//! - **ステータス管理**: ライフサイクルに応じた状態遷移をサポート
//! - **型安全なクエリ**: sqlx のコンパイル時検証を活用
//!
//! 詳細: [データベース設計](../../../../docs/03_詳細設計書/02_データベース設計.md)

use async_trait::async_trait;
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::Version,
   workflow::{WorkflowDefinitionId, WorkflowInstance, WorkflowInstanceId, WorkflowInstanceStatus},
};
use sqlx::PgPool;

use crate::error::InfraError;

/// ワークフローインスタンスリポジトリトレイト
///
/// ワークフローインスタンスの永続化操作を定義する。
#[async_trait]
pub trait WorkflowInstanceRepository: Send + Sync {
   /// インスタンスを保存（新規作成または更新）
   ///
   /// # 引数
   ///
   /// - `instance`: ワークフローインスタンス
   ///
   /// # 戻り値
   ///
   /// - `Ok(())`: 保存成功
   /// - `Err(_)`: データベースエラー
   async fn save(&self, instance: &WorkflowInstance) -> Result<(), InfraError>;

   /// ID でインスタンスを取得
   ///
   /// # 引数
   ///
   /// - `id`: ワークフローインスタンス ID
   /// - `tenant_id`: テナント ID
   ///
   /// # 戻り値
   ///
   /// - `Ok(Some(instance))`: インスタンスが見つかった場合
   /// - `Ok(None)`: インスタンスが見つからない場合
   /// - `Err(_)`: データベースエラー
   async fn find_by_id(
      &self,
      id: &WorkflowInstanceId,
      tenant_id: &TenantId,
   ) -> Result<Option<WorkflowInstance>, InfraError>;

   /// テナント内のインスタンス一覧を取得
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   ///
   /// # 戻り値
   ///
   /// - `Ok(Vec<WorkflowInstance>)`: インスタンス一覧
   /// - `Err(_)`: データベースエラー
   async fn find_by_tenant(
      &self,
      tenant_id: &TenantId,
   ) -> Result<Vec<WorkflowInstance>, InfraError>;

   /// 申請者によるインスタンス一覧を取得
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `user_id`: ユーザー ID
   ///
   /// # 戻り値
   ///
   /// - `Ok(Vec<WorkflowInstance>)`: インスタンス一覧
   /// - `Err(_)`: データベースエラー
   async fn find_by_initiated_by(
      &self,
      tenant_id: &TenantId,
      user_id: &UserId,
   ) -> Result<Vec<WorkflowInstance>, InfraError>;
}

/// PostgreSQL 実装の WorkflowInstanceRepository
#[derive(Debug, Clone)]
pub struct PostgresWorkflowInstanceRepository {
   pool: PgPool,
}

impl PostgresWorkflowInstanceRepository {
   /// 新しいリポジトリインスタンスを作成
   pub fn new(pool: PgPool) -> Self {
      Self { pool }
   }
}

#[async_trait]
impl WorkflowInstanceRepository for PostgresWorkflowInstanceRepository {
   async fn save(&self, instance: &WorkflowInstance) -> Result<(), InfraError> {
      sqlx::query!(
         r#"
            INSERT INTO workflow_instances (
                id, tenant_id, definition_id, definition_version,
                title, form_data, status, current_step_id,
                initiated_by, submitted_at, completed_at,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO UPDATE SET
                title = EXCLUDED.title,
                form_data = EXCLUDED.form_data,
                status = EXCLUDED.status,
                current_step_id = EXCLUDED.current_step_id,
                submitted_at = EXCLUDED.submitted_at,
                completed_at = EXCLUDED.completed_at,
                updated_at = EXCLUDED.updated_at
            "#,
         instance.id().as_uuid(),
         instance.tenant_id().as_uuid(),
         instance.definition_id().as_uuid(),
         instance.definition_version().as_i32(),
         instance.title(),
         instance.form_data(),
         instance.status().as_str(),
         instance.current_step_id(),
         instance.initiated_by().as_uuid(),
         instance.submitted_at(),
         instance.completed_at(),
         instance.created_at(),
         instance.updated_at()
      )
      .execute(&self.pool)
      .await?;

      Ok(())
   }

   async fn find_by_id(
      &self,
      id: &WorkflowInstanceId,
      tenant_id: &TenantId,
   ) -> Result<Option<WorkflowInstance>, InfraError> {
      let row = sqlx::query!(
         r#"
            SELECT
                id, tenant_id, definition_id, definition_version,
                title, form_data, status, current_step_id,
                initiated_by, submitted_at, completed_at,
                created_at, updated_at
            FROM workflow_instances
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

      let instance = WorkflowInstance::from_db(
         WorkflowInstanceId::from_uuid(row.id),
         TenantId::from_uuid(row.tenant_id),
         WorkflowDefinitionId::from_uuid(row.definition_id),
         Version::new(row.definition_version as u32)
            .map_err(|e| InfraError::Unexpected(e.to_string()))?,
         row.title,
         row.form_data,
         row.status
            .parse::<WorkflowInstanceStatus>()
            .map_err(|e| InfraError::Unexpected(e.to_string()))?,
         row.current_step_id,
         UserId::from_uuid(row.initiated_by),
         row.submitted_at,
         row.completed_at,
         row.created_at,
         row.updated_at,
      );

      Ok(Some(instance))
   }

   async fn find_by_tenant(
      &self,
      tenant_id: &TenantId,
   ) -> Result<Vec<WorkflowInstance>, InfraError> {
      let rows = sqlx::query!(
         r#"
            SELECT
                id, tenant_id, definition_id, definition_version,
                title, form_data, status, current_step_id,
                initiated_by, submitted_at, completed_at,
                created_at, updated_at
            FROM workflow_instances
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
         tenant_id.as_uuid()
      )
      .fetch_all(&self.pool)
      .await?;

      let instances = rows
         .into_iter()
         .map(|row| -> Result<WorkflowInstance, InfraError> {
            Ok(WorkflowInstance::from_db(
               WorkflowInstanceId::from_uuid(row.id),
               TenantId::from_uuid(row.tenant_id),
               WorkflowDefinitionId::from_uuid(row.definition_id),
               Version::new(row.definition_version as u32)
                  .map_err(|e| InfraError::Unexpected(e.to_string()))?,
               row.title,
               row.form_data,
               row.status
                  .parse::<WorkflowInstanceStatus>()
                  .map_err(|e| InfraError::Unexpected(e.to_string()))?,
               row.current_step_id,
               UserId::from_uuid(row.initiated_by),
               row.submitted_at,
               row.completed_at,
               row.created_at,
               row.updated_at,
            ))
         })
         .collect::<Result<Vec<_>, InfraError>>()?;

      Ok(instances)
   }

   async fn find_by_initiated_by(
      &self,
      tenant_id: &TenantId,
      user_id: &UserId,
   ) -> Result<Vec<WorkflowInstance>, InfraError> {
      let rows = sqlx::query!(
         r#"
            SELECT
                id, tenant_id, definition_id, definition_version,
                title, form_data, status, current_step_id,
                initiated_by, submitted_at, completed_at,
                created_at, updated_at
            FROM workflow_instances
            WHERE tenant_id = $1 AND initiated_by = $2
            ORDER BY created_at DESC
            "#,
         tenant_id.as_uuid(),
         user_id.as_uuid()
      )
      .fetch_all(&self.pool)
      .await?;

      let instances = rows
         .into_iter()
         .map(|row| -> Result<WorkflowInstance, InfraError> {
            Ok(WorkflowInstance::from_db(
               WorkflowInstanceId::from_uuid(row.id),
               TenantId::from_uuid(row.tenant_id),
               WorkflowDefinitionId::from_uuid(row.definition_id),
               Version::new(row.definition_version as u32)
                  .map_err(|e| InfraError::Unexpected(e.to_string()))?,
               row.title,
               row.form_data,
               row.status
                  .parse::<WorkflowInstanceStatus>()
                  .map_err(|e| InfraError::Unexpected(e.to_string()))?,
               row.current_step_id,
               UserId::from_uuid(row.initiated_by),
               row.submitted_at,
               row.completed_at,
               row.created_at,
               row.updated_at,
            ))
         })
         .collect::<Result<Vec<_>, InfraError>>()?;

      Ok(instances)
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   /// トレイトオブジェクトとして使用できることを確認
   #[test]
   fn test_トレイトはsendとsyncを実装している() {
      fn assert_send_sync<T: Send + Sync>() {}
      assert_send_sync::<Box<dyn WorkflowInstanceRepository>>();
   }
}
