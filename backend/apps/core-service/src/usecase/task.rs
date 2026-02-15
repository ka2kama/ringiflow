//! # タスクユースケース
//!
//! タスク（自分にアサインされたワークフローステップ）の
//! 一覧・詳細取得に関するビジネスロジックを実装する。

use std::{collections::HashMap, sync::Arc};

use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::DisplayNumber,
    workflow::{
        WorkflowInstance,
        WorkflowInstanceId,
        WorkflowStep,
        WorkflowStepId,
        WorkflowStepStatus,
    },
};
use ringiflow_infra::repository::{
    UserRepository,
    WorkflowInstanceRepository,
    WorkflowStepRepository,
};

use crate::error::CoreError;

/// タスク一覧の要素: ステップ + ワークフロー概要
#[derive(Debug, PartialEq, Eq)]
pub struct TaskItem {
    pub step:     WorkflowStep,
    pub workflow: WorkflowInstance,
}

/// タスク詳細: ステップ + ワークフロー（全ステップ含む）
#[derive(Debug, PartialEq, Eq)]
pub struct TaskDetail {
    pub step:     WorkflowStep,
    pub workflow: WorkflowInstance,
    pub steps:    Vec<WorkflowStep>,
}

/// タスクユースケース実装
pub struct TaskUseCaseImpl {
    instance_repo: Arc<dyn WorkflowInstanceRepository>,
    step_repo:     Arc<dyn WorkflowStepRepository>,
    user_repo:     Arc<dyn UserRepository>,
}

