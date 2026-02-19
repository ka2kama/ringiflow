//! WorkflowStepRepository: ワークフローステップの永続化
//!
//! ワークフローインスタンスの個々の承認ステップを管理する。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayNumber, Version},
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
use uuid::Uuid;

use crate::error::InfraError;

/// WorkflowStepRepository トレイト
#[async_trait]
pub trait WorkflowStepRepository: Send + Sync {
    /// 新規ステップを作成する
    ///
    /// `tenant_id` は RLS
    /// 二重防御用。ドメインモデルではなくインフラ層で管理する。
    async fn insert(&self, step: &WorkflowStep, tenant_id: &TenantId) -> Result<(), InfraError>;

    /// 楽観的ロック付きでステップを更新する
    ///
    /// `expected_version` と DB 上のバージョンが一致する場合のみ更新する。
    /// 不一致の場合は `InfraError::Conflict` を返す。
    /// `tenant_id` は RLS
    /// 二重防御用。アプリケーション層でもテナント分離を保証する。
    async fn update_with_version_check(
        &self,
        step: &WorkflowStep,
        expected_version: Version,
        tenant_id: &TenantId,
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

    /// 表示用連番でステップを検索する
    ///
    /// `display_number` はインスタンススコープでユニーク。
    async fn find_by_display_number(
        &self,
        display_number: DisplayNumber,
        instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowStep>, InfraError>;
}

/// DB の workflow_steps テーブルの行を表す中間構造体
///
/// `query_as!` マクロが SQL 結果を直接マッピングする対象。
/// `TryFrom` で `WorkflowStep` への変換ロジックを一箇所に集約する。
struct WorkflowStepRow {
    id: Uuid,
    instance_id: Uuid,
    display_number: i64,
    step_id: String,
    step_name: String,
    step_type: String,
    status: String,
    version: i32,
    assigned_to: Option<Uuid>,
    decision: Option<String>,
    comment: Option<String>,
    due_date: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<WorkflowStepRow> for WorkflowStep {
    type Error = InfraError;

    fn try_from(row: WorkflowStepRow) -> Result<Self, Self::Error> {
        Ok(WorkflowStep::from_db(WorkflowStepRecord {
            id: WorkflowStepId::from_uuid(row.id),
            instance_id: WorkflowInstanceId::from_uuid(row.instance_id),
            display_number: DisplayNumber::new(row.display_number)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            step_id: row.step_id,
            step_name: row.step_name,
            step_type: row.step_type,
            status: row
                .status
                .parse::<WorkflowStepStatus>()
                .map_err(|e| InfraError::Unexpected(format!("不正なステータス: {}", e)))?,
            version: Version::new(row.version as u32)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            assigned_to: row.assigned_to.map(UserId::from_uuid),
            decision: row
                .decision
                .as_deref()
                .map(|s| s.parse::<StepDecision>())
                .transpose()
                .map_err(|e| InfraError::Unexpected(format!("不正な判断: {}", e)))?,
            comment: row.comment,
            due_date: row.due_date,
            started_at: row.started_at,
            completed_at: row.completed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
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
    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn insert(&self, step: &WorkflowStep, tenant_id: &TenantId) -> Result<(), InfraError> {
        let status: &str = step.status().into();
        let decision: Option<&str> = step.decision().map(|d| d.into());
        sqlx::query!(
            r#"
         INSERT INTO workflow_steps (
            id, instance_id, tenant_id, display_number, step_id, step_name, step_type,
            status, version, assigned_to, decision, comment,
            due_date, started_at, completed_at,
            created_at, updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
         "#,
            step.id().as_uuid(),
            step.instance_id().as_uuid(),
            tenant_id.as_uuid(),
            step.display_number().as_i64(),
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

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn update_with_version_check(
        &self,
        step: &WorkflowStep,
        expected_version: Version,
        tenant_id: &TenantId,
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
         WHERE id = $8 AND version = $9 AND tenant_id = $10
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
            tenant_id.as_uuid(),
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

    #[tracing::instrument(skip_all, level = "debug", fields(%id, %tenant_id))]
    async fn find_by_id(
        &self,
        id: &WorkflowStepId,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowStep>, InfraError> {
        let row = sqlx::query_as!(
            WorkflowStepRow,
            r#"
         SELECT
            id, instance_id, display_number, step_id, step_name, step_type,
            status, version, assigned_to, decision, comment,
            due_date, started_at, completed_at,
            created_at, updated_at
         FROM workflow_steps
         WHERE id = $1 AND tenant_id = $2
         "#,
            id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(WorkflowStep::try_from).transpose()
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%instance_id, %tenant_id))]
    async fn find_by_instance(
        &self,
        instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowStep>, InfraError> {
        let rows = sqlx::query_as!(
            WorkflowStepRow,
            r#"
         SELECT
            id, instance_id, display_number, step_id, step_name, step_type,
            status, version, assigned_to, decision, comment,
            due_date, started_at, completed_at,
            created_at, updated_at
         FROM workflow_steps
         WHERE instance_id = $1 AND tenant_id = $2
         ORDER BY display_number ASC
         "#,
            instance_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(WorkflowStep::try_from).collect()
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id, %user_id))]
    async fn find_by_assigned_to(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Vec<WorkflowStep>, InfraError> {
        let rows = sqlx::query_as!(
            WorkflowStepRow,
            r#"
         SELECT
            id, instance_id, display_number, step_id, step_name, step_type,
            status, version, assigned_to, decision, comment,
            due_date, started_at, completed_at,
            created_at, updated_at
         FROM workflow_steps
         WHERE tenant_id = $1 AND assigned_to = $2
         ORDER BY created_at DESC
         "#,
            tenant_id.as_uuid(),
            user_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(WorkflowStep::try_from).collect()
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%display_number, %instance_id, %tenant_id))]
    async fn find_by_display_number(
        &self,
        display_number: DisplayNumber,
        instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowStep>, InfraError> {
        let row = sqlx::query_as!(
            WorkflowStepRow,
            r#"
         SELECT
            id, instance_id, display_number, step_id, step_name, step_type,
            status, version, assigned_to, decision, comment,
            due_date, started_at, completed_at,
            created_at, updated_at
         FROM workflow_steps
         WHERE display_number = $1 AND instance_id = $2 AND tenant_id = $3
         "#,
            display_number.as_i64(),
            instance_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(WorkflowStep::try_from).transpose()
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
