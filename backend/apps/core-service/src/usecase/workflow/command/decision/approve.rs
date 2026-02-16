//! ワークフローステップの承認

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::DisplayNumber,
    workflow::WorkflowStepId,
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
    pub async fn approve_step(
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
        check_step_assigned_to(&step, &user_id, "承認")?;

        // 3. 楽観的ロック（バージョン一致チェック — 早期フェイル）
        if step.version() != input.version {
            return Err(CoreError::Conflict(
                "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
            ));
        }

        // 4. ステップを承認
        let now = self.clock.now();
        let step_expected_version = step.version();
        let current_step_id = step.step_id().to_string();
        let approved_step = step
            .approve(input.comment, now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 5. インスタンスを取得
        let instance = self
            .instance_repo
            .find_by_id(approved_step.instance_id(), &tenant_id)
            .await
            .or_not_found("インスタンス")?;

        let instance_expected_version = instance.version();

        // 6. 定義から承認ステップの順序を取得し、次ステップを判定
        let definition = self
            .definition_repo
            .find_by_id(instance.definition_id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
            .ok_or_else(|| CoreError::Internal("定義が見つかりません".to_string()))?;

        let approval_step_defs =
            ringiflow_domain::workflow::extract_approval_steps(definition.definition())
                .map_err(|e| CoreError::Internal(format!("定義の解析に失敗: {}", e)))?;

        // 現在のステップの位置を特定し、次のステップがあるか判定
        let current_index = approval_step_defs
            .iter()
            .position(|s| s.id == current_step_id);

        let next_step_def = current_index.and_then(|i| approval_step_defs.get(i + 1));

        // 7. 次ステップの有無でインスタンスの遷移を分岐
        let (updated_instance, next_step_to_activate) = if let Some(next_def) = next_step_def {
            // 次ステップあり → current_step_id を更新、InProgress のまま
            let advanced = instance
                .advance_to_next_step(next_def.id.clone(), now)
                .map_err(|e| CoreError::BadRequest(e.to_string()))?;
            (advanced, Some(next_def.id.clone()))
        } else {
            // 最終ステップ → インスタンスを Approved に遷移
            let completed = instance
                .complete_with_approval(now)
                .map_err(|e| CoreError::BadRequest(e.to_string()))?;
            (completed, None)
        };

        // 8. 楽観的ロック付きでステップを保存
        self.step_repo
            .update_with_version_check(&approved_step, step_expected_version, &tenant_id)
            .await
            .map_err(|e| match e {
                InfraError::Conflict { .. } => CoreError::Conflict(
                    "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
                ),
                other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
            })?;

        // 9. 次ステップがあれば Active 化して保存
        if let Some(next_step_id) = next_step_to_activate {
            // インスタンスに紐づくステップから次ステップを見つけて Active 化
            let all_steps = self
                .step_repo
                .find_by_instance(updated_instance.id(), &tenant_id)
                .await
                .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

            if let Some(next_step) = all_steps.into_iter().find(|s| s.step_id() == next_step_id) {
                let next_expected_version = next_step.version();
                let activated_step = next_step.activated(now);
                self.step_repo
                    .update_with_version_check(&activated_step, next_expected_version, &tenant_id)
                    .await
                    .map_err(|e| match e {
                        InfraError::Conflict { .. } => CoreError::Conflict(
                            "ステップは既に更新されています。最新の情報を取得してください。"
                                .to_string(),
                        ),
                        other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
                    })?;
            }
        }

        // 10. インスタンスを保存
        self.instance_repo
            .update_with_version_check(&updated_instance, instance_expected_version)
            .await
            .map_err(|e| match e {
                InfraError::Conflict { .. } => CoreError::Conflict(
                    "インスタンスは既に更新されています。最新の情報を取得してください。"
                        .to_string(),
                ),
                other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
            })?;

        // 11. 保存後のステップ一覧を取得して返却
        let steps = self
            .step_repo
            .find_by_instance(updated_instance.id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

        Ok(WorkflowWithSteps {
            instance: updated_instance,
            steps,
        })
    }

    /// display_number でワークフローステップを承認する
    ///
    /// BFF が公開 API で display_number を使う場合に、
    /// 1回の呼び出しでステップ承認を完了する。
    pub async fn approve_step_by_display_number(
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

        // 既存の approve_step を呼び出し
        self.approve_step(input, step.id().clone(), tenant_id, user_id)
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
    async fn test_approve_step_正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        // 1段階承認の定義を追加
        let now = chrono::Utc::now();
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

        // InProgress のインスタンスを作成
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

        // Active なステップを作成
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
            comment: Some("承認しました".to_string()),
        };

        // Act
        let result = sut
            .approve_step(
                input,
                step.id().clone(),
                tenant_id.clone(),
                approver_id.clone(),
            )
            .await;

        // Assert
        let result = result.unwrap();
        let expected = WorkflowWithSteps {
            instance: instance.complete_with_approval(now).unwrap(),
            steps:    vec![step.approve(Some("承認しました".to_string()), now).unwrap()],
        };
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_approve_step_未割り当てユーザーは403() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let other_user_id = UserId::new(); // 別のユーザー

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
            assigned_to: Some(approver_id.clone()), // approver_id に割り当て
            now: chrono::Utc::now(),
        })
        .activated(chrono::Utc::now());
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
            comment: None,
        };

        // Act: 別のユーザーで承認を試みる
        let result = sut
            .approve_step(input, step.id().clone(), tenant_id, other_user_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_approve_step_active以外は400() {
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

        // Pending 状態のステップ（Active ではない）
        let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "approval".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now: chrono::Utc::now(),
        });
        // activated() を呼ばないので Pending のまま
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
            comment: None,
        };

        // Act
        let result = sut
            .approve_step(input, step.id().clone(), tenant_id, approver_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_approve_step_バージョン不一致で409() {
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
            now: chrono::Utc::now(),
        })
        .activated(chrono::Utc::now());
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

        // 不一致バージョンを指定（ステップの version は 1 だが、2 を指定）
        let wrong_version = Version::initial().next();
        let input = ApproveRejectInput {
            version: wrong_version,
            comment: None,
        };

        // Act
        let result = sut
            .approve_step(input, step.id().clone(), tenant_id, approver_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_approve_step_中間ステップ_次のステップがactiveになる() {
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
            comment: Some("上長承認OK".to_string()),
        };

        // Act
        let result = sut
            .approve_step(
                input,
                step1.id().clone(),
                tenant_id.clone(),
                approver1_id.clone(),
            )
            .await;

        // Assert
        let result = result.unwrap();

        // インスタンスのステータスは InProgress のまま
        assert_eq!(
            result.instance.status(),
            ringiflow_domain::workflow::WorkflowInstanceStatus::InProgress
        );

        // current_step_id が次のステップ（finance_approval）に更新されている
        assert_eq!(result.instance.current_step_id(), Some("finance_approval"));

        // ステップ一覧の確認
        assert_eq!(result.steps.len(), 2);

        // ステップ1は承認済み
        let result_step1 = result
            .steps
            .iter()
            .find(|s| s.step_id() == "manager_approval")
            .unwrap();
        assert_eq!(
            result_step1.status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Completed
        );

        // ステップ2は Active になっている
        let result_step2 = result
            .steps
            .iter()
            .find(|s| s.step_id() == "finance_approval")
            .unwrap();
        assert_eq!(
            result_step2.status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Active
        );
    }

    #[tokio::test]
    async fn test_approve_step_最終ステップ_インスタンスがapprovedになる() {
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

        // ステップ1は既に承認済み、current_step_id は finance_approval に移行済み
        let instance_at_step2 = instance
            .advance_to_next_step("finance_approval".to_string(), now)
            .unwrap();
        instance_repo.insert(&instance_at_step2).await.unwrap();

        let completed_step1 = step1.approve(Some("上長承認OK".to_string()), now).unwrap();
        let active_step2 = step2.activated(now);
        step_repo
            .insert(&completed_step1, &tenant_id)
            .await
            .unwrap();
        step_repo.insert(&active_step2, &tenant_id).await.unwrap();

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
            version: active_step2.version(),
            comment: Some("経理承認OK".to_string()),
        };

        // Act
        let result = sut
            .approve_step(
                input,
                active_step2.id().clone(),
                tenant_id.clone(),
                approver2_id.clone(),
            )
            .await;

        // Assert
        let result = result.unwrap();

        // インスタンスが Approved になっている
        assert_eq!(
            result.instance.status(),
            ringiflow_domain::workflow::WorkflowInstanceStatus::Approved
        );

        // ステップ2も承認済み
        let result_step2 = result
            .steps
            .iter()
            .find(|s| s.step_id() == "finance_approval")
            .unwrap();
        assert_eq!(
            result_step2.status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Completed
        );
    }
}
