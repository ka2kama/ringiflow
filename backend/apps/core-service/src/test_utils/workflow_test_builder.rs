//! ワークフローテストビルダー
//!
//! テストコードの重複を削減するためのビルダーパターン実装。
//! 標準的なテストデータとモックリポジトリのセットアップを提供する。

use std::sync::Arc;

use chrono::{DateTime, Utc};
use ringiflow_domain::{
    clock::FixedClock,
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayNumber, Version},
    workflow::{NewWorkflowInstance, WorkflowDefinitionId, WorkflowInstance, WorkflowInstanceId},
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
    repository::{
        WorkflowCommentRepository,
        WorkflowDefinitionRepository,
        WorkflowInstanceRepository,
        WorkflowStepRepository,
    },
};

use crate::usecase::{
    notification::{NotificationService, TemplateRenderer},
    workflow::WorkflowUseCaseImpl,
};

/// ワークフローテストのセットアップデータ
///
/// WorkflowTestBuilder が生成する SUT と Mock リポジトリのセット。
pub struct WorkflowTestSetup {
    pub sut: WorkflowUseCaseImpl,
    pub definition_repo: Arc<dyn WorkflowDefinitionRepository>,
    pub instance_repo: Arc<dyn WorkflowInstanceRepository>,
    pub step_repo: Arc<dyn WorkflowStepRepository>,
    pub comment_repo: Arc<dyn WorkflowCommentRepository>,
}

/// ワークフローテストビルダー
///
/// テストコードで繰り返し出現するセットアップコードを削減するためのビルダー。
///
/// # 使用例
///
/// ```ignore
/// use ringiflow_core_service::test_utils::WorkflowTestBuilder;
///
/// #[tokio::test]
/// async fn test_example() {
///     let builder = WorkflowTestBuilder::new();
///     let instance = builder.build_submitted_instance("テスト申請", 100);
///     let setup = builder.build_workflow_usecase_impl();
///
///     // Mock リポジトリに初期データを設定
///     setup.instance_repo.insert(&instance).await.unwrap();
///
///     // SUT を使ってテスト
///     let result = setup.sut.some_method(...).await;
/// }
/// ```
pub struct WorkflowTestBuilder {
    tenant_id: TenantId,
    user_id:   UserId,
    now:       DateTime<Utc>,
}

impl WorkflowTestBuilder {
    /// デフォルト値で新しいビルダーを作成
    pub fn new() -> Self {
        Self {
            tenant_id: TenantId::new(),
            user_id:   UserId::new(),
            now:       Utc::now(),
        }
    }

    /// テナントIDを指定
    pub fn with_tenant_id(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    /// ユーザーIDを指定
    pub fn with_user_id(mut self, user_id: UserId) -> Self {
        self.user_id = user_id;
        self
    }

    /// 現在時刻を指定
    pub fn with_now(mut self, now: DateTime<Utc>) -> Self {
        self.now = now;
        self
    }

    /// ビルダーの tenant_id を取得
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    /// ビルダーの user_id を取得
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    /// ビルダーの now を取得
    pub fn now(&self) -> DateTime<Utc> {
        self.now
    }

    /// 標準的なワークフローインスタンスを作成（submitted状態）
    ///
    /// # 引数
    ///
    /// - `title`: ワークフロータイトル
    /// - `display_number`: 表示番号
    ///
    /// # 戻り値
    ///
    /// submitted状態のワークフローインスタンス（current_stepが"approval"
    /// に設定済み）
    pub fn build_submitted_instance(&self, title: &str, display_number: i64) -> WorkflowInstance {
        WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: self.tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(display_number).unwrap(),
            title: title.to_string(),
            form_data: serde_json::json!({}),
            initiated_by: self.user_id.clone(),
            now: self.now,
        })
        .submitted(self.now)
        .unwrap()
        .with_current_step("approval".to_string(), self.now)
        .unwrap()
    }

    /// Mock リポジトリ群を含む SUT（System Under Test）を構築
    ///
    /// # 戻り値
    ///
    /// WorkflowTestSetup（SUT と各 Mock リポジトリへの参照を含む）
    pub fn build_workflow_usecase_impl(&self) -> WorkflowTestSetup {
        let definition_repo: Arc<dyn WorkflowDefinitionRepository> =
            Arc::new(MockWorkflowDefinitionRepository::new());
        let instance_repo: Arc<dyn WorkflowInstanceRepository> =
            Arc::new(MockWorkflowInstanceRepository::new());
        let step_repo: Arc<dyn WorkflowStepRepository> =
            Arc::new(MockWorkflowStepRepository::new());
        let comment_repo: Arc<dyn WorkflowCommentRepository> =
            Arc::new(MockWorkflowCommentRepository::new());

        let notification_service = Arc::new(NotificationService::new(
            Arc::new(MockNotificationSender::new()),
            TemplateRenderer::new().unwrap(),
            Arc::new(MockNotificationLogRepository::new()),
            "http://localhost:5173".to_string(),
        ));

        let sut = WorkflowUseCaseImpl::new(
            definition_repo.clone(),
            instance_repo.clone(),
            step_repo.clone(),
            comment_repo.clone(),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(self.now)),
            Arc::new(MockTransactionManager),
            notification_service,
        );

        WorkflowTestSetup {
            sut,
            definition_repo,
            instance_repo,
            step_repo,
            comment_repo,
        }
    }
}

impl Default for WorkflowTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_デフォルト値が設定される() {
        // Act
        let builder = WorkflowTestBuilder::new();

        // Assert
        assert!(builder.tenant_id != TenantId::new());
        assert!(builder.user_id != UserId::new());
    }

    #[test]
    fn test_with_tenant_id_カスタマイズできる() {
        // Arrange
        let custom_tenant_id = TenantId::new();

        // Act
        let builder = WorkflowTestBuilder::new().with_tenant_id(custom_tenant_id.clone());

        // Assert
        assert_eq!(builder.tenant_id, custom_tenant_id);
    }

    #[test]
    fn test_with_user_id_カスタマイズできる() {
        // Arrange
        let custom_user_id = UserId::new();

        // Act
        let builder = WorkflowTestBuilder::new().with_user_id(custom_user_id.clone());

        // Assert
        assert_eq!(builder.user_id, custom_user_id);
    }

    #[test]
    fn test_with_now_カスタマイズできる() {
        // Arrange
        let custom_now = Utc::now();

        // Act
        let builder = WorkflowTestBuilder::new().with_now(custom_now);

        // Assert
        assert_eq!(builder.now, custom_now);
    }

    #[test]
    fn test_build_submitted_instance_標準インスタンスが作成される() {
        // Arrange
        let builder = WorkflowTestBuilder::new();

        // Act
        let instance = builder.build_submitted_instance("テスト申請", 100);

        // Assert
        assert_eq!(instance.title(), "テスト申請");
        assert_eq!(instance.display_number(), DisplayNumber::new(100).unwrap());
        assert_eq!(instance.tenant_id(), &builder.tenant_id);
        assert_eq!(instance.initiated_by(), &builder.user_id);
        assert!(instance.submitted_at().is_some());
        assert_eq!(instance.current_step_id().unwrap(), "approval");
    }

    #[tokio::test]
    async fn test_build_workflow_usecase_impl_sutが作成される() {
        // Arrange
        let builder = WorkflowTestBuilder::new();

        // Act
        let setup = builder.build_workflow_usecase_impl();

        // Assert
        // WorkflowTestSetup が正常に作成できることを確認
        assert!(Arc::strong_count(&setup.instance_repo) >= 2);
    }
}
