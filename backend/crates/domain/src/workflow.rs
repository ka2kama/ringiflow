//! # ワークフロー
//!
//! ワークフロー定義、インスタンス、ステップを管理する。
//!
//! ## 概念モデル
//!
//! - **WorkflowDefinition**: ワークフローのテンプレート（再利用可能）
//! - **WorkflowInstance**: 定義から生成された実行中の案件
//! - **WorkflowStep**: インスタンス内の各承認ステップ
//!
//! ## 使用例
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ringiflow_domain::workflow::{
//!     NewWorkflowDefinition, WorkflowDefinition, WorkflowDefinitionId,
//!     WorkflowDefinitionStatus,
//! };
//! use ringiflow_domain::{tenant::TenantId, user::UserId, value_objects::WorkflowName};
//! use serde_json::json;
//!
//! // ワークフロー定義の作成
//! let definition = WorkflowDefinition::new(NewWorkflowDefinition {
//!     id: WorkflowDefinitionId::new(),
//!     tenant_id: TenantId::new(),
//!     name: WorkflowName::new("汎用申請")?,
//!     description: Some("シンプルな1段階承認".to_string()),
//!     definition: json!({"steps": []}),
//!     created_by: UserId::new(),
//!     now: chrono::Utc::now(),
//! });
//! assert_eq!(definition.status(), WorkflowDefinitionStatus::Draft);
//! # Ok(())
//! # }
//! ```

mod definition;
mod instance;

use chrono::{DateTime, Utc};
pub use definition::*;
use derive_more::Display;
pub use instance::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use strum::IntoStaticStr;
use uuid::Uuid;

use crate::{
   DomainError,
   user::UserId,
   value_objects::{DisplayNumber, Version},
};

// =========================================================================
// Workflow Step（ワークフローステップ）
// =========================================================================

/// ワークフローステップ ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
pub struct WorkflowStepId(Uuid);

impl WorkflowStepId {
   pub fn new() -> Self {
      Self(Uuid::now_v7())
   }

   pub fn from_uuid(uuid: Uuid) -> Self {
      Self(uuid)
   }

   pub fn as_uuid(&self) -> &Uuid {
      &self.0
   }
}

impl Default for WorkflowStepId {
   fn default() -> Self {
      Self::new()
   }
}

/// ワークフローステップステータス
#[derive(
   Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr, strum::Display,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WorkflowStepStatus {
   /// 待機中
   Pending,
   /// アクティブ（処理中）
   Active,
   /// 完了
   Completed,
   /// スキップ
   Skipped,
}

impl std::str::FromStr for WorkflowStepStatus {
   type Err = DomainError;

   fn from_str(s: &str) -> Result<Self, Self::Err> {
      match s {
         "pending" => Ok(Self::Pending),
         "active" => Ok(Self::Active),
         "completed" => Ok(Self::Completed),
         "skipped" => Ok(Self::Skipped),
         _ => Err(DomainError::Validation(format!(
            "不正なワークフローステップステータス: {}",
            s
         ))),
      }
   }
}

/// ワークフローステップの判断
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "snake_case")]
pub enum StepDecision {
   /// 承認
   Approved,
   /// 却下
   Rejected,
   /// 修正依頼
   RequestChanges,
}

impl std::str::FromStr for StepDecision {
   type Err = DomainError;

   fn from_str(s: &str) -> Result<Self, Self::Err> {
      match s {
         "approved" => Ok(Self::Approved),
         "rejected" => Ok(Self::Rejected),
         "request_changes" => Ok(Self::RequestChanges),
         _ => Err(DomainError::Validation(format!(
            "不正なステップ判断: {}",
            s
         ))),
      }
   }
}

/// ワークフローステップエンティティ
///
/// ワークフローインスタンス内の個々の承認タスク。
/// 担当者への割り当てと判断結果を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStep {
   id: WorkflowStepId,
   instance_id: WorkflowInstanceId,
   display_number: DisplayNumber,
   step_id: String,
   step_name: String,
   step_type: String,
   status: WorkflowStepStatus,
   version: Version,
   assigned_to: Option<UserId>,
   decision: Option<StepDecision>,
   comment: Option<String>,
   due_date: Option<DateTime<Utc>>,
   started_at: Option<DateTime<Utc>>,
   completed_at: Option<DateTime<Utc>>,
   created_at: DateTime<Utc>,
   updated_at: DateTime<Utc>,
}

