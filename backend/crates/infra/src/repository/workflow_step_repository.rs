//! WorkflowStepRepository: ワークフローステップの永続化
//!
//! ワークフローインスタンスの個々の承認ステップを管理する。

use std::str::FromStr;

use async_trait::async_trait;
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   workflow::{StepDecision, WorkflowInstanceId, WorkflowStep, WorkflowStepId, WorkflowStepStatus},
};
use sqlx::PgPool;

use crate::error::InfraError;

/// WorkflowStepRepository トレイト
#[async_trait]
pub trait WorkflowStepRepository: Send + Sync {
   /// ステップを保存する（新規作成または更新）
   async fn save(&self, step: &WorkflowStep) -> Result<(), InfraError>;

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
   async fn save(&self, step: &WorkflowStep) -> Result<(), InfraError> {
      sqlx::query!(
         r#"
         INSERT INTO workflow_steps (
            id, instance_id, step_id, step_name, step_type,
            status, assigned_to, decision, comment,
            due_date, started_at, completed_at,
            created_at, updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
         ON CONFLICT (id) DO UPDATE SET
            status = EXCLUDED.status,
            decision = EXCLUDED.decision,
            comment = EXCLUDED.comment,
            started_at = EXCLUDED.started_at,
            completed_at = EXCLUDED.completed_at,
            updated_at = EXCLUDED.updated_at
         "#,
         step.id().as_uuid(),
         step.instance_id().as_uuid(),
         step.step_id(),
         step.step_name(),
         step.step_type(),
         step.status().as_str(),
         step.assigned_to().map(|u| u.as_uuid()),
         step.decision().map(|d| d.as_str()),
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

   async fn find_by_id(
      &self,
      id: &WorkflowStepId,
      tenant_id: &TenantId,
   ) -> Result<Option<WorkflowStep>, InfraError> {
      let row = sqlx::query!(
         r#"
         SELECT
            s.id, s.instance_id, s.step_id, s.step_name, s.step_type,
            s.status, s.assigned_to, s.decision, s.comment,
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

      let step = WorkflowStep::from_db(
         WorkflowStepId::from_uuid(r.id),
         WorkflowInstanceId::from_uuid(r.instance_id),
         r.step_id,
         r.step_name,
         r.step_type,
         WorkflowStepStatus::from_str(&r.status)
            .map_err(|e| InfraError::Unexpected(format!("不正なステータス: {}", e)))?,
         r.assigned_to.map(UserId::from_uuid),
         r.decision
            .as_deref()
            .map(StepDecision::from_str)
            .transpose()
            .map_err(|e| InfraError::Unexpected(format!("不正な判断: {}", e)))?,
         r.comment,
         r.due_date,
         r.started_at,
         r.completed_at,
         r.created_at,
         r.updated_at,
      );

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
            s.status, s.assigned_to, s.decision, s.comment,
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
            Ok(WorkflowStep::from_db(
               WorkflowStepId::from_uuid(r.id),
               WorkflowInstanceId::from_uuid(r.instance_id),
               r.step_id,
               r.step_name,
               r.step_type,
               WorkflowStepStatus::from_str(&r.status)
                  .map_err(|e| InfraError::Unexpected(format!("不正なステータス: {}", e)))?,
               r.assigned_to.map(UserId::from_uuid),
               r.decision
                  .as_deref()
                  .map(StepDecision::from_str)
                  .transpose()
                  .map_err(|e| InfraError::Unexpected(format!("不正な判断: {}", e)))?,
               r.comment,
               r.due_date,
               r.started_at,
               r.completed_at,
               r.created_at,
               r.updated_at,
            ))
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
            s.status, s.assigned_to, s.decision, s.comment,
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
            Ok(WorkflowStep::from_db(
               WorkflowStepId::from_uuid(r.id),
               WorkflowInstanceId::from_uuid(r.instance_id),
               r.step_id,
               r.step_name,
               r.step_type,
               WorkflowStepStatus::from_str(&r.status)
                  .map_err(|e| InfraError::Unexpected(format!("不正なステータス: {}", e)))?,
               r.assigned_to.map(UserId::from_uuid),
               r.decision
                  .as_deref()
                  .map(StepDecision::from_str)
                  .transpose()
                  .map_err(|e| InfraError::Unexpected(format!("不正な判断: {}", e)))?,
               r.comment,
               r.due_date,
               r.started_at,
               r.completed_at,
               r.created_at,
               r.updated_at,
            ))
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
