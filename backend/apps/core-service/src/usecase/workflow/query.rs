//! ワークフローユースケースの読み取り操作

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::DisplayNumber,
    workflow::{WorkflowComment, WorkflowInstanceId},
};

use super::{WorkflowUseCaseImpl, WorkflowWithSteps};
use crate::{error::CoreError, usecase::helpers::FindResultExt};

impl WorkflowUseCaseImpl {
    // ===== GET 系メソッド =====

    /// 自分の申請一覧を取得する
    ///
    /// ログインユーザーが申請したワークフローインスタンスの一覧を返す。
    ///
    /// ## 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `user_id`: ユーザー ID
    ///
    /// ## 戻り値
    ///
    /// - `Ok(Vec<WorkflowInstance>)`: 申請一覧
    /// - `Err(_)`: データベースエラー
    pub async fn list_my_workflows(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<Vec<ringiflow_domain::workflow::WorkflowInstance>, CoreError> {
        self.instance_repo
            .find_by_initiated_by(&tenant_id, &user_id)
            .await
            .map_err(|e| CoreError::Internal(format!("申請一覧の取得に失敗: {}", e)))
    }

    /// ワークフローインスタンスの詳細を取得する
    ///
    /// 指定された ID のワークフローインスタンスを取得する。
    ///
    /// ## 引数
    ///
    /// - `id`: ワークフローインスタンス ID
    /// - `tenant_id`: テナント ID
    ///
    /// ## 戻り値
    ///
    /// - `Ok(instance)`: ワークフローインスタンス
    /// - `Err(NotFound)`: インスタンスが見つからない場合
    /// - `Err(_)`: データベースエラー
    pub async fn get_workflow(
        &self,
        id: WorkflowInstanceId,
        tenant_id: TenantId,
    ) -> Result<WorkflowWithSteps, CoreError> {
        let instance = self
            .instance_repo
            .find_by_id(&id, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        let steps = self
            .step_repo
            .find_by_instance(&id, &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

        Ok(WorkflowWithSteps { instance, steps })
    }

    // ===== display_number 対応メソッド（読み取り） =====

    /// display_number でワークフローインスタンスの詳細を取得する
    ///
    /// BFF が公開 API で display_number を使う場合に、
    /// 1回の呼び出しでワークフロー詳細を返す。
    ///
    /// ## 引数
    ///
    /// - `display_number`: 表示用連番
    /// - `tenant_id`: テナント ID
    ///
    /// ## 戻り値
    ///
    /// - `Ok(workflow)`: ワークフロー詳細（インスタンス + ステップ）
    /// - `Err(NotFound)`: インスタンスが見つからない場合
    /// - `Err(_)`: データベースエラー
    pub async fn get_workflow_by_display_number(
        &self,
        display_number: DisplayNumber,
        tenant_id: TenantId,
    ) -> Result<WorkflowWithSteps, CoreError> {
        let instance = self
            .instance_repo
            .find_by_display_number(display_number, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        let steps = self
            .step_repo
            .find_by_instance(instance.id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

        Ok(WorkflowWithSteps { instance, steps })
    }

    // ===== コメント取得メソッド =====

    /// ワークフローのコメント一覧を取得する
    ///
    /// display_number でワークフローを特定し、そのコメント一覧を
    /// 時系列昇順で返す。
    ///
    /// ## 引数
    ///
    /// - `display_number`: 表示用連番
    /// - `tenant_id`: テナント ID
    ///
    /// ## 戻り値
    ///
    /// - `Ok(Vec<WorkflowComment>)`: コメント一覧（created_at ASC）
    /// - `Err(NotFound)`: インスタンスが見つからない場合
    /// - `Err(_)`: データベースエラー
    pub async fn list_comments(
        &self,
        display_number: DisplayNumber,
        tenant_id: TenantId,
    ) -> Result<Vec<WorkflowComment>, CoreError> {
        // 1. ワークフローインスタンスの存在確認
        let instance = self
            .instance_repo
            .find_by_display_number(display_number, &tenant_id)
            .await
            .or_not_found("ワークフローインスタンス")?;

        // 2. コメント一覧を取得
        self.comment_repo
            .find_by_instance(instance.id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("コメントの取得に失敗: {}", e)))
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
            CommentBody,
            NewWorkflowComment,
            NewWorkflowInstance,
            WorkflowComment,
            WorkflowCommentId,
            WorkflowDefinitionId,
            WorkflowInstance,
            WorkflowInstanceId,
        },
    };
    use ringiflow_infra::{
        mock::{
            MockDisplayIdCounterRepository,
            MockTransactionManager,
            MockUserRepository,
            MockWorkflowCommentRepository,
            MockWorkflowDefinitionRepository,
            MockWorkflowInstanceRepository,
            MockWorkflowStepRepository,
        },
        repository::{WorkflowCommentRepository, WorkflowInstanceRepositoryTestExt},
    };

    use super::super::WorkflowUseCaseImpl;
    use crate::error::CoreError;

    #[tokio::test]
    async fn test_list_comments_コメント一覧を取得できる() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
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
        .with_current_step("approval".to_string(), now);
        instance_repo.insert_for_test(&instance).await.unwrap();

        // コメントを2件追加
        let comment1 = WorkflowComment::new(NewWorkflowComment {
            id: WorkflowCommentId::new(),
            tenant_id: tenant_id.clone(),
            instance_id: instance.id().clone(),
            posted_by: user_id.clone(),
            body: CommentBody::new("コメント1").unwrap(),
            now,
        });
        let comment2 = WorkflowComment::new(NewWorkflowComment {
            id: WorkflowCommentId::new(),
            tenant_id: tenant_id.clone(),
            instance_id: instance.id().clone(),
            posted_by: user_id.clone(),
            body: CommentBody::new("コメント2").unwrap(),
            now,
        });
        comment_repo.insert(&comment1, &tenant_id).await.unwrap();
        comment_repo.insert(&comment2, &tenant_id).await.unwrap();

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(comment_repo),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
        );

        // Act
        let result = sut
            .list_comments(DisplayNumber::new(100).unwrap(), tenant_id)
            .await;

        // Assert
        assert!(result.is_ok());
        let comments = result.unwrap();
        assert_eq!(comments.len(), 2);
    }

    #[tokio::test]
    async fn test_list_comments_ワークフローが見つからない場合404() {
        // Arrange
        let tenant_id = TenantId::new();
        let now = chrono::Utc::now();

        let definition_repo = MockWorkflowDefinitionRepository::new();
        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();
        let comment_repo = MockWorkflowCommentRepository::new();

        // インスタンスを作成しない

        let sut = WorkflowUseCaseImpl::new(
            Arc::new(definition_repo),
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(comment_repo),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
        );

        // Act
        let result = sut
            .list_comments(DisplayNumber::new(999).unwrap(), tenant_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::NotFound(_))));
    }
}
