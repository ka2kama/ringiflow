//! ワークフローステップの差し戻し

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::DisplayNumber,
    workflow::{WorkflowStepId, WorkflowStepStatus},
};
use ringiflow_infra::InfraError;

use crate::{
    error::CoreError,
    usecase::{
        helpers::{FindResultExt, check_step_assigned_to},
        workflow::{ApproveRejectInput, WorkflowUseCaseImpl, WorkflowWithSteps},
    },
};

impl WorkflowUseCaseImpl {
    /// ワークフローステップを差し戻す
    ///
    /// ## 処理フロー
    ///
    /// 1. ステップを取得
    /// 2. 権限チェック（担当者のみ差し戻し可能）
    /// 3. 楽観的ロック（バージョン一致チェック）
    /// 4. ステップを差し戻し
    /// 5. 残りの Pending ステップを Skipped に遷移
    /// 6. インスタンスを ChangesRequested に遷移
    /// 7. 保存
    ///
    /// ## エラー
    ///
    /// - ステップが見つからない場合: 404
    /// - 権限がない場合: 403
    /// - Active 以外の場合: 400
    /// - バージョン不一致の場合: 409
    pub async fn request_changes_step(
        &self,
        input: ApproveRejectInput,
        step_id: WorkflowStepId,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowWithSteps, CoreError> {
        // 1. ステップを取得
        let step = self
            .step_repo
            .find_by_id(&step_id, &tenant_id)
            .await
            .or_not_found("ステップ")?;

        // 2. 権限チェック
        check_step_assigned_to(&step, &user_id, "差し戻し")?;

        // 3. 楽観的ロック（バージョン一致チェック — 早期フェイル）
        if step.version() != input.version {
            return Err(CoreError::Conflict(
                "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
            ));
        }

        // 4. ステップを差し戻し
        let now = self.clock.now();
        let step_expected_version = step.version();
        let request_changes_step = step
            .request_changes(input.comment, now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 5. 差し戻しステップを保存
        self.step_repo
            .update_with_version_check(&request_changes_step, step_expected_version, &tenant_id)
            .await
            .map_err(|e| match e {
                InfraError::Conflict { .. } => CoreError::Conflict(
                    "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
                ),
                other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
            })?;

        // 6. 残りの Pending ステップを Skipped に遷移
        let all_steps = self
            .step_repo
            .find_by_instance(request_changes_step.instance_id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

        for pending_step in all_steps
            .into_iter()
            .filter(|s| s.status() == WorkflowStepStatus::Pending)
        {
            let pending_expected_version = pending_step.version();
            let skipped_step = pending_step
                .skipped(now)
                .map_err(|e| CoreError::Internal(format!("ステップのスキップに失敗: {}", e)))?;
            self.step_repo
                .update_with_version_check(&skipped_step, pending_expected_version, &tenant_id)
                .await
                .map_err(|e| match e {
                    InfraError::Conflict { .. } => CoreError::Conflict(
                        "ステップは既に更新されています。最新の情報を取得してください。"
                            .to_string(),
                    ),
                    other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
                })?;
        }

        // 7. インスタンスを取得して ChangesRequested に遷移
        let instance = self
            .instance_repo
            .find_by_id(request_changes_step.instance_id(), &tenant_id)
            .await
            .or_not_found("インスタンス")?;

        let instance_expected_version = instance.version();
        let changes_requested_instance = instance
            .complete_with_request_changes(now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        self.instance_repo
            .update_with_version_check(&changes_requested_instance, instance_expected_version)
            .await
            .map_err(|e| match e {
                InfraError::Conflict { .. } => CoreError::Conflict(
                    "インスタンスは既に更新されています。最新の情報を取得してください。"
                        .to_string(),
                ),
                other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
            })?;

        // 8. 保存後のステップ一覧を取得して返却
        let steps = self
            .step_repo
            .find_by_instance(changes_requested_instance.id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

        Ok(WorkflowWithSteps {
            instance: changes_requested_instance,
            steps,
        })
    }

    /// display_number でワークフローステップを差し戻す
    pub async fn request_changes_step_by_display_number(
        &self,
        input: ApproveRejectInput,
        workflow_display_number: DisplayNumber,
        step_display_number: DisplayNumber,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowWithSteps, CoreError> {
        // display_number → WorkflowInstanceId を解決
        let instance = self
            .instance_repo
            .find_by_display_number(workflow_display_number, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        // display_number → WorkflowStepId を解決
        let step = self
            .step_repo
            .find_by_display_number(step_display_number, instance.id(), &tenant_id)
            .await
            .or_not_found("ステップ")?;

        // 既存の request_changes_step を呼び出し
        self.request_changes_step(input, step.id().clone(), tenant_id, user_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ringiflow_domain::{
        clock::FixedClock,
        tenant::TenantId,
        user::UserId,
        value_objects::{DisplayNumber, Version, WorkflowName},
        workflow::{
            NewWorkflowDefinition,
            NewWorkflowInstance,
            NewWorkflowStep,
            WorkflowDefinition,
            WorkflowDefinitionId,
            WorkflowInstance,
            WorkflowInstanceId,
            WorkflowStep,
            WorkflowStepId,
        },
    };
    use ringiflow_infra::{
        mock::{
            MockDisplayIdCounterRepository,
            MockUserRepository,
            MockWorkflowCommentRepository,
            MockWorkflowDefinitionRepository,
            MockWorkflowInstanceRepository,
            MockWorkflowStepRepository,
        },
        repository::{WorkflowInstanceRepository, WorkflowStepRepository},
    };

    use super::super::super::test_helpers::{
        setup_two_step_approval,
        single_approval_definition_json,
    };
    use crate::{
        error::CoreError,
        usecase::workflow::{ApproveRejectInput, WorkflowUseCaseImpl, WorkflowWithSteps},
    };

    #[tokio::test]
    async fn test_request_changes_step_正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let now = chrono::Utc::now();
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

        let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "approval".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now,
        })
        .activated(now);
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo.clone()),
            Arc::new(step_repo.clone()),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = ApproveRejectInput {
            version: step.version(),
            comment: Some("金額を修正してください".to_string()),
        };

        // Act
        let result = sut
            .request_changes_step(
                input,
                step.id().clone(),
                tenant_id.clone(),
                approver_id.clone(),
            )
            .await;

        // Assert
        let result = result.unwrap();
        let expected = WorkflowWithSteps {
            instance: instance.complete_with_request_changes(now).unwrap(),
            steps:    vec![
                step.request_changes(Some("金額を修正してください".to_string()), now)
                    .unwrap(),
            ],
        };
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_request_changes_step_未割り当てユーザーは403() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let other_user_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let definition = WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id: tenant_id.clone(),
            name: WorkflowName::new("汎用申請").unwrap(),
            description: Some("テスト用定義".to_string()),
            definition: single_approval_definition_json(),
            created_by: user_id.clone(),
            now,
        })
        .published(now)
        .unwrap();
        definition_repo.add_definition(definition.clone());

        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: definition.id().clone(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

        let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "approval".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now,
        })
        .activated(now);
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = ApproveRejectInput {
            version: step.version(),
            comment: Some("差し戻します".to_string()),
        };

        // Act: 別のユーザーで差し戻しを試みる
        let result = sut
            .request_changes_step(input, step.id().clone(), tenant_id, other_user_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_request_changes_step_active以外は400() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let definition = WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id: tenant_id.clone(),
            name: WorkflowName::new("汎用申請").unwrap(),
            description: Some("テスト用定義".to_string()),
            definition: single_approval_definition_json(),
            created_by: user_id.clone(),
            now,
        })
        .published(now)
        .unwrap();
        definition_repo.add_definition(definition.clone());

        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: definition.id().clone(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

        // Pending ステップを作成（Active ではない）
        let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "approval".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now,
        });
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = ApproveRejectInput {
            version: step.version(),
            comment: Some("差し戻します".to_string()),
        };

        // Act: Pending ステップに対して差し戻しを試みる
        let result = sut
            .request_changes_step(input, step.id().clone(), tenant_id, approver_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_request_changes_step_バージョン不一致で409() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let now = chrono::Utc::now();
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

        let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "approval".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now,
        })
        .activated(now);
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let wrong_version = Version::initial().next();
        let input = ApproveRejectInput {
            version: wrong_version,
            comment: None,
        };

