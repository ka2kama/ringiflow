//! ワークフローの申請

use ringiflow_domain::{
    tenant::TenantId,
    value_objects::{DisplayIdEntityType, DisplayNumber},
    workflow::{
        NewWorkflowStep,
        WorkflowInstance,
        WorkflowInstanceId,
        WorkflowInstanceStatus,
        WorkflowStep,
        WorkflowStepId,
    },
};
use ringiflow_infra::InfraError;

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
            .definition_repo
            .find_by_id(instance.definition_id(), &tenant_id)
            .await
            .or_not_found("ワークフロー定義")?;

        // 4. 定義から承認ステップを抽出
        let approval_step_defs = definition
            .extract_approval_steps()
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // approvers と定義のステップの整合性を検証
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

        // 5. 各承認ステップを作成
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

        // 6. ワークフローインスタンスを申請済みに遷移
        let expected_version = instance.version();
        let first_step_id = approval_step_defs[0].id.clone();
        let submitted_instance = instance
            .submitted(now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // current_step_id を最初の承認ステップに設定して in_progress に遷移
        let in_progress_instance = submitted_instance.with_current_step(first_step_id, now);

        // 7. インスタンスとステップを保存
        self.instance_repo
            .update_with_version_check(&in_progress_instance, expected_version)
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
        repository::{WorkflowInstanceRepository, WorkflowStepRepository},
    };

    use super::super::super::test_helpers::{
        single_approval_definition_json,
        two_step_approval_definition_json,
    };
    use crate::{
        error::CoreError,
        usecase::workflow::{StepApprover, SubmitWorkflowInput, WorkflowUseCaseImpl},
    };

    #[tokio::test]
    async fn test_submit_workflow_1段階承認の正常系() {
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
            .with_current_step("approval".to_string(), now);
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

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

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

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

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

    #[tokio::test]
    async fn test_submit_workflow_draft以外は400() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

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
