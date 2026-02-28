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
//! 詳細: [データベース設計](../../../../docs/40_詳細設計書/02_データベース設計.md)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayNumber, Version},
    workflow::{
        WorkflowDefinitionId,
        WorkflowInstance,
        WorkflowInstanceId,
        WorkflowInstanceRecord,
        WorkflowInstanceStatus,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{db::TxContext, error::InfraError};

/// ワークフローインスタンスリポジトリトレイト
///
/// ワークフローインスタンスの永続化操作を定義する。
#[async_trait]
pub trait WorkflowInstanceRepository: Send + Sync {
    /// 新規インスタンスを作成する
    ///
    /// # 引数
    ///
    /// - `tx`: トランザクションコンテキスト（構造的強制）
    /// - `instance`: ワークフローインスタンス
    ///
    /// # 戻り値
    ///
    /// - `Ok(())`: 作成成功
    /// - `Err(_)`: データベースエラー（重複 ID の場合を含む）
    async fn insert(
        &self,
        tx: &mut TxContext,
        instance: &WorkflowInstance,
    ) -> Result<(), InfraError>;

    /// 楽観的ロック付きでインスタンスを更新する
    ///
    /// `expected_version` と DB 上のバージョンが一致する場合のみ更新する。
    /// 不一致の場合は `InfraError::Conflict` を返す。
    /// `tx` はトランザクションコンテキスト（構造的強制）。
    /// `tenant_id` は RLS
    /// 二重防御用。アプリケーション層でもテナント分離を保証する。
    ///
    /// # 引数
    ///
    /// - `tx`: トランザクションコンテキスト（構造的強制）
    /// - `instance`: 更新後のワークフローインスタンス
    /// - `expected_version`: 読み取り時のバージョン（DB
    ///   上の現在値と一致すべき値）
    /// - `tenant_id`: テナント ID（RLS 二重防御）
    ///
    /// # エラー
    ///
    /// - `InfraError::Conflict`:
    ///   バージョン不一致（別のリクエストが先に更新済み）
    /// - `InfraError::Database`: データベースエラー
    async fn update_with_version_check(
        &self,
        tx: &mut TxContext,
        instance: &WorkflowInstance,
        expected_version: Version,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError>;

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

    /// 複数 ID によるインスタンス一覧を取得
    ///
    /// タスク一覧画面でワークフロータイトルを表示するために使用する。
    /// 存在しない ID は無視し、見つかったインスタンスのみ返す。
    ///
    /// # 引数
    ///
    /// - `ids`: ワークフローインスタンス ID の一覧
    /// - `tenant_id`: テナント ID
    ///
    /// # 戻り値
    ///
    /// - `Ok(Vec<WorkflowInstance>)`: 見つかったインスタンス一覧
    /// - `Err(_)`: データベースエラー
    async fn find_by_ids(
        &self,
        ids: &[WorkflowInstanceId],
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowInstance>, InfraError>;

    /// 表示用連番でインスタンスを取得
    ///
    /// # 引数
    ///
    /// - `display_number`: 表示用連番
    /// - `tenant_id`: テナント ID
    ///
    /// # 戻り値
    ///
    /// - `Ok(Some(instance))`: インスタンスが見つかった場合
    /// - `Ok(None)`: インスタンスが見つからない場合
    /// - `Err(_)`: データベースエラー
    async fn find_by_display_number(
        &self,
        display_number: DisplayNumber,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowInstance>, InfraError>;
}

/// DB の workflow_instances テーブルの行を表す中間構造体
///
/// `query_as!` マクロが SQL 結果を直接マッピングする対象。
/// `TryFrom` で `WorkflowInstance` への変換ロジックを一箇所に集約する。
struct WorkflowInstanceRow {
    id: Uuid,
    tenant_id: Uuid,
    definition_id: Uuid,
    definition_version: i32,
    display_number: i64,
    title: String,
    form_data: serde_json::Value,
    status: String,
    version: i32,
    current_step_id: Option<String>,
    initiated_by: Uuid,
    submitted_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<WorkflowInstanceRow> for WorkflowInstance {
    type Error = InfraError;

    fn try_from(row: WorkflowInstanceRow) -> Result<Self, Self::Error> {
        WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: WorkflowInstanceId::from_uuid(row.id),
            tenant_id: TenantId::from_uuid(row.tenant_id),
            definition_id: WorkflowDefinitionId::from_uuid(row.definition_id),
            definition_version: Version::new(row.definition_version as u32)
                .map_err(|e| InfraError::unexpected(e.to_string()))?,
            display_number: DisplayNumber::new(row.display_number)
                .map_err(|e| InfraError::unexpected(e.to_string()))?,
            title: row.title,
            form_data: row.form_data,
            status: row
                .status
                .parse::<WorkflowInstanceStatus>()
                .map_err(|e| InfraError::unexpected(e.to_string()))?,
            version: Version::new(row.version as u32)
                .map_err(|e| InfraError::unexpected(e.to_string()))?,
            current_step_id: row.current_step_id,
            initiated_by: UserId::from_uuid(row.initiated_by),
            submitted_at: row.submitted_at,
            completed_at: row.completed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .map_err(|e| InfraError::unexpected(e.to_string()))
    }
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
    #[tracing::instrument(skip_all, level = "debug")]
    async fn insert(
        &self,
        tx: &mut TxContext,
        instance: &WorkflowInstance,
    ) -> Result<(), InfraError> {
        let status: &str = instance.status().into();
        sqlx::query!(
            r#"
            INSERT INTO workflow_instances (
                id, tenant_id, definition_id, definition_version,
                display_number, title, form_data, status, version,
                current_step_id, initiated_by, submitted_at,
                completed_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
            instance.id().as_uuid(),
            instance.tenant_id().as_uuid(),
            instance.definition_id().as_uuid(),
            instance.definition_version().as_i32(),
            instance.display_number().as_i64(),
            instance.title(),
            instance.form_data(),
            status,
            instance.version().as_i32(),
            instance.current_step_id(),
            instance.initiated_by().as_uuid(),
            instance.submitted_at(),
            instance.completed_at(),
            instance.created_at(),
            instance.updated_at()
        )
        .execute(tx.conn())
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug")]
    async fn update_with_version_check(
        &self,
        tx: &mut TxContext,
        instance: &WorkflowInstance,
        expected_version: Version,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        let status: &str = instance.status().into();
        let result = sqlx::query!(
            r#"
            UPDATE workflow_instances SET
                title = $1,
                form_data = $2,
                status = $3,
                version = $4,
                current_step_id = $5,
                submitted_at = $6,
                completed_at = $7,
                updated_at = $8
            WHERE id = $9 AND version = $10 AND tenant_id = $11
            "#,
            instance.title(),
            instance.form_data(),
            status,
            instance.version().as_i32(),
            instance.current_step_id(),
            instance.submitted_at(),
            instance.completed_at(),
            instance.updated_at(),
            instance.id().as_uuid(),
            expected_version.as_i32(),
            tenant_id.as_uuid(),
        )
        .execute(tx.conn())
        .await?;

        if result.rows_affected() == 0 {
            return Err(InfraError::conflict(
                "WorkflowInstance",
                instance.id().as_uuid().to_string(),
            ));
        }

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%id, %tenant_id))]
    async fn find_by_id(
        &self,
        id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowInstance>, InfraError> {
        let row = sqlx::query_as!(
            WorkflowInstanceRow,
            r#"
            SELECT
                id, tenant_id, definition_id, definition_version,
                display_number, title, form_data, status, version,
                current_step_id, initiated_by, submitted_at,
                completed_at, created_at, updated_at
            FROM workflow_instances
            WHERE id = $1 AND tenant_id = $2
            "#,
            id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(WorkflowInstance::try_from).transpose()
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowInstance>, InfraError> {
        let rows = sqlx::query_as!(
            WorkflowInstanceRow,
            r#"
            SELECT
                id, tenant_id, definition_id, definition_version,
                display_number, title, form_data, status, version,
                current_step_id, initiated_by, submitted_at,
                completed_at, created_at, updated_at
            FROM workflow_instances
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(WorkflowInstance::try_from).collect()
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id, %user_id))]
    async fn find_by_initiated_by(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Vec<WorkflowInstance>, InfraError> {
        let rows = sqlx::query_as!(
            WorkflowInstanceRow,
            r#"
            SELECT
                id, tenant_id, definition_id, definition_version,
                display_number, title, form_data, status, version,
                current_step_id, initiated_by, submitted_at,
                completed_at, created_at, updated_at
            FROM workflow_instances
            WHERE tenant_id = $1 AND initiated_by = $2
            ORDER BY created_at DESC
            "#,
            tenant_id.as_uuid(),
            user_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(WorkflowInstance::try_from).collect()
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn find_by_ids(
        &self,
        ids: &[WorkflowInstanceId],
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowInstance>, InfraError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let uuid_ids: Vec<Uuid> = ids.iter().map(|id| *id.as_uuid()).collect();

        let rows = sqlx::query_as!(
            WorkflowInstanceRow,
            r#"
            SELECT
                id, tenant_id, definition_id, definition_version,
                display_number, title, form_data, status, version,
                current_step_id, initiated_by, submitted_at,
                completed_at, created_at, updated_at
            FROM workflow_instances
            WHERE id = ANY($1) AND tenant_id = $2
            ORDER BY created_at DESC
            "#,
            &uuid_ids,
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(WorkflowInstance::try_from).collect()
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%display_number, %tenant_id))]
    async fn find_by_display_number(
        &self,
        display_number: DisplayNumber,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowInstance>, InfraError> {
        let row = sqlx::query_as!(
            WorkflowInstanceRow,
            r#"
            SELECT
                id, tenant_id, definition_id, definition_version,
                display_number, title, form_data, status, version,
                current_step_id, initiated_by, submitted_at,
                completed_at, created_at, updated_at
            FROM workflow_instances
            WHERE display_number = $1 AND tenant_id = $2
            "#,
            display_number.as_i64(),
            tenant_id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(WorkflowInstance::try_from).transpose()
    }
}

// =============================================================================
// テスト用拡張 trait
// =============================================================================

/// テストセットアップ用の拡張メソッド
///
/// mock TxContext で書き込みメソッドを呼ぶヘルパー。
/// テストセットアップコードの冗長化を抑制する。
#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
pub trait WorkflowInstanceRepositoryTestExt {
    /// テスト用: mock TxContext で insert する
    async fn insert_for_test(&self, instance: &WorkflowInstance) -> Result<(), InfraError>;
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl<T: WorkflowInstanceRepository + ?Sized> WorkflowInstanceRepositoryTestExt for T {
    async fn insert_for_test(&self, instance: &WorkflowInstance) -> Result<(), InfraError> {
        let mut tx = TxContext::mock();
        self.insert(&mut tx, instance).await
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