        // Act
        let result = sut
            .request_changes_step(input, step.id().clone(), tenant_id, approver_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_request_changes_step_残りのpendingステップがskipped() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver1_id = UserId::new();
        let approver2_id = UserId::new();
        let now = chrono::Utc::now();

        let (definition, instance, step1, step2) =
            setup_two_step_approval(&tenant_id, &user_id, &approver1_id, &approver2_id, now);

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        definition_repo.add_definition(definition);
        instance_repo.insert(&instance).await.unwrap();
        step_repo.insert(&step1, &tenant_id).await.unwrap();
        step_repo.insert(&step2, &tenant_id).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo.clone()),
            Arc::new(step_repo.clone()),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = ApproveRejectInput {
            version: step1.version(),
            comment: Some("差し戻します".to_string()),
        };

        // Act: 最初のステップを差し戻し
        let result = sut
            .request_changes_step(
                input,
                step1.id().clone(),
                tenant_id.clone(),
                approver1_id.clone(),
            )
            .await;

        // Assert
        let result = result.unwrap();

        // インスタンスのステータスが ChangesRequested になっている
        assert_eq!(
            result.instance.status(),
            ringiflow_domain::workflow::WorkflowInstanceStatus::ChangesRequested
        );

        // ステップ一覧の確認
        assert_eq!(result.steps.len(), 2);

        // ステップ1は Completed になっている
        let result_step1 = result
            .steps
            .iter()
            .find(|s| s.step_id() == "manager_approval")
            .unwrap();
        assert_eq!(
            result_step1.status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Completed
        );

        // ステップ2は Skipped になっている
        let result_step2 = result
            .steps
            .iter()
            .find(|s| s.step_id() == "finance_approval")
            .unwrap();
        assert_eq!(
            result_step2.status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Skipped
        );
    }
}
