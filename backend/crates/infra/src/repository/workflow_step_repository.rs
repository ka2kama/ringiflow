//! WorkflowStepRepository: ワークフローステップの永続化
//!
//! ワークフローインスタンスの個々の承認ステップを管理する。

use std::str::FromStr;

use async_trait::async_trait;
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::Version,
   workflow::{
      StepDecision,
      WorkflowInstanceId,
      WorkflowStep,
      WorkflowStepId,
      WorkflowStepRecord,
      WorkflowStepStatus,
   },
};
use sqlx::PgPool;

use crate::error::InfraError;

/// WorkflowStepRepository トレイト
#[async_trait]
pub trait WorkflowStepRepository: Send + Sync {
   /// 新規ステップを作成する
   async fn insert(&self, step: &WorkflowStep) -> Result<(), InfraError>;

   /// 楽観的ロック付きでステップを更新する
   ///
   /// `expected_version` と DB 上のバージョンが一致する場合のみ更新する。
   /// 不一致の場合は `InfraError::Conflict` を返す。
   async fn update_with_version_check(
      &self,
      step: &WorkflowStep,
      expected_version: Version,
   ) -> Result<(), InfraError>;

   /// ID でステップを検索する
   async fn find_by_id(
      &self,
      id: &WorkflowStepId,
      tenant_id: &TenantId,
   ) -> Result<Option<WorkflowStep>, InfraError>;

   /// インスタンスIDでステップ一覧を取得する
   async fn find_by_instance(
      &self,
      instance_id: &WorkflowInstanceId,
      tenant_id: &TenantId,
   ) -> Result<Vec<WorkflowStep>, InfraError>;

   /// 担当者でステップ一覧を取得する（タスク一覧用）
   async fn find_by_assigned_to(
      &self,
      tenant_id: &TenantId,
      user_id: &UserId,
   ) -> Result<Vec<WorkflowStep>, InfraError>;
}

/// PostgreSQL 実装
pub struct PostgresWorkflowStepRepository {
   pool: PgPool,
}

impl PostgresWorkflowStepRepository {
   pub fn new(pool: PgPool) -> Self {
      Self { pool }
   }
}