/// ワークフローステップの新規作成パラメータ
pub struct NewWorkflowStep {
   pub id: WorkflowStepId,
   pub instance_id: WorkflowInstanceId,
   pub display_number: DisplayNumber,
   pub step_id: String,
   pub step_name: String,
   pub step_type: String,
   pub assigned_to: Option<UserId>,
   pub now: DateTime<Utc>,
}

/// ワークフローステップの DB 復元パラメータ
pub struct WorkflowStepRecord {
   pub id: WorkflowStepId,
   pub instance_id: WorkflowInstanceId,
   pub display_number: DisplayNumber,
   pub step_id: String,
   pub step_name: String,
   pub step_type: String,
   pub status: WorkflowStepStatus,
   pub version: Version,
   pub assigned_to: Option<UserId>,
   pub decision: Option<StepDecision>,
   pub comment: Option<String>,
   pub due_date: Option<DateTime<Utc>>,
   pub started_at: Option<DateTime<Utc>>,
   pub completed_at: Option<DateTime<Utc>>,
   pub created_at: DateTime<Utc>,
   pub updated_at: DateTime<Utc>,
}

impl WorkflowStep {
   /// 新しいワークフローステップを作成する
   pub fn new(params: NewWorkflowStep) -> Self {
      Self {
         id: params.id,
         instance_id: params.instance_id,
         display_number: params.display_number,
         step_id: params.step_id,
         step_name: params.step_name,
         step_type: params.step_type,
         status: WorkflowStepStatus::Pending,
         version: Version::initial(),
         assigned_to: params.assigned_to,
         decision: None,
         comment: None,
         due_date: None,
         started_at: None,
         completed_at: None,
         created_at: params.now,
         updated_at: params.now,
      }
   }

   /// 既存のデータから復元する
   pub fn from_db(record: WorkflowStepRecord) -> Self {
      Self {
         id: record.id,
         instance_id: record.instance_id,
         display_number: record.display_number,
         step_id: record.step_id,
         step_name: record.step_name,
         step_type: record.step_type,
         status: record.status,
         version: record.version,
         assigned_to: record.assigned_to,
         decision: record.decision,
         comment: record.comment,
         due_date: record.due_date,
         started_at: record.started_at,
         completed_at: record.completed_at,
         created_at: record.created_at,
         updated_at: record.updated_at,
      }
   }

   // Getter メソッド

   pub fn id(&self) -> &WorkflowStepId {
      &self.id
   }

   pub fn instance_id(&self) -> &WorkflowInstanceId {
      &self.instance_id
   }

   pub fn display_number(&self) -> DisplayNumber {
      self.display_number
   }

   pub fn step_id(&self) -> &str {
      &self.step_id
   }

   pub fn step_name(&self) -> &str {
      &self.step_name
   }

   pub fn step_type(&self) -> &str {
      &self.step_type
   }

   pub fn status(&self) -> WorkflowStepStatus {
      self.status
   }

   pub fn version(&self) -> Version {
      self.version
   }

   pub fn assigned_to(&self) -> Option<&UserId> {
      self.assigned_to.as_ref()
   }

   pub fn decision(&self) -> Option<StepDecision> {
      self.decision
   }

   pub fn comment(&self) -> Option<&str> {
      self.comment.as_deref()
   }

   pub fn due_date(&self) -> Option<DateTime<Utc>> {
      self.due_date
   }

   pub fn started_at(&self) -> Option<DateTime<Utc>> {
      self.started_at
   }

   pub fn completed_at(&self) -> Option<DateTime<Utc>> {
      self.completed_at
   }

   pub fn created_at(&self) -> DateTime<Utc> {
      self.created_at
   }

   pub fn updated_at(&self) -> DateTime<Utc> {
      self.updated_at
   }

   // ビジネスロジックメソッド

   /// ステップをアクティブにした新しいインスタンスを返す
   pub fn activated(self, now: DateTime<Utc>) -> Self {
      Self {
         status: WorkflowStepStatus::Active,
         started_at: Some(now),
         updated_at: now,
         ..self
      }
   }

   /// ステップを完了した新しいインスタンスを返す
   pub fn completed(
      self,
      decision: StepDecision,
      comment: Option<String>,
      now: DateTime<Utc>,
   ) -> Result<Self, DomainError> {
      if self.status != WorkflowStepStatus::Active {
         return Err(DomainError::Validation(
            "アクティブ状態でのみ完了できます".to_string(),
         ));
      }

      Ok(Self {
         status: WorkflowStepStatus::Completed,
         decision: Some(decision),
         comment,
         completed_at: Some(now),
         updated_at: now,
         ..self
      })
   }

