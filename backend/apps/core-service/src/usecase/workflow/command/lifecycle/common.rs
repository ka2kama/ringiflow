//! submit / resubmit の共通ヘルパー
//!
//! approvers 検証とステップ作成ループは submit / resubmit で同一のため共通化する。

use chrono::{DateTime, Utc};
use ringiflow_domain::{
    tenant::TenantId,
    value_objects::DisplayIdEntityType,
    workflow::{
        ApprovalStepDef,
        NewWorkflowStep,
        WorkflowInstanceId,
        WorkflowStep,
        WorkflowStepId,
    },
};

use crate::{
    error::CoreError,
    usecase::workflow::{StepApprover, WorkflowUseCaseImpl},
};

/// approvers と定義のステップの整合性を検証する
///
/// - 数の一致チェック
/// - 各 step_id の一致チェック
pub(super) fn validate_approvers(
    approvers: &[StepApprover],
    approval_step_defs: &[ApprovalStepDef],
) -> Result<(), CoreError> {
    if approvers.len() != approval_step_defs.len() {
        return Err(CoreError::BadRequest(format!(
            "承認者の数({})が定義のステップ数({})と一致しません",
            approvers.len(),
            approval_step_defs.len()
        )));
    }

    for (approver, step_def) in approvers.iter().zip(approval_step_defs) {
        if approver.step_id != step_def.id {
            return Err(CoreError::BadRequest(format!(
                "承認者のステップ ID({})が定義のステップ ID({})と一致しません",
                approver.step_id, step_def.id
            )));
        }
    }

    Ok(())
}

impl WorkflowUseCaseImpl {
    /// 定義と approvers に基づいて承認ステップを作成する
    ///
    /// 最初のステップのみ Active、残りは Pending。
    pub(super) async fn create_approval_steps(
        &self,
        instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
        approval_step_defs: &[ApprovalStepDef],
        approvers: &[StepApprover],
        now: DateTime<Utc>,
    ) -> Result<Vec<WorkflowStep>, CoreError> {
        let mut steps = Vec::with_capacity(approval_step_defs.len());

        for (i, (step_def, approver)) in approval_step_defs.iter().zip(approvers).enumerate() {
            let display_number = self
                .counter_repo
                .next_display_number(tenant_id, DisplayIdEntityType::WorkflowStep)
                .await
                .map_err(|e| CoreError::Internal(format!("採番に失敗: {}", e)))?;

            let step = WorkflowStep::new(NewWorkflowStep {
                id: WorkflowStepId::new(),
                instance_id: instance_id.clone(),
                display_number,
                step_id: step_def.id.clone(),
                step_name: step_def.name.clone(),
                step_type: "approval".to_string(),
                assigned_to: Some(approver.assigned_to.clone()),
                now,
            });

            // 最初のステップのみ Active にする
            let step = if i == 0 { step.activated(now) } else { step };
            steps.push(step);
        }

        Ok(steps)
    }
}