#[async_trait]
impl WorkflowStepRepository for PostgresWorkflowStepRepository {
   async fn insert(&self, step: &WorkflowStep) -> Result<(), InfraError> {
      let status: &str = step.status().into();
      let decision: Option<&str> = step.decision().map(|d| d.into());
      sqlx::query!(
         r#"
         INSERT INTO workflow_steps (
            id, instance_id, step_id, step_name, step_type,
            status, version, assigned_to, decision, comment,
            due_date, started_at, completed_at,
            created_at, updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         "#,
         step.id().as_uuid(),
         step.instance_id().as_uuid(),
         step.step_id(),
         step.step_name(),
         step.step_type(),
         status,
         step.version().as_i32(),
         step.assigned_to().map(|u| u.as_uuid()),
         decision,
         step.comment(),
         step.due_date(),
         step.started_at(),
         step.completed_at(),
         step.created_at(),
         step.updated_at(),
      )
      .execute(&self.pool)
      .await?;

      Ok(())
   }

   async fn update_with_version_check(
      &self,
      step: &WorkflowStep,
      expected_version: Version,
   ) -> Result<(), InfraError> {
      let status: &str = step.status().into();
      let decision: Option<&str> = step.decision().map(|d| d.into());
      let result = sqlx::query!(
         r#"
         UPDATE workflow_steps SET
            status = $1,
            version = $2,
            decision = $3,
            comment = $4,
            started_at = $5,
            completed_at = $6,
            updated_at = $7
         WHERE id = $8 AND version = $9
         "#,
         status,
         step.version().as_i32(),
         decision,
         step.comment(),
         step.started_at(),
         step.completed_at(),
         step.updated_at(),
         step.id().as_uuid(),
         expected_version.as_i32(),
      )
      .execute(&self.pool)
      .await?;

      if result.rows_affected() == 0 {
         return Err(InfraError::Conflict {
            entity: "WorkflowStep".to_string(),
            id:     step.id().as_uuid().to_string(),
         });
      }

      Ok(())
   }

   async fn find_by_id(
      &self,
      id: &WorkflowStepId,
      tenant_id: &TenantId,
   ) -> Result<Option<WorkflowStep>, InfraError> {
      let row = sqlx::query!(
         r#"
         SELECT
            s.id, s.instance_id, s.step_id, s.step_name, s.step_type,
            s.status, s.version, s.assigned_to, s.decision, s.comment,
            s.due_date, s.started_at, s.completed_at,
            s.created_at, s.updated_at
         FROM workflow_steps s
         INNER JOIN workflow_instances i ON s.instance_id = i.id
         WHERE s.id = $1 AND i.tenant_id = $2
         "#,
         id.as_uuid(),
         tenant_id.as_uuid()
      )
      .fetch_optional(&self.pool)
      .await?;

      let Some(r) = row else {
         return Ok(None);
      };

      let step = WorkflowStep::from_db(WorkflowStepRecord {
         id:           WorkflowStepId::from_uuid(r.id),
         instance_id:  WorkflowInstanceId::from_uuid(r.instance_id),
         step_id:      r.step_id,
         step_name:    r.step_name,
         step_type:    r.step_type,
         status:       WorkflowStepStatus::from_str(&r.status)
            .map_err(|e| InfraError::Unexpected(format!("不正なステータス: {}", e)))?,
         version:      Version::new(r.version as u32)
            .map_err(|e| InfraError::Unexpected(e.to_string()))?,
         assigned_to:  r.assigned_to.map(UserId::from_uuid),
         decision:     r
            .decision
            .as_deref()
            .map(StepDecision::from_str)
            .transpose()
            .map_err(|e| InfraError::Unexpected(format!("不正な判断: {}", e)))?,
         comment:      r.comment,
         due_date:     r.due_date,
         started_at:   r.started_at,
         completed_at: r.completed_at,
         created_at:   r.created_at,
         updated_at:   r.updated_at,
      });

      Ok(Some(step))
   }

   async fn find_by_instance(
      &self,
      instance_id: &WorkflowInstanceId,
      tenant_id: &TenantId,
   ) -> Result<Vec<WorkflowStep>, InfraError> {
      let rows = sqlx::query!(
         r#"
         SELECT
            s.id, s.instance_id, s.step_id, s.step_name, s.step_type,
            s.status, s.version, s.assigned_to, s.decision, s.comment,
            s.due_date, s.started_at, s.completed_at,
            s.created_at, s.updated_at
         FROM workflow_steps s
         INNER JOIN workflow_instances i ON s.instance_id = i.id
         WHERE s.instance_id = $1 AND i.tenant_id = $2
         ORDER BY s.created_at ASC
         "#,
         instance_id.as_uuid(),
         tenant_id.as_uuid()
      )
      .fetch_all(&self.pool)
      .await?;

      rows
         .into_iter()
         .map(|r| -> Result<WorkflowStep, InfraError> {
            Ok(WorkflowStep::from_db(WorkflowStepRecord {
               id:           WorkflowStepId::from_uuid(r.id),
               instance_id:  WorkflowInstanceId::from_uuid(r.instance_id),
               step_id:      r.step_id,
               step_name:    r.step_name,
               step_type:    r.step_type,
               status:       WorkflowStepStatus::from_str(&r.status)
                  .map_err(|e| InfraError::Unexpected(format!("不正なステータス: {}", e)))?,
               version:      Version::new(r.version as u32)
                  .map_err(|e| InfraError::Unexpected(e.to_string()))?,
               assigned_to:  r.assigned_to.map(UserId::from_uuid),
               decision:     r
                  .decision
                  .as_deref()
                  .map(StepDecision::from_str)
                  .transpose()
                  .map_err(|e| InfraError::Unexpected(format!("不正な判断: {}", e)))?,
               comment:      r.comment,
               due_date:     r.due_date,
               started_at:   r.started_at,
               completed_at: r.completed_at,
               created_at:   r.created_at,
               updated_at:   r.updated_at,
            }))
         })
         .collect()
   }

   async fn find_by_assigned_to(
      &self,
      tenant_id: &TenantId,
      user_id: &UserId,
   ) -> Result<Vec<WorkflowStep>, InfraError> {
      let rows = sqlx::query!(
         r#"
         SELECT
            s.id, s.instance_id, s.step_id, s.step_name, s.step_type,
            s.status, s.version, s.assigned_to, s.decision, s.comment,
            s.due_date, s.started_at, s.completed_at,
            s.created_at, s.updated_at
         FROM workflow_steps s
         INNER JOIN workflow_instances i ON s.instance_id = i.id
         WHERE i.tenant_id = $1 AND s.assigned_to = $2
         ORDER BY s.created_at DESC
         "#,
         tenant_id.as_uuid(),
         user_id.as_uuid()
      )
      .fetch_all(&self.pool)
      .await?;

      rows
         .into_iter()
         .map(|r| -> Result<WorkflowStep, InfraError> {
            Ok(WorkflowStep::from_db(WorkflowStepRecord {
               id:           WorkflowStepId::from_uuid(r.id),
               instance_id:  WorkflowInstanceId::from_uuid(r.instance_id),
               step_id:      r.step_id,
               step_name:    r.step_name,
               step_type:    r.step_type,
               status:       WorkflowStepStatus::from_str(&r.status)
                  .map_err(|e| InfraError::Unexpected(format!("不正なステータス: {}", e)))?,
               version:      Version::new(r.version as u32)
                  .map_err(|e| InfraError::Unexpected(e.to_string()))?,
               assigned_to:  r.assigned_to.map(UserId::from_uuid),
               decision:     r
                  .decision
                  .as_deref()
                  .map(StepDecision::from_str)
                  .transpose()
                  .map_err(|e| InfraError::Unexpected(format!("不正な判断: {}", e)))?,
               comment:      r.comment,
               due_date:     r.due_date,
               started_at:   r.started_at,
               completed_at: r.completed_at,
               created_at:   r.created_at,
               updated_at:   r.updated_at,
            }))
         })
         .collect()
   }
}

// Send + Sync 検証
#[cfg(test)]
mod tests {
   use super::*;

   fn assert_send_sync<T: Send + Sync>() {}

   #[test]
   fn test_トレイトはsendとsyncを実装している() {
      assert_send_sync::<Box<dyn WorkflowStepRepository>>();
   }
}
