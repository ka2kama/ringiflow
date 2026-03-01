//! ワークフローの申請

use ringiflow_domain::{
    tenant::TenantId,
    value_objects::DisplayNumber,
    workflow::{WorkflowInstance, WorkflowInstanceId, WorkflowInstanceStatus},
};
use ringiflow_shared::{event_log::event, log_business_event};

use super::common::validate_approvers;
use crate::{
    error::CoreError,
    usecase::{
        helpers::FindResultExt,
        workflow::{SubmitWorkflowInput, WorkflowUseCaseImpl},
    },
};

impl WorkflowUseCaseImpl {
    /// ワークフローを申請する
    ///
    /// 下書き状態のワークフローを申請状態に遷移させ、
    /// ワークフロー定義に基づいて複数の承認ステップを作成する。
    ///
    /// ## 処理フロー
    ///
    /// 1. ワークフローインスタンスが存在するか確認
    /// 2. draft 状態であるか確認
    /// 3. ワークフロー定義を取得
    /// 4. 定義から承認ステップを抽出し、approvers との整合性を検証
    /// 5. 各承認ステップを作成（最初を Active、残りを Pending）
    /// 6. ワークフローインスタンスを pending → in_progress に遷移
    /// 7. インスタンスとステップをリポジトリに保存
    ///
    /// ## エラー
    ///
    /// - ワークフローインスタンスが見つからない場合
    /// - ワークフローインスタンスが draft でない場合
    /// - approvers と定義のステップが一致しない場合
    /// - データベースエラー
    pub async fn submit_workflow(
        &self,
        input: SubmitWorkflowInput,
        instance_id: WorkflowInstanceId,
        tenant_id: TenantId,
    ) -> Result<WorkflowInstance, CoreError> {
        // 1. ワークフローインスタンスを取得
        let instance = self
            .deps
            .instance_repo
            .find_by_id(&instance_id, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        // 2. draft 状態であるか確認
        if instance.status() != WorkflowInstanceStatus::Draft {
            return Err(CoreError::BadRequest(
                "下書き状態のワークフローのみ申請できます".to_string(),
            ));
        }

        // 3. ワークフロー定義を取得
        let definition = self
            .deps
            .definition_repo
            .find_by_id(instance.definition_id(), &tenant_id)
            .await
            .or_not_found("ワークフロー定義")?;

        // 4. 定義から承認ステップを抽出
        let approval_step_defs = definition
            .extract_approval_steps()
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // approvers と定義のステップの整合性を検証
        validate_approvers(&input.approvers, &approval_step_defs)?;

        // 5. 各承認ステップを作成
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

        // 6. ワークフローインスタンスを申請済みに遷移
        let expected_version = instance.version();
        let first_step_id = approval_step_defs[0].id.clone();
        let submitted_instance = instance
            .submitted(now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // current_step_id を最初の承認ステップに設定して in_progress に遷移
        let in_progress_instance = submitted_instance
            .with_current_step(first_step_id, now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 7. インスタンスとステップを保存（単一トランザクション）
        let mut tx = self.begin_tx().await?;
        self.save_instance(&mut tx, &in_progress_instance, expected_version, &tenant_id)
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
            event.action = event::action::WORKFLOW_SUBMITTED,
            event.entity_type = event::entity_type::WORKFLOW_INSTANCE,
            event.entity_id = %instance_id,
            event.actor_id = %in_progress_instance.initiated_by(),
            event.tenant_id = %tenant_id,
            event.result = event::result::SUCCESS,
            "ワークフロー申請"
        );

        // 承認依頼通知を送信（fire-and-forget）
        self.send_approval_request_notification(&in_progress_instance, &steps, &tenant_id)
            .await;

        Ok(in_progress_instance)
    }

    // ===== display_number 対応メソッド（申請系） =====

    /// display_number でワークフローを申請する
    ///
    /// BFF が公開 API で display_number を使う場合に、
    /// 1回の呼び出しで申請を完了する。
    ///
    /// ## 引数
    ///
    /// - `input`: 申請入力
    /// - `display_number`: 表示用連番
    /// - `tenant_id`: テナント ID
    ///
    /// ## 戻り値
    ///
    /// - `Ok(instance)`: 申請後のワークフローインスタンス
    /// - `Err(NotFound)`: インスタンスが見つからない場合
    /// - `Err(_)`: データベースエラー
    pub async fn submit_workflow_by_display_number(
        &self,
        input: SubmitWorkflowInput,
        display_number: DisplayNumber,
        tenant_id: TenantId,
    ) -> Result<WorkflowInstance, CoreError> {
        // display_number → WorkflowInstanceId を解決
        let instance = self
            .deps
            .instance_repo
            .find_by_display_number(display_number, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        // 既存の submit_workflow を呼び出し
        self.submit_workflow(input, instance.id().clone(), tenant_id)
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
        repository::{WorkflowInstanceRepositoryTestExt, WorkflowStepRepository},
    };

    use super::super::super::test_helpers::{
        build_sut,
        build_sut_with_notification,
        single_approval_definition_json,
        two_step_approval_definition_json,
    };
    use crate::{
        error::CoreError,
        usecase::workflow::{StepApprover, SubmitWorkflowInput},
    };

    #[tokio::test]
    async fn test_submit_workflow_1段階承認の正常系() {
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
        });
        let published_definition = definition.published(now).unwrap();
        definition_repo.add_definition(published_definition.clone());

        // 下書きのインスタンスを作成
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: published_definition.id().clone(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        });
        instance_repo.insert_for_test(&instance).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

        let input = SubmitWorkflowInput {
            approvers: vec![StepApprover {
                step_id:     "approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
        };

        // Act
        let result = sut
            .submit_workflow(input, instance.id().clone(), tenant_id.clone())
            .await;

        // Assert
        let result = result.unwrap();
        let expected = instance
            .submitted(now)
            .unwrap()
            .with_current_step("approval".to_string(), now)
            .unwrap();
        assert_eq!(result, expected);

        // ステップが作成されていることを確認
        let steps = step_repo
            .find_by_instance(result.id(), &tenant_id)
            .await
            .unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].assigned_to(), Some(&approver_id));
        assert_eq!(
            steps[0].status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Active
        );
    }

