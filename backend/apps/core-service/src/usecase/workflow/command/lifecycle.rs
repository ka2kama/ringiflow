//! ワークフローのライフサイクル管理（作成・申請・再申請）

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayIdEntityType, DisplayNumber},
    workflow::{
        NewWorkflowInstance,
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
    usecase::workflow::{
        CreateWorkflowInput,
        ResubmitWorkflowInput,
        SubmitWorkflowInput,
        WorkflowUseCaseImpl,
        WorkflowWithSteps,
    },
};

impl WorkflowUseCaseImpl {
    /// ワークフローインスタンスを作成する（下書き）
    ///
    /// ## 処理フロー
    ///
    /// 1. ワークフロー定義が存在するか確認
    /// 2. 公開済み (published) であるか確認
    /// 3. WorkflowInstance を draft として作成
    /// 4. リポジトリに保存
    ///
    /// ## エラー
    ///
    /// - ワークフロー定義が見つからない場合
    /// - ワークフロー定義が公開されていない場合
    /// - データベースエラー
    pub async fn create_workflow(
        &self,
        input: CreateWorkflowInput,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowInstance, CoreError> {
        // 1. ワークフロー定義を取得
        let definition = self
            .definition_repo
            .find_by_id(&input.definition_id, &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
            .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

        // 2. 公開済みであるか確認
        if definition.status() != ringiflow_domain::workflow::WorkflowDefinitionStatus::Published {
            return Err(CoreError::BadRequest(
                "公開されていないワークフロー定義です".to_string(),
            ));
        }

        // 3. WorkflowInstance を draft として作成
        let now = self.clock.now();
        let display_number = self
            .counter_repo
            .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowInstance)
            .await
            .map_err(|e| CoreError::Internal(format!("採番に失敗: {}", e)))?;
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id,
            definition_id: input.definition_id,
            definition_version: definition.version(),
            display_number,
            title: input.title,
            form_data: input.form_data,
            initiated_by: user_id,
            now,
        });

        // 4. リポジトリに保存
        self.instance_repo
            .insert(&instance)
            .await
            .map_err(|e| CoreError::Internal(format!("インスタンスの保存に失敗: {}", e)))?;

        Ok(instance)
    }

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
            .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
            .ok_or_else(|| {
                CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
            })?;

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
            .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
            .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

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
            .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
            .ok_or_else(|| {
                CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
            })?;

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
            .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
            .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

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
            .update_with_version_check(&resubmitted_instance, instance_expected_version)
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

        Ok(WorkflowWithSteps {
            instance: resubmitted_instance,
            steps,
        })
    }

    // ===== display_number 対応メソッド（ライフサイクル系） =====

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
            .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
            .ok_or_else(|| {
                CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
            })?;

        // 既存の submit_workflow を呼び出し
        self.submit_workflow(input, instance.id().clone(), tenant_id)
            .await
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
            .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
            .ok_or_else(|| {
                CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
            })?;

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
        repository::{WorkflowInstanceRepository, WorkflowStepRepository},
    };

    use super::super::test_helpers::{
        single_approval_definition_json,
        two_step_approval_definition_json,
    };
    use crate::{
        error::CoreError,
        usecase::workflow::{
            CreateWorkflowInput,
            ResubmitWorkflowInput,
            StepApprover,
            SubmitWorkflowInput,
            WorkflowUseCaseImpl,
        },
    };

    #[tokio::test]
    async fn test_create_workflow_正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        // 公開済みの定義を追加
        let now = chrono::Utc::now();
        let definition = WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id: tenant_id.clone(),
            name: WorkflowName::new("汎用申請").unwrap(),
            description: Some("テスト用定義".to_string()),
            definition: serde_json::json!({"steps": []}),
            created_by: user_id.clone(),
            now,
        });
        let published_definition = definition.published(now).unwrap();
        definition_repo.add_definition(published_definition.clone());

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo.clone()),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = CreateWorkflowInput {
            definition_id: published_definition.id().clone(),
            title:         "テスト申請".to_string(),
            form_data:     serde_json::json!({"note": "test"}),
        };

        // Act
        let result = sut
            .create_workflow(input, tenant_id.clone(), user_id.clone())
            .await;

        // Assert
        let result = result.unwrap();

        // result の ID を使って expected を構築（ID は内部で UUID v7 生成されるため）
        let expected = WorkflowInstance::new(NewWorkflowInstance {
            id: result.id().clone(),
            tenant_id: tenant_id.clone(),
            definition_id: published_definition.id().clone(),
            definition_version: published_definition.version(),
            display_number: DisplayNumber::new(1).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({"note": "test"}),
            initiated_by: user_id.clone(),
            now,
        });
        assert_eq!(result, expected);

        // リポジトリに保存されていることを確認
        let saved = instance_repo
            .find_by_id(result.id(), &tenant_id)
            .await
            .unwrap();
        assert_eq!(saved, Some(expected));
    }

    #[tokio::test]
    async fn test_create_workflow_定義が見つからない() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let now = chrono::Utc::now();
        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
        );

        let input = CreateWorkflowInput {
            definition_id: WorkflowDefinitionId::new(),
            title:         "テスト申請".to_string(),
            form_data:     serde_json::json!({}),
        };

        // Act
        let result = sut.create_workflow(input, tenant_id, user_id).await;

        // Assert
        assert!(matches!(result, Err(CoreError::NotFound(_))));
    }

    // ===== submit_workflow テスト =====

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

    // ===== resubmit_workflow テスト =====

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
