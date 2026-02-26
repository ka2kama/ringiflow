//! ワークフローのコラボレーション（コメント投稿）

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::DisplayNumber,
    workflow::{
        CommentBody,
        NewWorkflowComment,
        WorkflowComment,
        WorkflowCommentId,
        WorkflowInstance,
    },
};

use crate::{
    error::CoreError,
    usecase::{
        helpers::FindResultExt,
        workflow::{PostCommentInput, WorkflowUseCaseImpl},
    },
};

impl WorkflowUseCaseImpl {
    /// ワークフローにコメントを投稿する
    ///
    /// ## 処理フロー
    ///
    /// 1. ワークフローインスタンスを取得
    /// 2. 権限チェック（申請者 OR 承認者のみ投稿可能）
    /// 3. コメント本文のバリデーション
    /// 4. コメントを作成して保存
    ///
    /// ## エラー
    ///
    /// - インスタンスが見つからない場合: 404
    /// - 関与者でない場合: 403
    /// - コメント本文が不正な場合: 400
    /// - データベースエラー
    pub async fn post_comment(
        &self,
        input: PostCommentInput,
        display_number: DisplayNumber,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowComment, CoreError> {
        // 1. ワークフローインスタンスを取得
        let instance = self
            .instance_repo
            .find_by_display_number(display_number, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        // 2. 権限チェック
        if !self.is_participant(&instance, &user_id, &tenant_id).await? {
            return Err(CoreError::Forbidden(
                "このワークフローにコメントする権限がありません".to_string(),
            ));
        }

        // 3. コメント本文のバリデーション
        let body =
            CommentBody::new(input.body).map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 4. コメントを作成して保存
        let now = self.clock.now();
        let comment = WorkflowComment::new(NewWorkflowComment {
            id: WorkflowCommentId::new(),
            tenant_id: tenant_id.clone(),
            instance_id: instance.id().clone(),
            posted_by: user_id,
            body,
            now,
        });

        self.comment_repo
            .insert(&comment, &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("コメントの保存に失敗: {}", e)))?;

        Ok(comment)
    }

    /// ユーザーがワークフローの関与者かチェックする
    ///
    /// 関与者 = 申請者 OR いずれかのステップの承認者
    async fn is_participant(
        &self,
        instance: &WorkflowInstance,
        user_id: &UserId,
        tenant_id: &TenantId,
    ) -> Result<bool, CoreError> {
        // 申請者チェック
        if instance.initiated_by() == user_id {
            return Ok(true);
        }

        // 承認者チェック
        let steps = self
            .step_repo
            .find_by_instance(instance.id(), tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

        Ok(steps.iter().any(|s| s.assigned_to() == Some(user_id)))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ringiflow_domain::{
        clock::FixedClock,
        tenant::TenantId,
        user::UserId,
        value_objects::{DisplayNumber, Version},
        workflow::{
            NewWorkflowInstance,
            NewWorkflowStep,
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
            MockNotificationLogRepository,
            MockNotificationSender,
            MockTransactionManager,
            MockUserRepository,
            MockWorkflowCommentRepository,
            MockWorkflowDefinitionRepository,
            MockWorkflowInstanceRepository,
            MockWorkflowStepRepository,
        },
        repository::{WorkflowInstanceRepositoryTestExt, WorkflowStepRepositoryTestExt},
    };

    use crate::{
        error::CoreError,
        usecase::{
            notification::{NotificationService, TemplateRenderer},
            workflow::{PostCommentInput, WorkflowUseCaseImpl},
        },
    };

    #[tokio::test]
    async fn test_post_comment_申請者がコメントを投稿できる() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();
        let comment_repo = MockWorkflowCommentRepository::new();

        // InProgress のインスタンスを作成（user_id が申請者）
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
            Arc::new(comment_repo),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
            notification_service,
        );

        let input = PostCommentInput {
            body: "テストコメント".to_string(),
        };

        // Act
        let result = sut
            .post_comment(input, DisplayNumber::new(100).unwrap(), tenant_id, user_id)
            .await;

        // Assert
        assert!(result.is_ok());
        let comment = result.unwrap();
        assert_eq!(comment.body().as_str(), "テストコメント");
    }

    #[tokio::test]
    async fn test_post_comment_承認者がコメントを投稿できる() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();
        let comment_repo = MockWorkflowCommentRepository::new();

        // InProgress のインスタンスを作成（user_id が申請者）
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

        // approver_id が承認者のステップを作成
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
            Arc::new(comment_repo),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
            notification_service,
        );

        let input = PostCommentInput {
            body: "承認者のコメント".to_string(),
        };

        // Act: 承認者がコメントを投稿
        let result = sut
            .post_comment(
                input,
                DisplayNumber::new(100).unwrap(),
                tenant_id,
                approver_id,
            )
            .await;

        // Assert
        assert!(result.is_ok());
        let comment = result.unwrap();
        assert_eq!(comment.body().as_str(), "承認者のコメント");
    }

    #[tokio::test]
    async fn test_post_comment_関与していないユーザーは403() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let other_user_id = UserId::new(); // 関与していないユーザー
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();
        let comment_repo = MockWorkflowCommentRepository::new();

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
            Arc::new(comment_repo),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
            notification_service,
        );

        let input = PostCommentInput {
            body: "無関係なコメント".to_string(),
        };

        // Act: 関与していないユーザーがコメントを試みる
        let result = sut
            .post_comment(
                input,
                DisplayNumber::new(100).unwrap(),
                tenant_id,
                other_user_id,
            )
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_post_comment_ワークフローが見つからない場合404() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();
        let comment_repo = MockWorkflowCommentRepository::new();

        // インスタンスを作成しない

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
            Arc::new(comment_repo),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
            notification_service,
        );

        let input = PostCommentInput {
            body: "存在しないワークフローへのコメント".to_string(),
        };

        // Act
        let result = sut
            .post_comment(input, DisplayNumber::new(999).unwrap(), tenant_id, user_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::NotFound(_))));
    }
}
