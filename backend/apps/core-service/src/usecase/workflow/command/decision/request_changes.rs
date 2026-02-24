//! ワークフローステップの差し戻し

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::DisplayNumber,
    workflow::WorkflowStepId,
};

use super::common::StepTerminationType;
use crate::{
    error::CoreError,
    usecase::{
        helpers::FindResultExt,
        workflow::{ApproveRejectInput, WorkflowUseCaseImpl, WorkflowWithSteps},
    },
};

impl WorkflowUseCaseImpl {
    /// ワークフローステップを差し戻す
    ///
    /// 共通フロー `terminate_step` に `RequestChanges` 種別で委譲する。
    /// 詳細な処理フローは `common::terminate_step` を参照。
    pub async fn request_changes_step(
        &self,
        input: ApproveRejectInput,
        step_id: WorkflowStepId,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowWithSteps, CoreError> {
        self.terminate_step(
            input,
            step_id,
            tenant_id,
            user_id,
            StepTerminationType::RequestChanges,
        )
        .await
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
    use ringiflow_domain::{
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
            MockWorkflowDefinitionRepository,
            MockWorkflowInstanceRepository,
            MockWorkflowStepRepository,
        },
        repository::{WorkflowInstanceRepositoryTestExt, WorkflowStepRepositoryTestExt},
    };

    use super::super::super::test_helpers::{
        build_sut,
        setup_two_step_approval,
        single_approval_definition_json,
    };
    use crate::{
        error::CoreError,
        usecase::workflow::{ApproveRejectInput, WorkflowWithSteps},
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
        .with_current_step("approval".to_string(), now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

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
        step_repo.insert_for_test(&step, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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
        .with_current_step("approval".to_string(), now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

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
        step_repo.insert_for_test(&step, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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
        .with_current_step("approval".to_string(), now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

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
        step_repo.insert_for_test(&step, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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
        .with_current_step("approval".to_string(), now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

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
        step_repo.insert_for_test(&step, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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
        instance_repo.insert_for_test(&instance).await.unwrap();
        step_repo.insert_for_test(&step1, &tenant_id).await.unwrap();
        step_repo.insert_for_test(&step2, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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
