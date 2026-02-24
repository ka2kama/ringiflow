//! reject / request_changes の共通フロー
//!
//! 両操作は 95% 同一のため、変動点（ドメインメソッド、インスタンス遷移、
//! イベント）を `StepTerminationType` enum で切り替える。

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    workflow::{WorkflowStep, WorkflowStepId, WorkflowStepStatus},
};
use ringiflow_shared::{event_log::event, log_business_event};

use crate::{
    error::CoreError,
    usecase::{
        helpers::{FindResultExt, check_step_assigned_to},
        workflow::{ApproveRejectInput, WorkflowUseCaseImpl, WorkflowWithSteps},
    },
};

/// ステップ終了操作の種別
pub(super) enum StepTerminationType {
    /// 却下
    Reject,
    /// 差し戻し
    RequestChanges,
}

impl StepTerminationType {
    /// 権限チェック用のアクション名
    fn action_name(&self) -> &'static str {
        match self {
            Self::Reject => "却下",
            Self::RequestChanges => "差し戻し",
        }
    }
}

impl WorkflowUseCaseImpl {
    /// reject / request_changes の共通フロー
    ///
    /// ## 処理フロー
    ///
    /// 1. ステップを取得
    /// 2. 権限チェック（担当者のみ操作可能）
    /// 3. 楽観的ロック（バージョン一致チェック）
    /// 4. ステップにドメイン操作を適用（種別で分岐）
    /// 5. 残りの Pending ステップを Skipped に遷移
    /// 6. インスタンスを終了状態に遷移（種別で分岐）
    /// 7. トランザクション保存
    /// 8. イベントログ（種別で分岐）
    pub(super) async fn terminate_step(
        &self,
        input: ApproveRejectInput,
        step_id: WorkflowStepId,
        tenant_id: TenantId,
        user_id: UserId,
        termination: StepTerminationType,
    ) -> Result<WorkflowWithSteps, CoreError> {
        // 1. ステップを取得
        let step = self
            .step_repo
            .find_by_id(&step_id, &tenant_id)
            .await
            .or_not_found("ステップ")?;

        // 2. 権限チェック
        check_step_assigned_to(&step, &user_id, termination.action_name())?;

        // 3. 楽観的ロック（バージョン一致チェック — 早期フェイル）
        if step.version() != input.version {
            return Err(CoreError::Conflict(
                "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
            ));
        }

        // 4. ステップにドメイン操作を適用（種別で分岐）
        let now = self.clock.now();
        let step_expected_version = step.version();
        let terminated_step = Self::apply_step_termination(step, &termination, &input, now)?;

        // 5. 残りの Pending ステップを Skipped に遷移（トランザクション開始前にドメインロジック実行）
        let all_steps = self
            .fetch_instance_steps(terminated_step.instance_id(), &tenant_id)
            .await?;

        let skipped_steps = Self::skip_pending_steps(all_steps, now)?;

        // 6. インスタンスを取得して終了状態に遷移（トランザクション開始前にドメインロジック実行）
        let instance = self
            .instance_repo
            .find_by_id(terminated_step.instance_id(), &tenant_id)
            .await
            .or_not_found("インスタンス")?;

        let instance_expected_version = instance.version();
        let completed_instance = match termination {
            StepTerminationType::Reject => instance
                .complete_with_rejection(now)
                .map_err(|e| CoreError::BadRequest(e.to_string()))?,
            StepTerminationType::RequestChanges => instance
                .complete_with_request_changes(now)
                .map_err(|e| CoreError::BadRequest(e.to_string()))?,
        };

        // 7. 全更新を単一トランザクションで実行
        let mut tx = self.begin_tx().await?;

        self.save_step(&mut tx, &terminated_step, step_expected_version, &tenant_id)
            .await?;

        for (skipped_step, pending_expected_version) in &skipped_steps {
            self.save_step(&mut tx, skipped_step, *pending_expected_version, &tenant_id)
                .await?;
        }

        self.save_instance(
            &mut tx,
            &completed_instance,
            instance_expected_version,
            &tenant_id,
        )
        .await?;

        self.commit_tx(tx).await?;

        // 8. 保存後のステップ一覧を取得して返却
        let steps = self
            .fetch_instance_steps(completed_instance.id(), &tenant_id)
            .await?;

        Self::log_termination_event(&termination, &step_id, &user_id, &tenant_id);

        Ok(WorkflowWithSteps {
            instance: completed_instance,
            steps,
        })
    }

    /// ステップにドメイン操作を適用する
    fn apply_step_termination(
        step: WorkflowStep,
        termination: &StepTerminationType,
        input: &ApproveRejectInput,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<WorkflowStep, CoreError> {
        let comment = input.comment.clone();
        match termination {
            StepTerminationType::Reject => step
                .reject(comment, now)
                .map_err(|e| CoreError::BadRequest(e.to_string())),
            StepTerminationType::RequestChanges => step
                .request_changes(comment, now)
                .map_err(|e| CoreError::BadRequest(e.to_string())),
        }
    }

    /// Pending ステップを Skipped に遷移する
    fn skip_pending_steps(
        all_steps: Vec<WorkflowStep>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<(WorkflowStep, ringiflow_domain::value_objects::Version)>, CoreError> {
        let mut skipped_steps = Vec::new();
        for pending_step in all_steps
            .into_iter()
            .filter(|s| s.status() == WorkflowStepStatus::Pending)
        {
            let version = pending_step.version();
            let skipped = pending_step
                .skipped(now)
                .map_err(|e| CoreError::Internal(format!("ステップのスキップに失敗: {}", e)))?;
            skipped_steps.push((skipped, version));
        }
        Ok(skipped_steps)
    }

    /// イベントログを記録する
    fn log_termination_event(
        termination: &StepTerminationType,
        step_id: &WorkflowStepId,
        user_id: &UserId,
        tenant_id: &TenantId,
    ) {
        match termination {
            StepTerminationType::Reject => {
                log_business_event!(
                    event.category = event::category::WORKFLOW,
                    event.action = event::action::STEP_REJECTED,
                    event.entity_type = event::entity_type::WORKFLOW_STEP,
                    event.entity_id = %step_id,
                    event.actor_id = %user_id,
                    event.tenant_id = %tenant_id,
                    event.result = event::result::SUCCESS,
                    "却下ステップ完了"
                );
            }
            StepTerminationType::RequestChanges => {
                log_business_event!(
                    event.category = event::category::WORKFLOW,
                    event.action = event::action::STEP_CHANGES_REQUESTED,
                    event.entity_type = event::entity_type::WORKFLOW_STEP,
                    event.entity_id = %step_id,
                    event.actor_id = %user_id,
                    event.tenant_id = %tenant_id,
                    event.result = event::result::SUCCESS,
                    "差し戻しステップ完了"
                );
            }
        }
    }
}
