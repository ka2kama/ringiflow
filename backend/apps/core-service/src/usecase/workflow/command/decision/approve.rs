//! ワークフローステップの承認

use ringiflow_domain::{
    notification::WorkflowNotification,
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayId, DisplayNumber, display_prefix},
    workflow::WorkflowStepId,
};
use ringiflow_shared::{event_log::event, log_business_event};

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
            .deps
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
        let now = self.deps.clock.now();
        let step_expected_version = step.version();
        let current_step_id = step.step_id().to_string();
        let approved_step = step
            .approve(input.comment, now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 5. インスタンスを取得
        let instance = self
            .deps
            .instance_repo
            .find_by_id(approved_step.instance_id(), &tenant_id)
            .await
            .or_not_found("インスタンス")?;

        let instance_expected_version = instance.version();

        // 6. 定義から承認ステップの順序を取得し、次ステップを判定
        let definition = self
            .deps
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

        // 8. 次ステップがあればデータを準備（トランザクション開始前に読み取り）
        let activated_next_step = if let Some(next_step_id) = next_step_to_activate {
            let all_steps = self
                .fetch_instance_steps(updated_instance.id(), &tenant_id)
                .await?;

            all_steps
                .into_iter()
                .find(|s| s.step_id() == next_step_id)
                .map(|next_step| {
                    let version = next_step.version();
                    let activated = next_step.activated(now);
                    (activated, version)
                })
        } else {
            None
        };

        // 9. 全更新を単一トランザクションで実行
        let mut tx = self.begin_tx().await?;

        self.save_step(&mut tx, &approved_step, step_expected_version, &tenant_id)
            .await?;

        if let Some((ref activated_step, next_expected_version)) = activated_next_step {
            self.save_step(&mut tx, activated_step, next_expected_version, &tenant_id)
                .await?;
        }

        self.save_instance(
            &mut tx,
            &updated_instance,
            instance_expected_version,
            &tenant_id,
        )
        .await?;

        self.commit_tx(tx).await?;

        // 10. 保存後のステップ一覧を取得して返却
        let steps = self
            .fetch_instance_steps(updated_instance.id(), &tenant_id)
            .await?;

        log_business_event!(
            event.category = event::category::WORKFLOW,
            event.action = event::action::STEP_APPROVED,
            event.entity_type = event::entity_type::WORKFLOW_STEP,
            event.entity_id = %step_id,
            event.actor_id = %user_id,
            event.tenant_id = %tenant_id,
            event.result = event::result::SUCCESS,
            "承認ステップ完了"
        );

        // 通知送信（fire-and-forget）
        self.send_approval_notifications(
            &updated_instance,
            &approved_step,
            activated_next_step.as_ref().map(|(s, _)| s),
            &steps,
            &tenant_id,
        )
        .await;

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
            .deps
            .instance_repo
            .find_by_display_number(workflow_display_number, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        // display_number → WorkflowStepId を解決
        let step = self
            .deps
            .step_repo
            .find_by_display_number(step_display_number, instance.id(), &tenant_id)
            .await
            .or_not_found("ステップ")?;

        // 既存の approve_step を呼び出し
        self.approve_step(input, step.id().clone(), tenant_id, user_id)
            .await
    }

    /// 承認操作後の通知を送信する（fire-and-forget）
    ///
    /// 最終ステップか中間ステップかで送信する通知が異なる:
    /// - 最終ステップ: `Approved` → 申請者
    /// - 中間ステップ: `StepApproved` → 申請者 + `ApprovalRequest` → 次の承認者
    async fn send_approval_notifications(
        &self,
        instance: &ringiflow_domain::workflow::WorkflowInstance,
        approved_step: &ringiflow_domain::workflow::WorkflowStep,
        activated_next_step: Option<&ringiflow_domain::workflow::WorkflowStep>,
        all_steps: &[ringiflow_domain::workflow::WorkflowStep],
        tenant_id: &TenantId,
    ) {
        let workflow_display_id =
            DisplayId::new(display_prefix::WORKFLOW_INSTANCE, instance.display_number())
                .to_string();

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

        if activated_next_step.is_some() {
            // 中間ステップ: StepApproved → 申請者
            let step_approved = WorkflowNotification::StepApproved {
                workflow_title:      instance.title().to_string(),
                workflow_display_id: workflow_display_id.clone(),
                step_name:           approved_step.step_name().to_string(),
                approver_name:       self.resolve_user_name(approved_step.assigned_to()).await,
                applicant_email:     applicant.email().as_str().to_string(),
                applicant_user_id:   applicant.id().clone(),
            };
            self.deps
                .notification_service
                .notify(step_approved, tenant_id, instance.id())
                .await;

            // ApprovalRequest → 次の承認者（既存ヘルパーを再利用）
            self.send_approval_request_notification(instance, all_steps, tenant_id)
                .await;
        } else {
            // 最終ステップ: Approved → 申請者
            let approved = WorkflowNotification::Approved {
                workflow_title: instance.title().to_string(),
                workflow_display_id,
                applicant_email: applicant.email().as_str().to_string(),
                applicant_user_id: applicant.id().clone(),
            };
            self.deps
                .notification_service
                .notify(approved, tenant_id, instance.id())
                .await;
        }
    }

    /// ユーザー ID からユーザー名を解決する（通知用）
    ///
    /// 取得できない場合は空文字を返す。
    async fn resolve_user_name(&self, user_id: Option<&UserId>) -> String {
        let Some(user_id) = user_id else {
            return String::new();
        };
        match self.deps.user_repo.find_by_id(user_id).await {
            Ok(Some(user)) => user.name().as_str().to_string(),
            _ => String::new(),
        }
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
        fake::{
            FakeUserRepository,
            FakeWorkflowDefinitionRepository,
            FakeWorkflowInstanceRepository,
            FakeWorkflowStepRepository,
        },
        repository::{WorkflowInstanceRepositoryTestExt, WorkflowStepRepositoryTestExt},
    };

    use super::super::super::test_helpers::{
        build_sut,
        build_sut_with_notification,
        setup_two_step_approval,
        single_approval_definition_json,
    };
    use crate::{
        error::CoreError,
        usecase::workflow::{ApproveRejectInput, WorkflowWithSteps},
    };

    #[tokio::test]
    async fn test_approve_step_正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
        .with_current_step("approval".to_string(), now)
        .unwrap();
        instance_repo.insert_for_test(&instance).await.unwrap();

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
        step_repo.insert_for_test(&step, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
            assigned_to: Some(approver_id.clone()), // approver_id に割り当て
            now: chrono::Utc::now(),
        })
        .activated(chrono::Utc::now());
        step_repo.insert_for_test(&step, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
        step_repo.insert_for_test(&step, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

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
            now: chrono::Utc::now(),
        })
        .activated(chrono::Utc::now());
        step_repo.insert_for_test(&step, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

        definition_repo.add_definition(definition);
        instance_repo.insert_for_test(&instance).await.unwrap();
        step_repo.insert_for_test(&step1, &tenant_id).await.unwrap();
        step_repo.insert_for_test(&step2, &tenant_id).await.unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

        definition_repo.add_definition(definition);

        // ステップ1は既に承認済み、current_step_id は finance_approval に移行済み
        let instance_at_step2 = instance
            .advance_to_next_step("finance_approval".to_string(), now)
            .unwrap();
        instance_repo
            .insert_for_test(&instance_at_step2)
            .await
            .unwrap();

        let completed_step1 = step1.approve(Some("上長承認OK".to_string()), now).unwrap();
        let active_step2 = step2.activated(now);
        step_repo
            .insert_for_test(&completed_step1, &tenant_id)
            .await
            .unwrap();
        step_repo
            .insert_for_test(&active_step2, &tenant_id)
            .await
            .unwrap();

        let sut = build_sut(&definition_repo, &instance_repo, &step_repo, now);

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

    // ===== 通知テスト =====

    #[tokio::test]
    async fn test_approve_step_最終承認で承認完了通知が申請者に送信される() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new(); // 申請者
        let approver_id = UserId::new(); // 承認者
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

        let (sut, sender) = build_sut_with_notification(
            &definition_repo,
            &instance_repo,
            &step_repo,
            Arc::new(user_repo),
            now,
        );

        let input = ApproveRejectInput {
            version: step.version(),
            comment: Some("承認します".to_string()),
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

        // Assert: ワークフロー操作は成功
        assert!(result.is_ok());

        // Assert: 承認完了通知が申請者に送信されている
        let sent = sender.sent_emails();
        assert_eq!(sent.len(), 1, "承認完了メールが1通送信されるべき");
        assert_eq!(sent[0].to, "tanaka@example.com");
        assert!(
            sent[0].subject.contains("承認完了"),
            "件名に「承認完了」が含まれるべき: {}",
            sent[0].subject
        );
        assert!(
            sent[0].subject.contains("テスト申請"),
            "件名にワークフロータイトルが含まれるべき: {}",
            sent[0].subject
        );
    }

    #[tokio::test]
    async fn test_approve_step_中間ステップで通知2通が送信される() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new(); // 申請者
        let approver1_id = UserId::new(); // ステップ1の承認者
        let approver2_id = UserId::new(); // ステップ2の承認者
        let now = chrono::Utc::now();

        let (definition, instance, step1, step2) =
            setup_two_step_approval(&tenant_id, &user_id, &approver1_id, &approver2_id, now);

        let definition_repo = FakeWorkflowDefinitionRepository::new();
        let instance_repo = FakeWorkflowInstanceRepository::new();
        let step_repo = FakeWorkflowStepRepository::new();

        definition_repo.add_definition(definition);
        instance_repo.insert_for_test(&instance).await.unwrap();
        step_repo.insert_for_test(&step1, &tenant_id).await.unwrap();
        step_repo.insert_for_test(&step2, &tenant_id).await.unwrap();

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
            approver1_id.clone(),
            tenant_id.clone(),
            DisplayNumber::new(2).unwrap(),
            Email::new("suzuki@example.com").unwrap(),
            UserName::new("鈴木一郎").unwrap(),
            now,
        ));
        user_repo.add_user(User::new(
            approver2_id.clone(),
            tenant_id.clone(),
            DisplayNumber::new(3).unwrap(),
            Email::new("yamada@example.com").unwrap(),
            UserName::new("山田花子").unwrap(),
            now,
        ));

        let (sut, sender) = build_sut_with_notification(
            &definition_repo,
            &instance_repo,
            &step_repo,
            Arc::new(user_repo),
            now,
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

        // Assert: ワークフロー操作は成功
        assert!(result.is_ok());

        // Assert: 2通の通知が送信されている
        // 1. StepApproved → 申請者
        // 2. ApprovalRequest → 次の承認者
        let sent = sender.sent_emails();
        assert_eq!(
            sent.len(),
            2,
            "ステップ承認通知と承認依頼通知の計2通が送信されるべき"
        );

        // 申請者への StepApproved 通知
        let step_approved_mail = sent
            .iter()
            .find(|m| m.to == "tanaka@example.com")
            .expect("申請者への通知があるべき");
        assert!(
            step_approved_mail.subject.contains("ステップ承認"),
            "件名に「ステップ承認」が含まれるべき: {}",
            step_approved_mail.subject
        );

        // 次の承認者への ApprovalRequest 通知
        let approval_request_mail = sent
            .iter()
            .find(|m| m.to == "yamada@example.com")
            .expect("次の承認者への通知があるべき");
        assert!(
            approval_request_mail.subject.contains("承認依頼"),
            "件名に「承認依頼」が含まれるべき: {}",
            approval_request_mail.subject
        );
    }

    #[tokio::test]
    async fn test_approve_step_ユーザー情報取得失敗でも承認操作は成功する() {
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

        // ユーザー情報を登録しない（空の FakeUserRepository）
        let user_repo = FakeUserRepository::new();
        let (sut, sender) = build_sut_with_notification(
            &definition_repo,
            &instance_repo,
            &step_repo,
            Arc::new(user_repo),
            now,
        );

        let input = ApproveRejectInput {
            version: step.version(),
            comment: Some("承認します".to_string()),
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

        // Assert: ワークフロー操作自体は成功する（fire-and-forget）
        assert!(result.is_ok());

        // Assert: 通知は送信されない（ユーザー情報がないため）
        let sent = sender.sent_emails();
        assert_eq!(sent.len(), 0, "ユーザー情報がない場合、通知は送信されない");
    }
}
