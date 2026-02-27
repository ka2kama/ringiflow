//! submit / resubmit の共通ヘルパー
//!
//! approvers 検証とステップ作成ループは submit / resubmit で同一のため共通化する。

use chrono::{DateTime, Utc};
use ringiflow_domain::{
    notification::WorkflowNotification,
    tenant::TenantId,
    value_objects::{DisplayId, DisplayIdEntityType, display_prefix},
    workflow::{
        ApprovalStepDef,
        NewWorkflowStep,
        WorkflowInstance,
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
                .deps
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

    /// 承認依頼通知を送信する（fire-and-forget）
    ///
    /// 最初の Active ステップの承認者に対して通知メールを送信する。
    /// ユーザー情報の取得失敗や通知送信の失敗はログ出力のみで、
    /// ワークフロー操作の結果には影響しない。
    pub(super) async fn send_approval_request_notification(
        &self,
        instance: &WorkflowInstance,
        steps: &[WorkflowStep],
        tenant_id: &TenantId,
    ) {
        // Active ステップ（最初の承認ステップ）を取得
        let Some(active_step) = steps
            .iter()
            .find(|s| s.status() == ringiflow_domain::workflow::WorkflowStepStatus::Active)
        else {
            return;
        };

        // 承認者のユーザー ID を取得
        let Some(approver_id) = active_step.assigned_to() else {
            return;
        };

        // 申請者の情報を取得
        let applicant = match self
            .deps
            .user_repo
            .find_by_id(instance.initiated_by())
            .await
        {
            Ok(Some(user)) => user,
            Ok(None) => {
                tracing::warn!(
                    user_id = %instance.initiated_by(),
                    "通知用の申請者情報が見つかりません"
                );
                return;
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    user_id = %instance.initiated_by(),
                    "通知用の申請者情報の取得に失敗"
                );
                return;
            }
        };

        // 承認者の情報を取得
        let approver = match self.deps.user_repo.find_by_id(approver_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                tracing::warn!(
                    user_id = %approver_id,
                    "通知用の承認者情報が見つかりません"
                );
                return;
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    user_id = %approver_id,
                    "通知用の承認者情報の取得に失敗"
                );
                return;
            }
        };

        let workflow_display_id =
            DisplayId::new(display_prefix::WORKFLOW_INSTANCE, instance.display_number())
                .to_string();

        let notification = WorkflowNotification::ApprovalRequest {
            workflow_title: instance.title().to_string(),
            workflow_display_id,
            applicant_name: applicant.name().as_str().to_string(),
            step_name: active_step.step_name().to_string(),
            approver_email: approver.email().as_str().to_string(),
            approver_user_id: approver_id.clone(),
        };

        self.deps
            .notification_service
            .notify(notification, tenant_id, instance.id())
            .await;
    }
}
