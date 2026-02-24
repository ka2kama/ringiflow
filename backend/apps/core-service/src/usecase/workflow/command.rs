//! ワークフローユースケースの状態変更操作

mod comment;
mod decision;
mod helpers;
mod lifecycle;

#[cfg(test)]
pub(super) mod test_helpers {
    use std::sync::Arc;

    use ringiflow_domain::{
        clock::FixedClock,
        tenant::TenantId,
        user::UserId,
        value_objects::{DisplayNumber, Version, WorkflowName},
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
    use ringiflow_infra::mock::{
        MockDisplayIdCounterRepository,
        MockTransactionManager,
        MockUserRepository,
        MockWorkflowCommentRepository,
        MockWorkflowDefinitionRepository,
        MockWorkflowInstanceRepository,
        MockWorkflowStepRepository,
    };

    use crate::usecase::workflow::WorkflowUseCaseImpl;

    /// SUT（WorkflowUseCaseImpl）を構築する
    ///
    /// テストで繰り返される 8 引数の構築ボイラープレートを共通化する。
    /// Mock repos は参照で受け取り、内部で clone する（共有ステートが保持される）。
    pub fn build_sut(
        definition_repo: &MockWorkflowDefinitionRepository,
        instance_repo: &MockWorkflowInstanceRepository,
        step_repo: &MockWorkflowStepRepository,
        now: chrono::DateTime<chrono::Utc>,
    ) -> WorkflowUseCaseImpl {
        WorkflowUseCaseImpl::new(
            Arc::new(definition_repo.clone()),
            Arc::new(instance_repo.clone()),
            Arc::new(step_repo.clone()),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(now)),
            Arc::new(MockTransactionManager),
        )
    }

    /// テスト用の1段階承認定義 JSON
    pub fn single_approval_definition_json() -> serde_json::Value {
        serde_json::json!({
           "steps": [
              {"id": "start", "type": "start", "name": "開始"},
              {"id": "approval", "type": "approval", "name": "承認"},
              {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
              {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
           ]
        })
    }

    /// テスト用の2段階承認定義 JSON
    pub fn two_step_approval_definition_json() -> serde_json::Value {
        serde_json::json!({
           "steps": [
              {"id": "start", "type": "start", "name": "開始"},
              {"id": "manager_approval", "type": "approval", "name": "上長承認"},
              {"id": "finance_approval", "type": "approval", "name": "経理承認"},
              {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
              {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
           ]
        })
    }

    /// 2段階承認用テストヘルパー: 定義・インスタンス・2ステップを作成
    ///
    /// 戻り値: (definition, instance, step1(Active), step2(Pending))
    pub fn setup_two_step_approval(
        tenant_id: &TenantId,
        user_id: &UserId,
        approver1_id: &UserId,
        approver2_id: &UserId,
        now: chrono::DateTime<chrono::Utc>,
    ) -> (
        WorkflowDefinition,
        WorkflowInstance,
        WorkflowStep,
        WorkflowStep,
    ) {
        let definition = WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id: tenant_id.clone(),
            name: WorkflowName::new("2段階承認").unwrap(),
            description: Some("テスト用定義".to_string()),
            definition: two_step_approval_definition_json(),
            created_by: user_id.clone(),
            now,
        })
        .published(now)
        .unwrap();

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
        .with_current_step("manager_approval".to_string(), now)
        .unwrap();

        let step1 = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "manager_approval".to_string(),
            step_name: "上長承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver1_id.clone()),
            now,
        })
        .activated(now);

        let step2 = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance.id().clone(),
            display_number: DisplayNumber::new(2).unwrap(),
            step_id: "finance_approval".to_string(),
            step_name: "経理承認".to_string(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver2_id.clone()),
            now,
        });

        (definition, instance, step1, step2)
    }
}
