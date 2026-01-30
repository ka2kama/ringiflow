//! # ユースケース層
//!
//! Core Service のビジネスロジックを実装する。
//!
//! ## 設計方針
//!
//! - **トレイトベースの設計**: テスト可能性のためトレイトを定義
//! - **依存性注入**: リポジトリを外部から注入
//! - **薄いハンドラ**: ハンドラは薄く保ち、ロジックはユースケースに集約
//!
//! ## モジュール構成
//!
//! - `workflow`: ワークフロー関連のユースケース

pub mod task;
pub mod workflow;

use async_trait::async_trait;
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   workflow::{WorkflowInstance, WorkflowInstanceId},
};
pub use task::TaskUseCaseImpl;
pub use workflow::{
   ApproveRejectInput,
   CreateWorkflowInput,
   SubmitWorkflowInput,
   WorkflowUseCaseImpl,
   WorkflowWithSteps,
};

use crate::error::CoreError;

/// ワークフローユースケーストレイト
///
/// Core Service のワークフロー関連ビジネスロジックを定義する。
/// 具体的な実装は `WorkflowUseCaseImpl` で提供される。
#[allow(dead_code)]
#[async_trait]
pub trait WorkflowUseCase: Send + Sync {
   /// ワークフローインスタンスを作成する（下書き）
   ///
   /// ## 引数
   ///
   /// - `input`: ワークフロー作成入力
   /// - `tenant_id`: テナント ID
   /// - `user_id`: 申請者のユーザー ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(WorkflowInstance)`: 作成されたワークフローインスタンス
   /// - `Err(CoreError)`: エラー
   async fn create_workflow(
      &self,
      input: CreateWorkflowInput,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<WorkflowInstance, CoreError>;

   /// ワークフローを申請する
   ///
   /// 下書き状態のワークフローを申請状態に遷移させ、
   /// ワークフロー定義に基づいてステップを作成する。
   ///
   /// ## 引数
   ///
   /// - `input`: ワークフロー申請入力
   /// - `instance_id`: ワークフローインスタンス ID
   /// - `tenant_id`: テナント ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(WorkflowInstance)`: 申請後のワークフローインスタンス
   /// - `Err(CoreError)`: エラー
   async fn submit_workflow(
      &self,
      input: SubmitWorkflowInput,
      instance_id: WorkflowInstanceId,
      tenant_id: TenantId,
   ) -> Result<WorkflowInstance, CoreError>;
}

/// WorkflowUseCaseImpl に WorkflowUseCase トレイトを実装
#[async_trait]
impl<D, I, S> WorkflowUseCase for WorkflowUseCaseImpl<D, I, S>
where
   D: ringiflow_infra::repository::WorkflowDefinitionRepository + Send + Sync,
   I: ringiflow_infra::repository::WorkflowInstanceRepository + Send + Sync,
   S: ringiflow_infra::repository::WorkflowStepRepository + Send + Sync,
{
   async fn create_workflow(
      &self,
      input: CreateWorkflowInput,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<WorkflowInstance, CoreError> {
      self.create_workflow(input, tenant_id, user_id).await
   }

   async fn submit_workflow(
      &self,
      input: SubmitWorkflowInput,
      instance_id: WorkflowInstanceId,
      tenant_id: TenantId,
   ) -> Result<WorkflowInstance, CoreError> {
      self.submit_workflow(input, instance_id, tenant_id).await
   }
}