    #[tokio::test]
    async fn test_submit_workflow_2段階承認の正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver1_id = UserId::new();
        let approver2_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

        // 2段階承認の定義を追加
        let definition = WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id: tenant_id.clone(),
            name: WorkflowName::new("2段階承認").unwrap(),
            description: Some("テスト用定義".to_string()),
            definition: two_step_approval_definition_json(),
            created_by: user_id.clone(),
            now,
        });
        let published_definition = definition.published(now).unwrap();
        definition_repo.add_definition(published_definition.clone());

        // 下書きのインスタンスを作成
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: published_definition.id().clone(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        });
        instance_repo.insert_for_test(&instance).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

        let input = SubmitWorkflowInput {
            approvers: vec![
                StepApprover {
                    step_id:     "manager_approval".to_string(),
                    assigned_to: approver1_id.clone(),
                },
                StepApprover {
                    step_id:     "finance_approval".to_string(),
                    assigned_to: approver2_id.clone(),
                },
            ],
        };

        // Act
        let result = sut
            .submit_workflow(input, instance.id().clone(), tenant_id.clone())
            .await;

        // Assert
        let result = result.unwrap();
        // current_step_id は最初の承認ステップ
        assert_eq!(result.current_step_id(), Some("manager_approval"));

        // 2つのステップが作成されていること
        let steps = step_repo
            .find_by_instance(result.id(), &tenant_id)
            .await
            .unwrap();
        assert_eq!(steps.len(), 2);

        // 最初のステップは Active
        assert_eq!(steps[0].step_id(), "manager_approval");
        assert_eq!(steps[0].assigned_to(), Some(&approver1_id));
        assert_eq!(
            steps[0].status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Active
        );

        // 2番目のステップは Pending
        assert_eq!(steps[1].step_id(), "finance_approval");
        assert_eq!(steps[1].assigned_to(), Some(&approver2_id));
        assert_eq!(
            steps[1].status(),
            ringiflow_domain::workflow::WorkflowStepStatus::Pending
        );
    }

    #[tokio::test]
    async fn test_submit_workflow_approversと定義のステップが一致しない場合エラー() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

        // 2段階承認の定義だが、1人しか指定しない
        let definition = WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id: tenant_id.clone(),
            name: WorkflowName::new("2段階承認").unwrap(),
            description: Some("テスト用定義".to_string()),
            definition: two_step_approval_definition_json(),
            created_by: user_id.clone(),
            now,
        });
        let published_definition = definition.published(now).unwrap();
        definition_repo.add_definition(published_definition.clone());

        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: published_definition.id().clone(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        });
        instance_repo.insert_for_test(&instance).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

        // 2段階定義に1人しか指定しない
        let input = SubmitWorkflowInput {
            approvers: vec![StepApprover {
                step_id:     "manager_approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
        };

        // Act
        let result = sut
            .submit_workflow(input, instance.id().clone(), tenant_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    // ===== 通知テスト =====

    #[tokio::test]
    async fn test_submit_workflow_正常系で承認依頼通知が送信される() {
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
            name: WorkflowName::new("経費精算申請").unwrap(),
            description: Some("テスト用定義".to_string()),
            definition: single_approval_definition_json(),
            created_by: user_id.clone(),
            now,
        });
        let published_definition = definition.published(now).unwrap();
        definition_repo.add_definition(published_definition.clone());

        // 下書きのインスタンスを作成
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: published_definition.id().clone(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        });
        instance_repo.insert_for_test(&instance).await.unwrap();

        // ユーザー情報をモックに登録
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

        let input = SubmitWorkflowInput {
            approvers: vec![StepApprover {
                step_id:     "approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
        };

        // Act
        let result = sut
            .submit_workflow(input, instance.id().clone(), tenant_id.clone())
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
    async fn test_submit_workflow_ユーザー情報取得失敗でもワークフロー操作は成功する() {
        // Arrange: ユーザー情報を登録しない（find_by_id が None を返す）
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
        });
        let published_definition = definition.published(now).unwrap();
        definition_repo.add_definition(published_definition.clone());

        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: published_definition.id().clone(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        });
        instance_repo.insert_for_test(&instance).await.unwrap();

        // ユーザー情報を登録しない（空の FakeUserRepository）
        let user_repo = FakeUserRepository::new();
        let (sut, sender) = build_sut_with_notification(
            &definition_repo,
            &instance_repo,
            &step_repo,
            Arc::new(user_repo),
            now,
        );

        let input = SubmitWorkflowInput {
            approvers: vec![StepApprover {
                step_id:     "approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
        };

        // Act
        let result = sut
            .submit_workflow(input, instance.id().clone(), tenant_id.clone())
            .await;

        // Assert: ワークフロー操作自体は成功する（fire-and-forget）
        assert!(result.is_ok());

        // Assert: 通知は送信されない（ユーザー情報がないため）
        let sent = sender.sent_emails();
        assert_eq!(sent.len(), 0, "ユーザー情報がない場合、通知は送信されない");
    }

    #[tokio::test]
    async fn test_submit_workflow_draft以外は400() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

        // 公開済みの定義を追加
        let definition = WorkflowDefinition::new(NewWorkflowDefinition {
            id:          WorkflowDefinitionId::new(),
            tenant_id:   tenant_id.clone(),
            name:        WorkflowName::new("汎用申請").unwrap(),
            description: Some("テスト用定義".to_string()),
            definition:  single_approval_definition_json(),
            created_by:  user_id.clone(),
            now:         chrono::Utc::now(),
        });
        let published_definition = definition.published(chrono::Utc::now()).unwrap();
        definition_repo.add_definition(published_definition.clone());

        // InProgress 状態のインスタンスを作成（Draft ではない）
        let now = chrono::Utc::now();
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: published_definition.id().clone(),
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

        let input = SubmitWorkflowInput {
            approvers: vec![StepApprover {
                step_id:     "approval".to_string(),
                assigned_to: approver_id.clone(),
            }],
        };

        // Act: InProgress 状態のインスタンスに対して申請を試みる
        let result = sut
            .submit_workflow(input, instance.id().clone(), tenant_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }
}