   /// ステップをスキップした新しいインスタンスを返す
   pub fn skipped(self, now: DateTime<Utc>) -> Self {
      Self {
         status: WorkflowStepStatus::Skipped,
         updated_at: now,
         ..self
      }
   }

   /// ステップを承認する
   ///
   /// Active 状態のステップを Completed (Approved) に遷移させる。
   /// version をインクリメントして楽観的ロックに対応。
   ///
   /// # Errors
   ///
   /// - `DomainError::Validation`: Active 以外の状態で呼び出した場合
   pub fn approve(self, comment: Option<String>, now: DateTime<Utc>) -> Result<Self, DomainError> {
      if self.status != WorkflowStepStatus::Active {
         return Err(DomainError::Validation(format!(
            "承認はアクティブ状態でのみ可能です（現在: {}）",
            self.status
         )));
      }

      Ok(Self {
         status: WorkflowStepStatus::Completed,
         version: self.version.next(),
         decision: Some(StepDecision::Approved),
         comment,
         completed_at: Some(now),
         updated_at: now,
         ..self
      })
   }

   /// ステップを却下する
   ///
   /// Active 状態のステップを Completed (Rejected) に遷移させる。
   /// version をインクリメントして楽観的ロックに対応。
   ///
   /// # Errors
   ///
   /// - `DomainError::Validation`: Active 以外の状態で呼び出した場合
   pub fn reject(self, comment: Option<String>, now: DateTime<Utc>) -> Result<Self, DomainError> {
      if self.status != WorkflowStepStatus::Active {
         return Err(DomainError::Validation(format!(
            "却下はアクティブ状態でのみ可能です（現在: {}）",
            self.status
         )));
      }

      Ok(Self {
         status: WorkflowStepStatus::Completed,
         version: self.version.next(),
         decision: Some(StepDecision::Rejected),
         comment,
         completed_at: Some(now),
         updated_at: now,
         ..self
      })
   }

