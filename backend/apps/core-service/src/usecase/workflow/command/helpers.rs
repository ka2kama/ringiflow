//! ワークフローコマンド共通のヘルパー関数
//!
//! 永続化ボイラープレート（トランザクション操作、version check 付き更新、
//! ステップ一覧取得）を共通化する。

use ringiflow_domain::{
    tenant::TenantId,
    value_objects::Version,
    workflow::{WorkflowInstance, WorkflowInstanceId, WorkflowStep},
};
use ringiflow_infra::{InfraErrorKind, TxContext};

use super::super::WorkflowUseCaseImpl;
use crate::error::CoreError;

impl WorkflowUseCaseImpl {
    /// トランザクションを開始する
    pub(super) async fn begin_tx(&self) -> Result<TxContext, CoreError> {
        self.deps
            .tx_manager
            .begin()
            .await
            .map_err(|e| CoreError::Internal(format!("トランザクション開始に失敗: {}", e)))
    }

    /// トランザクションをコミットする
    pub(super) async fn commit_tx(&self, tx: TxContext) -> Result<(), CoreError> {
        tx.commit()
            .await
            .map_err(|e| CoreError::Internal(format!("トランザクションコミットに失敗: {}", e)))
    }

    /// ステップを version check 付きで更新する
    pub(super) async fn save_step(
        &self,
        tx: &mut TxContext,
        step: &WorkflowStep,
        expected_version: Version,
        tenant_id: &TenantId,
    ) -> Result<(), CoreError> {
        self.deps
            .step_repo
            .update_with_version_check(tx, step, expected_version, tenant_id)
            .await
            .map_err(|e| match e.kind() {
                InfraErrorKind::Conflict { .. } => CoreError::Conflict(
                    "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
                ),
                _ => CoreError::Internal(format!("ステップの保存に失敗: {}", e)),
            })
    }

    /// インスタンスを version check 付きで更新する
    pub(super) async fn save_instance(
        &self,
        tx: &mut TxContext,
        instance: &WorkflowInstance,
        expected_version: Version,
        tenant_id: &TenantId,
    ) -> Result<(), CoreError> {
        self.deps
            .instance_repo
            .update_with_version_check(tx, instance, expected_version, tenant_id)
            .await
            .map_err(|e| match e.kind() {
                InfraErrorKind::Conflict { .. } => CoreError::Conflict(
                    "インスタンスは既に更新されています。最新の情報を取得してください。"
                        .to_string(),
                ),
                _ => CoreError::Internal(format!("インスタンスの保存に失敗: {}", e)),
            })
    }

    /// インスタンスに紐づくステップ一覧を取得する
    pub(super) async fn fetch_instance_steps(
        &self,
        instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowStep>, CoreError> {
        self.deps
            .step_repo
            .find_by_instance(instance_id, tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))
    }
}
