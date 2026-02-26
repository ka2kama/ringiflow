//! ワークフローの作成（下書き）

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::DisplayIdEntityType,
    workflow::{NewWorkflowInstance, WorkflowInstance, WorkflowInstanceId},
};
use ringiflow_shared::{event_log::event, log_business_event};

use crate::{
    error::CoreError,
    usecase::{
        helpers::FindResultExt,
        workflow::{CreateWorkflowInput, WorkflowUseCaseImpl},
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
            .or_not_found("ワークフロー定義")?;

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
        let mut tx = self
            .tx_manager
            .begin()
            .await
            .map_err(|e| CoreError::Internal(format!("トランザクション開始に失敗: {}", e)))?;
        self.instance_repo
            .insert(&mut tx, &instance)
            .await
            .map_err(|e| CoreError::Internal(format!("インスタンスの保存に失敗: {}", e)))?;
        tx.commit()
            .await
            .map_err(|e| CoreError::Internal(format!("トランザクションコミットに失敗: {}", e)))?;

        log_business_event!(
            event.category = event::category::WORKFLOW,
            event.action = event::action::WORKFLOW_CREATED,
            event.entity_type = event::entity_type::WORKFLOW_INSTANCE,
            event.entity_id = %instance.id(),
            event.actor_id = %instance.initiated_by(),
            event.tenant_id = %instance.tenant_id(),
            event.result = event::result::SUCCESS,
            "ワークフロー作成"
        );

        Ok(instance)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ringiflow_domain::{
        clock::FixedClock,
        tenant::TenantId,
        user::UserId,
        value_objects::{DisplayNumber, WorkflowName},
        workflow::{
            NewWorkflowDefinition,
            NewWorkflowInstance,
            WorkflowDefinition,
            WorkflowDefinitionId,
            WorkflowInstance,
        },
    };
    use ringiflow_infra::{
        mock::{
            MockDisplayIdCounterRepository,
            MockNotificationLogRepository,
            MockNotificationSender,
            MockTransactionManager,
            MockUserRepository,
            MockWorkflowCommentRepository,
            MockWorkflowDefinitionRepository,
            MockWorkflowInstanceRepository,
            MockWorkflowStepRepository,
        },
        repository::WorkflowInstanceRepository,
    };

    use crate::{
        error::CoreError,
        usecase::{
            notification::{NotificationService, TemplateRenderer},
            workflow::{CreateWorkflowInput, WorkflowUseCaseImpl},
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

        let notification_service = Arc::new(NotificationService::new(
            Arc::new(MockNotificationSender::new()),
            TemplateRenderer::new().unwrap(),
            Arc::new(MockNotificationLogRepository::new()),
            "http://localhost:5173".to_string(),
        ));

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo.clone()),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
            notification_service,
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

        let notification_service = Arc::new(NotificationService::new(
            Arc::new(MockNotificationSender::new()),
            TemplateRenderer::new().unwrap(),
            Arc::new(MockNotificationLogRepository::new()),
            "http://localhost:5173".to_string(),
        ));

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
            notification_service,
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
}
