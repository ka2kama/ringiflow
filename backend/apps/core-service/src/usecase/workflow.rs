//! # ワークフローユースケース
//!
//! ワークフローの作成・取得・申請に関するビジネスロジックを実装する。

mod command;
mod query;

use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;
use ringiflow_domain::{
   clock::Clock,
   user::UserId,
   value_objects::Version,
   workflow::{WorkflowDefinitionId, WorkflowInstance, WorkflowStep},
};
use ringiflow_infra::repository::{
   DisplayIdCounterRepository,
   UserRepository,
   WorkflowDefinitionRepository,
   WorkflowInstanceRepository,
   WorkflowStepRepository,
};
use serde_json::Value as JsonValue;

use crate::error::CoreError;

/// ユースケースの出力: ワークフローインスタンスとステップの集約
///
/// ドメインモデル (`WorkflowInstance`, `WorkflowStep`) を変更せず、
/// ユースケースの出力として集約する。詳細取得や承認/却下の結果を
/// ハンドラに返す際に使用する。
#[derive(Debug, PartialEq, Eq)]
pub struct WorkflowWithSteps {
   pub instance: WorkflowInstance,
   pub steps:    Vec<WorkflowStep>,
}

/// ワークフロー作成入力
#[derive(Debug, Clone)]
pub struct CreateWorkflowInput {
   /// ワークフロー定義 ID
   pub definition_id: WorkflowDefinitionId,
   /// ワークフロータイトル
   pub title:         String,
   /// フォームデータ
   pub form_data:     JsonValue,
}

/// ワークフロー申請入力
#[derive(Debug, Clone)]
pub struct SubmitWorkflowInput {
   /// 承認者のユーザー ID
   pub assigned_to: UserId,
}

/// ステップ承認/却下入力
#[derive(Debug, Clone)]
pub struct ApproveRejectInput {
   /// 楽観的ロック用バージョン
   pub version: Version,
   /// コメント（任意）
   pub comment: Option<String>,
}

/// WorkflowInstance + Steps からユーザー ID を収集する
///
/// ワークフローの initiated_by と各ステップの assigned_to を
/// 重複排除して返す。ユーザー名一括解決の前処理として使用する。
pub(crate) fn collect_user_ids_from_workflow(
   instance: &WorkflowInstance,
   steps: &[WorkflowStep],
) -> Vec<UserId> {
   std::iter::once(instance.initiated_by().clone())
      .chain(steps.iter().filter_map(|s| s.assigned_to().cloned()))
      .unique()
      .collect()
}

/// ワークフローユースケース実装
///
/// ワークフローの作成・申請に関するビジネスロジックを実装する。
pub struct WorkflowUseCaseImpl {
   definition_repo: Arc<dyn WorkflowDefinitionRepository>,
   instance_repo:   Arc<dyn WorkflowInstanceRepository>,
   step_repo:       Arc<dyn WorkflowStepRepository>,
   user_repo:       Arc<dyn UserRepository>,
   counter_repo:    Arc<dyn DisplayIdCounterRepository>,
   clock:           Arc<dyn Clock>,
}

impl WorkflowUseCaseImpl {
   /// 新しいワークフローユースケースを作成
   pub fn new(
      definition_repo: Arc<dyn WorkflowDefinitionRepository>,
      instance_repo: Arc<dyn WorkflowInstanceRepository>,
      step_repo: Arc<dyn WorkflowStepRepository>,
      user_repo: Arc<dyn UserRepository>,
      counter_repo: Arc<dyn DisplayIdCounterRepository>,
      clock: Arc<dyn Clock>,
   ) -> Self {
      Self {
         definition_repo,
         instance_repo,
         step_repo,
         user_repo,
         counter_repo,
         clock,
      }
   }

   /// ユーザー ID のリストからユーザー名を一括解決する
   pub async fn resolve_user_names(
      &self,
      user_ids: &[UserId],
   ) -> Result<HashMap<UserId, String>, CoreError> {
      crate::usecase::resolve_user_names(self.user_repo.as_ref(), user_ids).await
   }
}
