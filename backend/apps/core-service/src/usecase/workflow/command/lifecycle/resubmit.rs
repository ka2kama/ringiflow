//! ワークフローの再申請

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::DisplayNumber,
    workflow::{WorkflowInstanceId, WorkflowInstanceStatus},
};
use ringiflow_shared::{event_log::event, log_business_event};

use super::common::validate_approvers;
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
            .deps
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
            .deps
            .definition_repo
            .find_by_id(instance.definition_id(), &tenant_id)
            .await
            .or_not_found("ワークフロー定義")?;

        // 定義から承認ステップを抽出
        let approval_step_defs = definition
            .extract_approval_steps()
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 6. approvers と定義のステップの整合性を検証
        validate_approvers(&input.approvers, &approval_step_defs)?;

        // 7. 新しい承認ステップを作成
        let now = self.deps.clock.now();
        let steps = self
            .create_approval_steps(
                &instance_id,
                &tenant_id,
                &approval_step_defs,
                &input.approvers,
                now,
            )
            .await?;

        // 8. インスタンスを InProgress に遷移
        let instance_expected_version = instance.version();
        let first_step_id = approval_step_defs[0].id.clone();
        let resubmitted_instance = instance
            .resubmitted(input.form_data, first_step_id, now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 9. インスタンスとステップを保存（単一トランザクション）
        let mut tx = self.begin_tx().await?;
        self.save_instance(
            &mut tx,
            &resubmitted_instance,
            instance_expected_version,
            &tenant_id,
        )
        .await?;
        for step in &steps {
            self.deps
                .step_repo
                .insert(&mut tx, step, &tenant_id)
                .await
                .map_err(|e| CoreError::Internal(format!("ステップの保存に失敗: {}", e)))?;
        }
        self.commit_tx(tx).await?;

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

        // 承認依頼通知を送信（fire-and-forget）
        self.send_approval_request_notification(&resubmitted_instance, &steps, &tenant_id)
            .await;

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
            .deps
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
        tenant::TenantId,
        user::{Email, User, UserId},
        value_objects::{DisplayNumber, UserName, Version, WorkflowName},
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
        fake::{
            FakeUserRepository,
            FakeWorkflowDefinitionRepository,
            FakeWorkflowInstanceRepository,
            FakeWorkflowStepRepository,
        },
        repository::WorkflowInstanceRepositoryTestExt,
    };

    use super::super::super::test_helpers::{
        build_sut,
        build_sut_with_notification,
        single_approval_definition_json,
    };
    use crate::{
        error::CoreError,
        usecase::workflow::{ResubmitWorkflowInput, StepApprover},
    };

    #[tokio::test]
    async fn test_resubmit_workflow_正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
        .unwrap()
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

    // ===== 通知テスト =====

    #[tokio::test]
    async fn test_resubmit_workflow_正常系で承認依頼通知が送信される() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

        let definition = WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id: tenant_id.clone(),
            name: WorkflowName::new("経費精算申請").unwrap(),
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
        .unwrap()
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

        // ユーザー情報を Fake に登録
        let user_repo = FakeUserRepository::new();
        user_repo.add_user(User::new(
            user_id.clone(),
            tenant_id.clone(),
            DisplayNumber::new(1).unwrap(),
            Email::new("tanaka@example.com").unwrap(),
            UserName::new("田中太郎").unwrap(),
            now,
        ));
        user_repo.add_user(User::new(
            approver_id.clone(),
            tenant_id.clone(),
            DisplayNumber::new(2).unwrap(),
            Email::new("suzuki@example.com").unwrap(),
            UserName::new("鈴木一郎").unwrap(),
            now,
        ));

        let (sut, sender) = build_sut_with_notification(
            &definition_repo,
            &instance_repo,
            &step_repo,
            Arc::new(user_repo),
            now,
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

        // Assert: ワークフロー操作は成功
        assert!(result.is_ok());

        // Assert: 承認依頼通知が送信されている
        let sent = sender.sent_emails();
        assert_eq!(sent.len(), 1, "承認依頼メールが1通送信されるべき");
        assert_eq!(sent[0].to, "suzuki@example.com");
        assert!(
            sent[0].subject.contains("テスト申請"),
            "件名にワークフロータイトルが含まれるべき: {}",
            sent[0].subject
        );
    }

    #[tokio::test]
    async fn test_resubmit_workflow_要修正以外は400() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
        .with_current_step("approval".to_string(), now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
        .unwrap()
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
        .unwrap()
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
        .unwrap()
        .complete_with_request_changes(now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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