   /// ステップが期限切れかチェックする
   pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
      if let Some(due) = self.due_date
         && self.completed_at.is_none()
      {
         return now > due;
      }
      false
   }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
   use rstest::{fixture, rstest};
   use serde_json::json;

   use super::*;

   /// テスト用の固定タイムスタンプ
   #[fixture]
   fn now() -> DateTime<Utc> {
      DateTime::from_timestamp(1_700_000_000, 0).unwrap()
   }

   #[fixture]
   fn test_step(now: DateTime<Utc>) -> WorkflowStep {
      WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: WorkflowInstanceId::new(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "step_1".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(UserId::new()),
         now,
      })
   }

   // =========================================================================
   // WorkflowStep のテスト
   // =========================================================================

   mod workflow_step {
      use pretty_assertions::assert_eq;

      use super::*;

      #[rstest]
      fn test_新規作成の初期状態(test_step: WorkflowStep, now: DateTime<Utc>) {
         let expected = WorkflowStep::from_db(WorkflowStepRecord {
            id: test_step.id().clone(),
            instance_id: test_step.instance_id().clone(),
            display_number: test_step.display_number(),
            step_id: test_step.step_id().to_string(),
            step_name: test_step.step_name().to_string(),
            step_type: test_step.step_type().to_string(),
            status: WorkflowStepStatus::Pending,
            version: Version::initial(),
            assigned_to: test_step.assigned_to().cloned(),
            decision: None,
            comment: None,
            due_date: None,
            started_at: None,
            completed_at: None,
            created_at: now,
            updated_at: now,
         });
         assert_eq!(test_step, expected);
      }

      #[rstest]
      fn test_アクティブ化後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
         let before = test_step.clone();
         let sut = test_step.activated(now);

         let expected = WorkflowStep::from_db(WorkflowStepRecord {
            id: before.id().clone(),
            instance_id: before.instance_id().clone(),
            display_number: before.display_number(),
            step_id: before.step_id().to_string(),
            step_name: before.step_name().to_string(),
            step_type: before.step_type().to_string(),
            status: WorkflowStepStatus::Active,
            version: before.version(),
            assigned_to: before.assigned_to().cloned(),
            decision: None,
            comment: None,
            due_date: None,
            started_at: Some(now),
            completed_at: None,
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_承認後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
         let step = test_step.activated(now);
         let before = step.clone();

         let sut = step.approve(None, now).unwrap();

         let expected = WorkflowStep::from_db(WorkflowStepRecord {
            id: before.id().clone(),
            instance_id: before.instance_id().clone(),
            display_number: before.display_number(),
            step_id: before.step_id().to_string(),
            step_name: before.step_name().to_string(),
            step_type: before.step_type().to_string(),
            status: WorkflowStepStatus::Completed,
            version: before.version().next(),
            assigned_to: before.assigned_to().cloned(),
            decision: Some(StepDecision::Approved),
            comment: None,
            due_date: None,
            started_at: before.started_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_コメント付き承認後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
         let step = test_step.activated(now);
         let before = step.clone();

         let sut = step.approve(Some("承認します".to_string()), now).unwrap();

         let expected = WorkflowStep::from_db(WorkflowStepRecord {
            id: before.id().clone(),
            instance_id: before.instance_id().clone(),
            display_number: before.display_number(),
            step_id: before.step_id().to_string(),
            step_name: before.step_name().to_string(),
            step_type: before.step_type().to_string(),
            status: WorkflowStepStatus::Completed,
            version: before.version().next(),
            assigned_to: before.assigned_to().cloned(),
            decision: Some(StepDecision::Approved),
            comment: Some("承認します".to_string()),
            due_date: None,
            started_at: before.started_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_却下後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
         let step = test_step.activated(now);
         let before = step.clone();

         let sut = step.reject(None, now).unwrap();

         let expected = WorkflowStep::from_db(WorkflowStepRecord {
            id: before.id().clone(),
            instance_id: before.instance_id().clone(),
            display_number: before.display_number(),
            step_id: before.step_id().to_string(),
            step_name: before.step_name().to_string(),
            step_type: before.step_type().to_string(),
            status: WorkflowStepStatus::Completed,
            version: before.version().next(),
            assigned_to: before.assigned_to().cloned(),
            decision: Some(StepDecision::Rejected),
            comment: None,
            due_date: None,
            started_at: before.started_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_is_overdue_期限切れの場合trueを返す(now: DateTime<Utc>) {
         let past = DateTime::from_timestamp(1_699_999_000, 0).unwrap();
         let step = WorkflowStep::from_db(WorkflowStepRecord {
            id: WorkflowStepId::new(),
            instance_id: WorkflowInstanceId::new(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "step_1".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            status: WorkflowStepStatus::Active,
            version: Version::initial(),
            assigned_to: Some(UserId::new()),
            decision: None,
            comment: None,
            due_date: Some(past),
            started_at: Some(past),
            completed_at: None,
            created_at: past,
            updated_at: past,
         });
         assert!(step.is_overdue(now));
      }

      #[rstest]
      fn test_is_overdue_期限内の場合falseを返す(now: DateTime<Utc>) {
         let future = DateTime::from_timestamp(1_700_100_000, 0).unwrap();
         let step = WorkflowStep::from_db(WorkflowStepRecord {
            id: WorkflowStepId::new(),
            instance_id: WorkflowInstanceId::new(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "step_1".to_string(),
            step_name: "承認".to_string(),
            step_type: "approval".to_string(),
            status: WorkflowStepStatus::Active,
            version: Version::initial(),
            assigned_to: Some(UserId::new()),
            decision: None,
            comment: None,
            due_date: Some(future),
            started_at: Some(now),
            completed_at: None,
            created_at: now,
            updated_at: now,
         });
         assert!(!step.is_overdue(now));
      }

      #[rstest]
      fn test_アクティブ以外で承認するとエラー(
         test_step: WorkflowStep,
         now: DateTime<Utc>,
      ) {
         let result = test_step.approve(None, now);

         assert!(result.is_err());
      }

      #[rstest]
      fn test_アクティブ以外で却下するとエラー(
         test_step: WorkflowStep,
         now: DateTime<Utc>,
      ) {
         let result = test_step.reject(None, now);

         assert!(result.is_err());
      }

      #[rstest]
      fn test_差戻し後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
         let step = test_step.activated(now);
         let before = step.clone();

         let sut = step
            .completed(
               StepDecision::RequestChanges,
               Some("修正してください".to_string()),
               now,
            )
            .unwrap();

         let expected = WorkflowStep::from_db(WorkflowStepRecord {
            id: before.id().clone(),
            instance_id: before.instance_id().clone(),
            display_number: before.display_number(),
            step_id: before.step_id().to_string(),
            step_name: before.step_name().to_string(),
            step_type: before.step_type().to_string(),
            status: WorkflowStepStatus::Completed,
            version: before.version(),
            assigned_to: before.assigned_to().cloned(),
            decision: Some(StepDecision::RequestChanges),
            comment: Some("修正してください".to_string()),
            due_date: None,
            started_at: before.started_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }
   }
}