impl TaskUseCaseImpl {
    pub fn new(
        instance_repo: Arc<dyn WorkflowInstanceRepository>,
        step_repo: Arc<dyn WorkflowStepRepository>,
        user_repo: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            instance_repo,
            step_repo,
            user_repo,
        }
    }

    /// ユーザー ID のリストからユーザー名を一括解決する
    pub async fn resolve_user_names(
        &self,
        user_ids: &[UserId],
    ) -> Result<HashMap<UserId, String>, CoreError> {
        crate::usecase::resolve_user_names(self.user_repo.as_ref(), user_ids).await
    }

    /// 自分のタスク一覧を取得する
    ///
    /// アサインされた Active なステップのみ返す。
    /// 各ステップに対応するワークフローインスタンスを一括取得し結合する。
    pub async fn list_my_tasks(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<Vec<TaskItem>, CoreError> {
        // 1. 担当者でステップを取得
        let steps = self
            .step_repo
            .find_by_assigned_to(&tenant_id, &user_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップ取得エラー: {}", e)))?;

        // 2. Active のみフィルタ
        let active_steps: Vec<WorkflowStep> = steps
            .into_iter()
            .filter(|s| s.status() == WorkflowStepStatus::Active)
            .collect();

        if active_steps.is_empty() {
            return Ok(Vec::new());
        }

        // 3. ワークフローインスタンスを一括取得
        let instance_ids: Vec<WorkflowInstanceId> = active_steps
            .iter()
            .map(|s| s.instance_id().clone())
            .collect();

        let instances = self
            .instance_repo
            .find_by_ids(&instance_ids, &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("インスタンス取得エラー: {}", e)))?;

        // 4. instance_id → WorkflowInstance のマップを構築
        let instance_map: HashMap<String, WorkflowInstance> = instances
            .into_iter()
            .map(|i| (i.id().to_string(), i))
            .collect();

        // 5. ステップ + インスタンスを結合
        let tasks = active_steps
            .into_iter()
            .filter_map(|step| {
                let instance_id_str = step.instance_id().to_string();
                instance_map.get(&instance_id_str).map(|workflow| TaskItem {
                    step,
                    workflow: workflow.clone(),
                })
            })
            .collect();

        Ok(tasks)
    }

    /// タスク詳細を取得する
    ///
    /// 指定されたステップ ID のタスクを取得し、権限チェックを行う。
    /// ワークフローの全ステップも含めて返す。
    pub async fn get_task(
        &self,
        step_id: WorkflowStepId,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<TaskDetail, CoreError> {
        // 1. ステップを取得
        let step = self
            .step_repo
            .find_by_id(&step_id, &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップ取得エラー: {}", e)))?
            .ok_or_else(|| CoreError::NotFound("タスクが見つかりません".to_string()))?;

        // 2. 権限チェック: 担当者のみアクセス可能
        if step.assigned_to() != Some(&user_id) {
            return Err(CoreError::Forbidden(
                "このタスクにアクセスする権限がありません".to_string(),
            ));
        }

        // 3. ワークフローインスタンスを取得
        let workflow = self
            .instance_repo
            .find_by_id(step.instance_id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("インスタンス取得エラー: {}", e)))?
            .ok_or_else(|| {
                CoreError::Internal("ステップに対応するワークフローが見つかりません".to_string())
            })?;

        // 4. ワークフローの全ステップを取得
        let steps = self
            .step_repo
            .find_by_instance(step.instance_id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップ一覧取得エラー: {}", e)))?;

        Ok(TaskDetail {
            step,
            workflow,
            steps,
        })
    }

    /// display_number でタスク詳細を取得する
    ///
    /// ワークフローの display_number とステップの display_number を指定して
    /// タスク詳細を取得する。権限チェックを行い、担当者のみアクセス可能。
    pub async fn get_task_by_display_numbers(
        &self,
        workflow_display_number: DisplayNumber,
        step_display_number: DisplayNumber,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<TaskDetail, CoreError> {
        // 1. ワークフローインスタンスを display_number で取得
        let workflow = self
            .instance_repo
            .find_by_display_number(workflow_display_number, &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("インスタンス取得エラー: {}", e)))?
            .ok_or_else(|| CoreError::NotFound("ワークフローが見つかりません".to_string()))?;

        // 2. ステップを display_number で取得
        let step = self
            .step_repo
            .find_by_display_number(step_display_number, workflow.id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップ取得エラー: {}", e)))?
            .ok_or_else(|| CoreError::NotFound("タスクが見つかりません".to_string()))?;

        // 3. 権限チェック: 担当者のみアクセス可能
        if step.assigned_to() != Some(&user_id) {
            return Err(CoreError::Forbidden(
                "このタスクにアクセスする権限がありません".to_string(),
            ));
        }

        // 4. ワークフローの全ステップを取得
        let steps = self
            .step_repo
            .find_by_instance(workflow.id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップ一覧取得エラー: {}", e)))?;

        Ok(TaskDetail {
            step,
            workflow,
            steps,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ringiflow_domain::{
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
    use ringiflow_infra::mock::{
        MockUserRepository,
        MockWorkflowInstanceRepository,
        MockWorkflowStepRepository,
    };

    use super::*;

    #[tokio::test]
    async fn test_list_my_tasks_activeなステップのみ返る() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        // ワークフローインスタンスを作成
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
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

        // Active ステップ
        let active_step = WorkflowStep::new(NewWorkflowStep {
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
        step_repo.insert(&active_step, &tenant_id).await.unwrap();

        // Pending ステップ（同じ approver）
        let pending_step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(2).unwrap(),
            step_id: "review".to_string(),
            step_name: "レビュー".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now,
        });
        step_repo.insert(&pending_step, &tenant_id).await.unwrap();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act
        let result = sut.list_my_tasks(tenant_id, approver_id).await;

        // Assert
        let expected = vec![TaskItem {
            step:     active_step,
            workflow: instance,
        }];
        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn test_list_my_tasks_workflowタイトルがタスクに含まれる() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let now = chrono::Utc::now();
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(101).unwrap(),
            title: "経費精算申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

        let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "approval".to_string(),
            step_name: "部長承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now,
        })
        .activated(now);
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act
        let result = sut.list_my_tasks(tenant_id, approver_id).await;

        // Assert
        let expected = vec![TaskItem {
            step,
            workflow: instance,
        }];
        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn test_list_my_tasks_他ユーザーのタスクは返らない() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let other_user_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

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
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

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
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act: 別のユーザーで取得
        let result = sut.list_my_tasks(tenant_id, other_user_id).await;

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_my_tasks_タスクがない場合は空vec() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act
        let result = sut.list_my_tasks(tenant_id, user_id).await;

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_task_正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let now = chrono::Utc::now();
        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(100).unwrap(),
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({"note": "test"}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

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
        let step_id = step.id().clone();
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act
        let result = sut.get_task(step_id, tenant_id, approver_id).await;

        // Assert
        let expected = TaskDetail {
            step:     step.clone(),
            workflow: instance,
            steps:    vec![step],
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn test_get_task_stepが見つからない場合はnotfound() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act: 存在しない step_id で取得
        let result = sut
            .get_task(WorkflowStepId::new(), tenant_id, user_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_task_他ユーザーのタスクはforbidden() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let other_user_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

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
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

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
        let step_id = step.id().clone();
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act: 別のユーザーで取得
        let result = sut.get_task(step_id, tenant_id, other_user_id).await;

        // Assert
        assert!(matches!(result, Err(CoreError::Forbidden(_))));
    }

    // ===== get_task_by_display_numbers のテスト =====

    #[tokio::test]
    async fn test_get_task_by_display_numbers_正常系() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let now = chrono::Utc::now();
        let workflow_dn = DisplayNumber::new(42).unwrap();
        let step_dn = DisplayNumber::new(1).unwrap();

        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: workflow_dn,
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({"note": "test"}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

        let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: step_dn,
            step_id: "approval".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now,
        })
        .activated(now);
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act
        let result = sut
            .get_task_by_display_numbers(workflow_dn, step_dn, tenant_id, approver_id)
            .await;

        // Assert
        let expected = TaskDetail {
            step:     step.clone(),
            workflow: instance,
            steps:    vec![step],
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn test_get_task_by_display_numbers_ワークフローが見つからない() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act
        let result = sut
            .get_task_by_display_numbers(
                DisplayNumber::new(999).unwrap(),
                DisplayNumber::new(1).unwrap(),
                tenant_id,
                user_id,
            )
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_task_by_display_numbers_ステップが見つからない() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let now = chrono::Utc::now();
        let workflow_dn = DisplayNumber::new(42).unwrap();

        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: workflow_dn,
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        });
        instance_repo.insert(&instance).await.unwrap();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act
        let result = sut
            .get_task_by_display_numbers(
                workflow_dn,
                DisplayNumber::new(999).unwrap(),
                tenant_id,
                user_id,
            )
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_task_by_display_numbers_権限がない() {
        // Arrange
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let approver_id = UserId::new();
        let other_user_id = UserId::new();

        let instance_repo = MockWorkflowInstanceRepository::new();
        let step_repo = MockWorkflowStepRepository::new();

        let now = chrono::Utc::now();
        let workflow_dn = DisplayNumber::new(42).unwrap();
        let step_dn = DisplayNumber::new(1).unwrap();

        let instance = WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: workflow_dn,
            title: "テスト申請".to_string(),
            form_data: serde_json::json!({}),
            initiated_by: user_id.clone(),
            now,
        })
        .submitted(now)
        .unwrap()
        .with_current_step("approval".to_string(), now);
        instance_repo.insert(&instance).await.unwrap();

        let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: step_dn,
            step_id: "approval".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver_id.clone()),
            now,
        })
        .activated(now);
        step_repo.insert(&step, &tenant_id).await.unwrap();

        let sut = TaskUseCaseImpl::new(
            Arc::new(instance_repo),
            Arc::new(step_repo),
            Arc::new(MockUserRepository),
        );

        // Act
        let result = sut
            .get_task_by_display_numbers(workflow_dn, step_dn, tenant_id, other_user_id)
            .await;

        // Assert
        assert!(matches!(result, Err(CoreError::Forbidden(_))));
    }
}
