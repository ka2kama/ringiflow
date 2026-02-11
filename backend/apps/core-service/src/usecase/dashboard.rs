//! # ダッシュボードユースケース
//!
//! ダッシュボード統計情報（KPI）の取得に関するビジネスロジックを実装する。
//!
//! ## 統計項目
//!
//! - 承認待ちタスク数: 自分にアサインされた Active なステップ数
//! - 申請中ワークフロー数: 自分が申請した InProgress なインスタンス数
//! - 本日完了タスク数: 自分にアサインされた本日 completed_at のステップ数

use std::sync::Arc;

use chrono::{DateTime, Utc};
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   workflow::{WorkflowInstanceStatus, WorkflowStepStatus},
};
use ringiflow_infra::repository::{WorkflowInstanceRepository, WorkflowStepRepository};
use serde::Serialize;

use crate::error::CoreError;

/// ダッシュボード統計情報
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DashboardStats {
   pub pending_tasks: i64,
   pub my_workflows_in_progress: i64,
   pub completed_today: i64,
}

/// ダッシュボードユースケース実装
pub struct DashboardUseCaseImpl {
   instance_repo: Arc<dyn WorkflowInstanceRepository>,
   step_repo:     Arc<dyn WorkflowStepRepository>,
}

impl DashboardUseCaseImpl {
   pub fn new(
      instance_repo: Arc<dyn WorkflowInstanceRepository>,
      step_repo: Arc<dyn WorkflowStepRepository>,
   ) -> Self {
      Self {
         instance_repo,
         step_repo,
      }
   }

   /// ダッシュボード統計情報を取得する
   ///
   /// 現在時刻を引数として受け取ることで、テスタビリティを確保する。
   pub async fn get_stats(
      &self,
      tenant_id: TenantId,
      user_id: UserId,
      now: DateTime<Utc>,
   ) -> Result<DashboardStats, CoreError> {
      // 1. 承認待ちタスク数: 自分にアサインされた Active なステップ
      let my_steps = self
         .step_repo
         .find_by_assigned_to(&tenant_id, &user_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップ取得エラー: {}", e)))?;

      let pending_tasks = my_steps
         .iter()
         .filter(|s| s.status() == WorkflowStepStatus::Active)
         .count() as i64;

      // 2. 申請中ワークフロー数: 自分が申請した InProgress なインスタンス
      let my_instances = self
         .instance_repo
         .find_by_initiated_by(&tenant_id, &user_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンス取得エラー: {}", e)))?;

      let my_workflows_in_progress = my_instances
         .iter()
         .filter(|i| i.status() == WorkflowInstanceStatus::InProgress)
         .count() as i64;

      // 3. 本日完了タスク数: 自分にアサインされた本日 completed_at のステップ
      let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
      let today_start_utc = today_start.and_utc();

      let completed_today = my_steps
         .iter()
         .filter(|s| {
            s.status() == WorkflowStepStatus::Completed
               && s
                  .completed_at()
                  .is_some_and(|completed| completed >= today_start_utc)
         })
         .count() as i64;

      Ok(DashboardStats {
         pending_tasks,
         my_workflows_in_progress,
         completed_today,
      })
   }
}

#[cfg(test)]
mod tests {
   use std::sync::Arc;

   use chrono::Utc;
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
   use ringiflow_infra::mock::{MockWorkflowInstanceRepository, MockWorkflowStepRepository};

   use super::*;

   #[tokio::test]
   async fn test_承認待ちタスク数がactiveステップのみカウントされる() {
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let now = Utc::now();
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

      // Active ステップ（カウント対象）
      let active_step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: Utc::now(),
      })
      .activated(Utc::now());
      step_repo.insert(&active_step, &tenant_id).await.unwrap();

      // Pending ステップ（カウント対象外）
      let pending_step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(2).unwrap(),
         step_id: "review".to_string(),
         step_name: "レビュー".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: Utc::now(),
      });
      step_repo.insert(&pending_step, &tenant_id).await.unwrap();

      let sut = DashboardUseCaseImpl::new(Arc::new(instance_repo), Arc::new(step_repo));
      let stats = sut
         .get_stats(tenant_id, approver_id, Utc::now())
         .await
         .unwrap();

      assert_eq!(stats.pending_tasks, 1);
   }

   #[tokio::test]
   async fn test_申請中ワークフロー数がinprogressのみカウントされる() {
      let tenant_id = TenantId::new();
      let user_id = UserId::new();

      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let now = Utc::now();

      // InProgress インスタンス（カウント対象）
      let in_progress = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "申請中1".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&in_progress).await.unwrap();

      // Draft インスタンス（カウント対象外）
      let draft = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(101).unwrap(),
         title: "下書き".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      });
      instance_repo.insert(&draft).await.unwrap();

      // Approved インスタンス（カウント対象外）
      let approved = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(102).unwrap(),
         title: "承認済み".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .approved(now);
      instance_repo.insert(&approved).await.unwrap();

      let sut = DashboardUseCaseImpl::new(Arc::new(instance_repo), Arc::new(step_repo));
      let stats = sut.get_stats(tenant_id, user_id, Utc::now()).await.unwrap();

      assert_eq!(stats.my_workflows_in_progress, 1);
   }

   #[tokio::test]
   async fn test_本日完了タスク数が今日のcompleted_atのみカウントされる() {
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let now = Utc::now();
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

      // 完了済みステップ（今日 → カウント対象）
      let completed_step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: Utc::now(),
      })
      .activated(Utc::now())
      .approve(Some("OK".to_string()), Utc::now())
      .unwrap();
      step_repo.insert(&completed_step, &tenant_id).await.unwrap();

      let sut = DashboardUseCaseImpl::new(Arc::new(instance_repo), Arc::new(step_repo));
      let now = Utc::now();
      let stats = sut.get_stats(tenant_id, approver_id, now).await.unwrap();

      assert_eq!(stats.completed_today, 1);
   }

   #[tokio::test]
   async fn test_タスクがない場合はすべて0を返す() {
      let tenant_id = TenantId::new();
      let user_id = UserId::new();

      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let sut = DashboardUseCaseImpl::new(Arc::new(instance_repo), Arc::new(step_repo));
      let stats = sut.get_stats(tenant_id, user_id, Utc::now()).await.unwrap();

      let expected = DashboardStats {
         pending_tasks: 0,
         my_workflows_in_progress: 0,
         completed_today: 0,
      };
      assert_eq!(stats, expected);
   }

   #[tokio::test]
   async fn test_他ユーザーのデータは含まれない() {
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let other_user_id = UserId::new();

      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let now = Utc::now();

      // user_id が申請した InProgress インスタンス
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "ユーザーAの申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      // approver_id にアサインされた Active ステップ
      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: Utc::now(),
      })
      .activated(Utc::now());
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = DashboardUseCaseImpl::new(Arc::new(instance_repo), Arc::new(step_repo));

      // other_user_id で統計を取得 → すべて 0
      let stats = sut
         .get_stats(tenant_id, other_user_id, Utc::now())
         .await
         .unwrap();

      assert_eq!(stats.pending_tasks, 0);
      assert_eq!(stats.my_workflows_in_progress, 0);
      assert_eq!(stats.completed_today, 0);
   }
}
