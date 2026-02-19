//! ワークフローの再申請

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayIdEntityType, DisplayNumber},
    workflow::{
        NewWorkflowStep,
        WorkflowInstanceId,
        WorkflowInstanceStatus,
        WorkflowStep,
        WorkflowStepId,
    },
};
use ringiflow_infra::InfraError;
use ringiflow_shared::{event_log::event, log_business_event};

use crate::{
    error::CoreError,
    usecase::{
        helpers::FindResultExt,
        workflow::{ResubmitWorkflowInput, WorkflowUseCaseImpl, WorkflowWithSteps},
    },
};

impl WorkflowUseCaseImpl {
    /// ワークフローを再申請する
    ///
    /// ## 処理フロー
    ///
    /// 1. ワークフローインスタンスを取得
    /// 2. ChangesRequested 状態であるか確認
    /// 3. 権限チェック（申請者本人のみ再申請可能）
    /// 4. 楽観的ロック（バージョン一致チェック）
    /// 5. ワークフロー定義を取得し、承認ステップを抽出
    /// 6. approvers との整合性を検証
    /// 7. 新しい承認ステップを作成
    /// 8. インスタンスを InProgress に遷移（form_data 更新）
    /// 9. 保存
    ///
    /// ## エラー
    ///
    /// - インスタンスが見つからない場合: 404
    /// - ChangesRequested 以外の場合: 400
    /// - 申請者以外の場合: 403
    /// - バージョン不一致の場合: 409
    /// - approvers と定義が不一致の場合: 400
    pub async fn resubmit_workflow(
        &self,
        input: ResubmitWorkflowInput,
        instance_id: WorkflowInstanceId,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowWithSteps, CoreError> {
        // 1. ワークフローインスタンスを取得
        let instance = self
            .instance_repo
            .find_by_id(&instance_id, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        // 2. ChangesRequested 状態であるか確認
        if instance.status() != WorkflowInstanceStatus::ChangesRequested {
            return Err(CoreError::BadRequest(
                "要修正状態のワークフローのみ再申請できます".to_string(),
            ));
        }

        // 3. 権限チェック（申請者本人のみ再申請可能）
        if instance.initiated_by() != &user_id {
            return Err(CoreError::Forbidden(
                "このワークフローを再申請する権限がありません".to_string(),
            ));
        }

        // 4. 楽観的ロック（バージョン一致チェック — 早期フェイル）
        if instance.version() != input.version {
            return Err(CoreError::Conflict(
                "インスタンスは既に更新されています。最新の情報を取得してください。".to_string(),
            ));
        }

        // 5. ワークフロー定義を取得
        let definition = self
            .definition_repo
            .find_by_id(instance.definition_id(), &tenant_id)
            .await
            .or_not_found("ワークフロー定義")?;

        // 定義から承認ステップを抽出
        let approval_step_defs = definition
            .extract_approval_steps()
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 6. approvers と定義のステップの整合性を検証
        if input.approvers.len() != approval_step_defs.len() {
            return Err(CoreError::BadRequest(format!(
                "承認者の数({})が定義のステップ数({})と一致しません",
                input.approvers.len(),
                approval_step_defs.len()
            )));
        }

        for (approver, step_def) in input.approvers.iter().zip(&approval_step_defs) {
            if approver.step_id != step_def.id {
                return Err(CoreError::BadRequest(format!(
                    "承認者のステップ ID({})が定義のステップ ID({})と一致しません",
                    approver.step_id, step_def.id
                )));
            }
        }

        // 7. 新しい承認ステップを作成
        let now = self.clock.now();
        let mut steps = Vec::with_capacity(approval_step_defs.len());

        for (i, (step_def, approver)) in approval_step_defs.iter().zip(&input.approvers).enumerate()
        {
            let display_number = self
                .counter_repo
                .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
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

        // 8. インスタンスを InProgress に遷移
        let instance_expected_version = instance.version();
        let first_step_id = approval_step_defs[0].id.clone();
        let resubmitted_instance = instance
            .resubmitted(input.form_data, first_step_id, now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 9. インスタンスとステップを保存
        self.instance_repo
            .update_with_version_check(&resubmitted_instance, instance_expected_version, &tenant_id)
            .await
            .map_err(|e| match e {
                InfraError::Conflict { .. } => CoreError::Conflict(
                    "インスタンスは既に更新されています。最新の情報を取得してください。"
                        .to_string(),
                ),
                other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
            })?;

        for step in &steps {
            self.step_repo
                .insert(step, &tenant_id)
                .await
                .map_err(|e| CoreError::Internal(format!("ステップの保存に失敗: {}", e)))?;
        }

        log_business_event!(
            event.category = event::category::WORKFLOW,
            event.action = event::action::WORKFLOW_RESUBMITTED,
            event.entity_type = event::entity_type::WORKFLOW_INSTANCE,
            event.entity_id = %instance_id,
            event.actor_id = %user_id,
            event.tenant_id = %tenant_id,
            event.result = event::result::SUCCESS,
            "ワークフロー再申請"
        );

        Ok(WorkflowWithSteps {
            instance: resubmitted_instance,
            steps,
        })
    }

    /// display_number でワークフローを再申請する
    pub async fn resubmit_workflow_by_display_number(
        &self,
        input: ResubmitWorkflowInput,
        display_number: DisplayNumber,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowWithSteps, CoreError> {
        // display_number → WorkflowInstanceId を解決
        let instance = self
            .instance_repo
            .find_by_display_number(display_number, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        // 既存の resubmit_workflow を呼び出し
        self.resubmit_workflow(input, instance.id().clone(), tenant_id, user_id)
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
            WorkflowDefinition,
            WorkflowDefinitionId,
            WorkflowInstance,
            WorkflowInstanceId,
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
        repository::WorkflowInstanceRepository,
    };

    use super::super::super::test_helpers::single_approval_definition_json;
    use crate::{
        error::CoreError,
        usecase::workflow::{ResubmitWorkflowInput, StepApprover, WorkflowUseCaseImpl},
    };

    #[tokio::test]
    async fn test_resubmit_workflow_正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        // 1段階承認の定義を追加
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

        // ChangesRequested 状態のインスタンスを作成
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: definition.id().clone(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({"note": "original"}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now)
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert(&instance).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo.clone()),
            Arc::new(step_repo.clone()),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = ResubmitWorkflowInput {
            form_data: serde_json::json!({"note": "updated"}),
            approvers: vec![StepApprover {
                step_id:     "approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
            version:   instance.version(),
        };

        // Act
        let result = sut
            .resubmit_workflow(
                input,
                instance.id().clone(),
                tenant_id.clone(),
                user_id.clone(),
            )
            .await;

        // Assert
        let result = result.unwrap();

        // ステータスが InProgress に戻っている
        assert_eq!(
            result.instance.status(),
            ringiflow_domain::workflow::WorkflowInstanceStatus::InProgress
        );

        // form_data が更新されている
        assert_eq!(
            result.instance.form_data(),
            &serde_json::json!({"note": "updated"})
        );

        // 新しいステップが作成されている
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].assigned_to(), Some(&approver_id));
        assert_eq!(
            result.steps[0].status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Active
        );
    }

    #[tokio::test]
    async fn test_resubmit_workflow_要修正以外は400() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        // InProgress 状態のインスタンス（ChangesRequested ではない）
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

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = ResubmitWorkflowInput {
            form_data: serde_json::json!({}),
            approvers: vec![StepApprover {
                step_id:     "approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
            version:   instance.version(),
        };

        // Act
        let result = sut
            .resubmit_workflow(input, instance.id().clone(), tenant_id, user_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_resubmit_workflow_バージョン不一致で409() {
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
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert(&instance).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let wrong_version = Version::initial(); // actual: initial.next().next() (submitted + request_changes)
        let input = ResubmitWorkflowInput {
            form_data: serde_json::json!({}),
            approvers: vec![StepApprover {
                step_id:     "approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
            version:   wrong_version,
        };

        // Act
        let result = sut
            .resubmit_workflow(input, instance.id().clone(), tenant_id, user_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_resubmit_workflow_approvers不一致でエラー() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        // 1段階承認の定義だが、2人指定する
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
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert(&instance).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        // 1段階定義に2人指定
        let input = ResubmitWorkflowInput {
            form_data: serde_json::json!({}),
            approvers: vec![
                StepApprover {
                    step_id:     "approval".to_string(),
                    assigned_to: approver_id.clone(),
                },
                StepApprover {
                    step_id:     "extra".to_string(),
                    assigned_to: UserId::new(),
                },
            ],
            version:   instance.version(),
        };

        // Act
        let result = sut
            .resubmit_workflow(input, instance.id().clone(), tenant_id, user_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_resubmit_workflow_申請者以外は403() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let other_user_id = UserId::new();
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
            initiated_by: user_id.clone(), // user_id が申請者
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now)
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert(&instance).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = ResubmitWorkflowInput {
            form_data: serde_json::json!({}),
            approvers: vec![StepApprover {
                step_id:     "approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
            version:   instance.version(),
        };

        // Act: 別のユーザーで再申請を試みる
        let result = sut
            .resubmit_workflow(
                input,
                instance.id().clone(),
                tenant_id,
                other_user_id, // 申請者ではない
            )
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::Forbidden(_))));
    }
}
