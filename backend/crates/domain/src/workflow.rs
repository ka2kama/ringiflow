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
//!     name: WorkflowName::new("汎用申請").unwrap(),
//!     description: Some("シンプルな1段階承認".to_string()),
//!     definition: json!({"steps": []}),
//!     created_by: UserId::new(),
//!     now: chrono::Utc::now(),
//! });
//! assert_eq!(definition.status(), WorkflowDefinitionStatus::Draft);
//! ```

use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use strum::IntoStaticStr;
use uuid::Uuid;

use crate::{
   DomainError,
   tenant::TenantId,
   user::UserId,
   value_objects::{DisplayNumber, Version, WorkflowName},
};

// =========================================================================
// Workflow Definition（ワークフロー定義）
// =========================================================================

/// ワークフロー定義 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
pub struct WorkflowDefinitionId(Uuid);

impl WorkflowDefinitionId {
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

impl Default for WorkflowDefinitionId {
   fn default() -> Self {
      Self::new()
   }
}

/// ワークフロー定義ステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WorkflowDefinitionStatus {
   /// 下書き（編集中）
   Draft,
   /// 公開済み（利用可能）
   Published,
   /// アーカイブ済み（非表示）
   Archived,
}

impl std::str::FromStr for WorkflowDefinitionStatus {
   type Err = DomainError;

   fn from_str(s: &str) -> Result<Self, Self::Err> {
      match s {
         "draft" => Ok(Self::Draft),
         "published" => Ok(Self::Published),
         "archived" => Ok(Self::Archived),
         _ => Err(DomainError::Validation(format!(
            "不正なワークフロー定義ステータス: {}",
            s
         ))),
      }
   }
}

/// ワークフロー定義エンティティ
///
/// 再利用可能なワークフローのテンプレート。
/// JSON 形式の定義を保持し、バージョン管理に対応。
#[derive(Debug, Clone)]
pub struct WorkflowDefinition {
   id:          WorkflowDefinitionId,
   tenant_id:   TenantId,
   name:        WorkflowName,
   description: Option<String>,
   version:     Version,
   definition:  JsonValue,
   status:      WorkflowDefinitionStatus,
   created_by:  UserId,
   created_at:  DateTime<Utc>,
   updated_at:  DateTime<Utc>,
}

/// ワークフロー定義の新規作成パラメータ
pub struct NewWorkflowDefinition {
   pub id:          WorkflowDefinitionId,
   pub tenant_id:   TenantId,
   pub name:        WorkflowName,
   pub description: Option<String>,
   pub definition:  JsonValue,
   pub created_by:  UserId,
   pub now:         DateTime<Utc>,
}

/// ワークフロー定義の DB 復元パラメータ
pub struct WorkflowDefinitionRecord {
   pub id:          WorkflowDefinitionId,
   pub tenant_id:   TenantId,
   pub name:        WorkflowName,
   pub description: Option<String>,
   pub version:     Version,
   pub definition:  JsonValue,
   pub status:      WorkflowDefinitionStatus,
   pub created_by:  UserId,
   pub created_at:  DateTime<Utc>,
   pub updated_at:  DateTime<Utc>,
}

impl WorkflowDefinition {
   /// 新しいワークフロー定義を作成する
   pub fn new(params: NewWorkflowDefinition) -> Self {
      Self {
         id:          params.id,
         tenant_id:   params.tenant_id,
         name:        params.name,
         description: params.description,
         version:     Version::initial(),
         definition:  params.definition,
         status:      WorkflowDefinitionStatus::Draft,
         created_by:  params.created_by,
         created_at:  params.now,
         updated_at:  params.now,
      }
   }

   /// 既存のデータから復元する
   pub fn from_db(record: WorkflowDefinitionRecord) -> Self {
      Self {
         id:          record.id,
         tenant_id:   record.tenant_id,
         name:        record.name,
         description: record.description,
         version:     record.version,
         definition:  record.definition,
         status:      record.status,
         created_by:  record.created_by,
         created_at:  record.created_at,
         updated_at:  record.updated_at,
      }
   }

   // Getter メソッド

   pub fn id(&self) -> &WorkflowDefinitionId {
      &self.id
   }

   pub fn tenant_id(&self) -> &TenantId {
      &self.tenant_id
   }

   pub fn name(&self) -> &WorkflowName {
      &self.name
   }

   pub fn description(&self) -> Option<&str> {
      self.description.as_deref()
   }

   pub fn version(&self) -> Version {
      self.version
   }

   pub fn definition(&self) -> &JsonValue {
      &self.definition
   }

   pub fn status(&self) -> WorkflowDefinitionStatus {
      self.status
   }

   pub fn created_by(&self) -> &UserId {
      &self.created_by
   }

   pub fn created_at(&self) -> DateTime<Utc> {
      self.created_at
   }

   pub fn updated_at(&self) -> DateTime<Utc> {
      self.updated_at
   }

   // ビジネスロジックメソッド

   /// 定義が公開可能かチェックする
   pub fn can_publish(&self) -> Result<(), DomainError> {
      if self.status == WorkflowDefinitionStatus::Published {
         return Err(DomainError::Validation("既に公開済みです".to_string()));
      }
      Ok(())
   }

   /// 定義を公開した新しいインスタンスを返す
   pub fn published(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
      self.can_publish()?;
      Ok(Self {
         status: WorkflowDefinitionStatus::Published,
         updated_at: now,
         ..self
      })
   }

   /// 定義をアーカイブした新しいインスタンスを返す
   pub fn archived(self, now: DateTime<Utc>) -> Self {
      Self {
         status: WorkflowDefinitionStatus::Archived,
         updated_at: now,
         ..self
      }
   }
}

// =========================================================================
// Workflow Instance（ワークフローインスタンス）
// =========================================================================

/// ワークフローインスタンス ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
pub struct WorkflowInstanceId(Uuid);

impl WorkflowInstanceId {
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

impl Default for WorkflowInstanceId {
   fn default() -> Self {
      Self::new()
   }
}

/// ワークフローインスタンスステータス
#[derive(
   Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr, strum::Display,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "snake_case")]
pub enum WorkflowInstanceStatus {
   /// 下書き
   Draft,
   /// 承認待ち
   Pending,
   /// 処理中
   InProgress,
   /// 承認完了
   Approved,
   /// 却下
   Rejected,
   /// 取り消し
   Cancelled,
}

impl std::str::FromStr for WorkflowInstanceStatus {
   type Err = DomainError;

   fn from_str(s: &str) -> Result<Self, Self::Err> {
      match s {
         "draft" => Ok(Self::Draft),
         "pending" => Ok(Self::Pending),
         "in_progress" => Ok(Self::InProgress),
         "approved" => Ok(Self::Approved),
         "rejected" => Ok(Self::Rejected),
         "cancelled" => Ok(Self::Cancelled),
         _ => Err(DomainError::Validation(format!(
            "不正なワークフローインスタンスステータス: {}",
            s
         ))),
      }
   }
}

/// ワークフローインスタンスエンティティ
///
/// 定義から生成された実行中の申請案件。
/// フォームデータと進捗状態を保持する。
///
/// ## 楽観的ロック
///
/// `version` フィールドにより、並行更新時の競合を検出する。
/// 更新操作時はリクエストの version と DB の version を比較し、
/// 一致しない場合は競合エラー（409 Conflict）を返す。
#[derive(Debug, Clone)]
pub struct WorkflowInstance {
   id: WorkflowInstanceId,
   tenant_id: TenantId,
   definition_id: WorkflowDefinitionId,
   definition_version: Version,
   display_number: DisplayNumber,
   title: String,
   form_data: JsonValue,
   status: WorkflowInstanceStatus,
   version: Version,
   current_step_id: Option<String>,
   initiated_by: UserId,
   submitted_at: Option<DateTime<Utc>>,
   completed_at: Option<DateTime<Utc>>,
   created_at: DateTime<Utc>,
   updated_at: DateTime<Utc>,
}

/// ワークフローインスタンスの新規作成パラメータ
pub struct NewWorkflowInstance {
   pub id: WorkflowInstanceId,
   pub tenant_id: TenantId,
   pub definition_id: WorkflowDefinitionId,
   pub definition_version: Version,
   pub display_number: DisplayNumber,
   pub title: String,
   pub form_data: JsonValue,
   pub initiated_by: UserId,
   pub now: DateTime<Utc>,
}

/// ワークフローインスタンスの DB 復元パラメータ
pub struct WorkflowInstanceRecord {
   pub id: WorkflowInstanceId,
   pub tenant_id: TenantId,
   pub definition_id: WorkflowDefinitionId,
   pub definition_version: Version,
   pub display_number: DisplayNumber,
   pub title: String,
   pub form_data: JsonValue,
   pub status: WorkflowInstanceStatus,
   pub version: Version,
   pub current_step_id: Option<String>,
   pub initiated_by: UserId,
   pub submitted_at: Option<DateTime<Utc>>,
   pub completed_at: Option<DateTime<Utc>>,
   pub created_at: DateTime<Utc>,
   pub updated_at: DateTime<Utc>,
}

impl WorkflowInstance {
   /// 新しいワークフローインスタンスを作成する
   pub fn new(params: NewWorkflowInstance) -> Self {
      Self {
         id: params.id,
         tenant_id: params.tenant_id,
         definition_id: params.definition_id,
         definition_version: params.definition_version,
         display_number: params.display_number,
         title: params.title,
         form_data: params.form_data,
         status: WorkflowInstanceStatus::Draft,
         version: Version::initial(),
         current_step_id: None,
         initiated_by: params.initiated_by,
         submitted_at: None,
         completed_at: None,
         created_at: params.now,
         updated_at: params.now,
      }
   }

   /// 既存のデータから復元する
   pub fn from_db(record: WorkflowInstanceRecord) -> Self {
      Self {
         id: record.id,
         tenant_id: record.tenant_id,
         definition_id: record.definition_id,
         definition_version: record.definition_version,
         display_number: record.display_number,
         title: record.title,
         form_data: record.form_data,
         status: record.status,
         version: record.version,
         current_step_id: record.current_step_id,
         initiated_by: record.initiated_by,
         submitted_at: record.submitted_at,
         completed_at: record.completed_at,
         created_at: record.created_at,
         updated_at: record.updated_at,
      }
   }

   // Getter メソッド

   pub fn id(&self) -> &WorkflowInstanceId {
      &self.id
   }

   pub fn tenant_id(&self) -> &TenantId {
      &self.tenant_id
   }

   pub fn definition_id(&self) -> &WorkflowDefinitionId {
      &self.definition_id
   }

   pub fn definition_version(&self) -> Version {
      self.definition_version
   }

   pub fn display_number(&self) -> DisplayNumber {
      self.display_number
   }

   pub fn version(&self) -> Version {
      self.version
   }

   pub fn title(&self) -> &str {
      &self.title
   }

   pub fn form_data(&self) -> &JsonValue {
      &self.form_data
   }

   pub fn status(&self) -> WorkflowInstanceStatus {
      self.status
   }

   pub fn current_step_id(&self) -> Option<&str> {
      self.current_step_id.as_deref()
   }

   pub fn initiated_by(&self) -> &UserId {
      &self.initiated_by
   }

   pub fn submitted_at(&self) -> Option<DateTime<Utc>> {
      self.submitted_at
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

   /// インスタンスが編集可能かチェックする
   pub fn can_edit(&self) -> Result<(), DomainError> {
      if self.status != WorkflowInstanceStatus::Draft {
         return Err(DomainError::Validation(
            "下書き状態でのみ編集可能です".to_string(),
         ));
      }
      Ok(())
   }

   /// インスタンスを申請した新しいインスタンスを返す
   pub fn submitted(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
      if self.status != WorkflowInstanceStatus::Draft {
         return Err(DomainError::Validation(
            "下書き状態でのみ申請可能です".to_string(),
         ));
      }

      Ok(Self {
         status: WorkflowInstanceStatus::Pending,
         submitted_at: Some(now),
         updated_at: now,
         ..self
      })
   }

   /// インスタンスを承認完了にした新しいインスタンスを返す
   pub fn approved(self, now: DateTime<Utc>) -> Self {
      Self {
         status: WorkflowInstanceStatus::Approved,
         completed_at: Some(now),
         updated_at: now,
         ..self
      }
   }

   /// インスタンスを却下した新しいインスタンスを返す
   pub fn rejected(self, now: DateTime<Utc>) -> Self {
      Self {
         status: WorkflowInstanceStatus::Rejected,
         completed_at: Some(now),
         updated_at: now,
         ..self
      }
   }

   /// インスタンスを取り消した新しいインスタンスを返す
   pub fn cancelled(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
      if matches!(
         self.status,
         WorkflowInstanceStatus::Approved
            | WorkflowInstanceStatus::Rejected
            | WorkflowInstanceStatus::Cancelled
      ) {
         return Err(DomainError::Validation(
            "完了済みのワークフローは取り消せません".to_string(),
         ));
      }

      Ok(Self {
         status: WorkflowInstanceStatus::Cancelled,
         completed_at: Some(now),
         updated_at: now,
         ..self
      })
   }

   /// 現在のステップを更新した新しいインスタンスを返す
   pub fn with_current_step(self, step_id: String, now: DateTime<Utc>) -> Self {
      Self {
         current_step_id: Some(step_id),
         status: WorkflowInstanceStatus::InProgress,
         version: self.version.next(),
         updated_at: now,
         ..self
      }
   }

   /// ステップ承認による完了処理
   ///
   /// InProgress 状態のインスタンスを Approved に遷移させる。
   /// version をインクリメントして楽観的ロックに対応。
   ///
   /// # Errors
   ///
   /// - `DomainError::Validation`: InProgress 以外の状態で呼び出した場合
   pub fn complete_with_approval(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
      if self.status != WorkflowInstanceStatus::InProgress {
         return Err(DomainError::Validation(format!(
            "承認完了は処理中状態でのみ可能です（現在: {}）",
            self.status
         )));
      }

      Ok(Self {
         status: WorkflowInstanceStatus::Approved,
         version: self.version.next(),
         completed_at: Some(now),
         updated_at: now,
         ..self
      })
   }

   /// ステップ却下による完了処理
   ///
   /// InProgress 状態のインスタンスを Rejected に遷移させる。
   /// version をインクリメントして楽観的ロックに対応。
   ///
   /// # Errors
   ///
   /// - `DomainError::Validation`: InProgress 以外の状態で呼び出した場合
   pub fn complete_with_rejection(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
      if self.status != WorkflowInstanceStatus::InProgress {
         return Err(DomainError::Validation(format!(
            "却下完了は処理中状態でのみ可能です（現在: {}）",
            self.status
         )));
      }

      Ok(Self {
         status: WorkflowInstanceStatus::Rejected,
         version: self.version.next(),
         completed_at: Some(now),
         updated_at: now,
         ..self
      })
   }
}

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
#[derive(Debug, Clone)]
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
   fn test_instance(now: DateTime<Utc>) -> WorkflowInstance {
      WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: TenantId::new(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(1).unwrap(),
         title: "テスト申請".to_string(),
         form_data: json!({"field": "value"}),
         initiated_by: UserId::new(),
         now,
      })
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
   // WorkflowInstance のテスト
   // =========================================================================

   mod workflow_instance {
      use super::*;

      #[rstest]
      fn test_新規作成時にversionは1(test_instance: WorkflowInstance) {
         assert_eq!(test_instance.version().as_u32(), 1);
      }

      #[rstest]
      fn test_新規作成時のcreated_atとupdated_atは注入された値と一致する(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         assert_eq!(test_instance.created_at(), now);
         assert_eq!(test_instance.updated_at(), now);
      }

      #[rstest]
      fn test_申請後のsubmitted_atは注入された値と一致する(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let submitted = test_instance.submitted(now).unwrap();
         assert_eq!(submitted.submitted_at(), Some(now));
         assert_eq!(submitted.updated_at(), now);
      }

      #[rstest]
      fn test_承認完了でステータスが承認済みになる(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);

         let result = instance.complete_with_approval(now);

         assert!(result.is_ok());
         let approved = result.unwrap();
         assert_eq!(approved.status(), WorkflowInstanceStatus::Approved);
         assert_eq!(approved.completed_at(), Some(now));
      }

      #[rstest]
      fn test_承認完了でversionがインクリメントされる(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let original_version = instance.version();

         let approved = instance.complete_with_approval(now).unwrap();

         assert_eq!(approved.version().as_u32(), original_version.as_u32() + 1);
      }

      #[rstest]
      fn test_却下完了でステータスが却下済みになる(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);

         let result = instance.complete_with_rejection(now);

         assert!(result.is_ok());
         let rejected = result.unwrap();
         assert_eq!(rejected.status(), WorkflowInstanceStatus::Rejected);
         assert_eq!(rejected.completed_at(), Some(now));
      }

      #[rstest]
      fn test_却下完了でversionがインクリメントされる(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let original_version = instance.version();

         let rejected = instance.complete_with_rejection(now).unwrap();

         assert_eq!(rejected.version().as_u32(), original_version.as_u32() + 1);
      }

      #[rstest]
      fn test_処理中以外で承認完了するとエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let result = test_instance.complete_with_approval(now);

         assert!(result.is_err());
      }

      #[rstest]
      fn test_処理中以外で却下完了するとエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let result = test_instance.complete_with_rejection(now);

         assert!(result.is_err());
      }

      // --- cancelled() テスト ---

      #[rstest]
      fn test_下書きからの取消でキャンセルになる(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let result = test_instance.cancelled(now);

         assert!(result.is_ok());
         let cancelled = result.unwrap();
         assert_eq!(cancelled.status(), WorkflowInstanceStatus::Cancelled);
         assert_eq!(cancelled.completed_at(), Some(now));
      }

      #[rstest]
      fn test_申請済みからの取消でキャンセルになる(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance.submitted(now).unwrap();

         let result = instance.cancelled(now);

         assert!(result.is_ok());
         let cancelled = result.unwrap();
         assert_eq!(cancelled.status(), WorkflowInstanceStatus::Cancelled);
      }

      #[rstest]
      fn test_処理中からの取消でキャンセルになる(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);

         let result = instance.cancelled(now);

         assert!(result.is_ok());
         let cancelled = result.unwrap();
         assert_eq!(cancelled.status(), WorkflowInstanceStatus::Cancelled);
      }

      #[rstest]
      fn test_承認済みからの取消はエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now)
            .complete_with_approval(now)
            .unwrap();

         let result = instance.cancelled(now);

         assert!(result.is_err());
      }

      #[rstest]
      fn test_却下済みからの取消はエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now)
            .complete_with_rejection(now)
            .unwrap();

         let result = instance.cancelled(now);

         assert!(result.is_err());
      }

      #[rstest]
      fn test_キャンセル済みからの取消はエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance.cancelled(now).unwrap();

         let result = instance.cancelled(now);

         assert!(result.is_err());
      }

      // --- submitted() 異常系テスト ---

      #[rstest]
      fn test_申請済みからの再申請はエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance.submitted(now).unwrap();

         let result = instance.submitted(now);

         assert!(result.is_err());
      }

      #[rstest]
      fn test_処理中からの申請はエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);

         let result = instance.submitted(now);

         assert!(result.is_err());
      }
   }

   // =========================================================================
   // WorkflowStep のテスト
   // =========================================================================

   mod workflow_step {
      use super::*;

      #[rstest]
      fn test_新規作成時にversionは1(test_step: WorkflowStep) {
         assert_eq!(test_step.version().as_u32(), 1);
      }

      #[rstest]
      fn test_新規作成時のcreated_atとupdated_atは注入された値と一致する(
         test_step: WorkflowStep,
         now: DateTime<Utc>,
      ) {
         assert_eq!(test_step.created_at(), now);
         assert_eq!(test_step.updated_at(), now);
      }

      #[rstest]
      fn test_アクティブ化後のstarted_atは注入された値と一致する(
         test_step: WorkflowStep,
         now: DateTime<Utc>,
      ) {
         let step = test_step.activated(now);
         assert_eq!(step.started_at(), Some(now));
         assert_eq!(step.updated_at(), now);
      }

      #[rstest]
      fn test_承認で完了と承認済みになる(test_step: WorkflowStep, now: DateTime<Utc>) {
         let step = test_step.activated(now);

         let result = step.approve(None, now);

         assert!(result.is_ok());
         let approved = result.unwrap();
         assert_eq!(approved.status(), WorkflowStepStatus::Completed);
         assert_eq!(approved.decision(), Some(StepDecision::Approved));
         assert_eq!(approved.completed_at(), Some(now));
      }

      #[rstest]
      fn test_承認でversionがインクリメントされる(
         test_step: WorkflowStep,
         now: DateTime<Utc>,
      ) {
         let step = test_step.activated(now);
         let original_version = step.version();

         let approved = step.approve(None, now).unwrap();

         assert_eq!(approved.version().as_u32(), original_version.as_u32() + 1);
      }

      #[rstest]
      fn test_承認でコメントが設定される(test_step: WorkflowStep, now: DateTime<Utc>) {
         let step = test_step.activated(now);

         let approved = step.approve(Some("承認します".to_string()), now).unwrap();

         assert_eq!(approved.comment(), Some("承認します"));
      }

      #[rstest]
      fn test_却下で完了と却下済みになる(test_step: WorkflowStep, now: DateTime<Utc>) {
         let step = test_step.activated(now);

         let result = step.reject(None, now);

         assert!(result.is_ok());
         let rejected = result.unwrap();
         assert_eq!(rejected.status(), WorkflowStepStatus::Completed);
         assert_eq!(rejected.decision(), Some(StepDecision::Rejected));
      }

      #[rstest]
      fn test_却下でversionがインクリメントされる(
         test_step: WorkflowStep,
         now: DateTime<Utc>,
      ) {
         let step = test_step.activated(now);
         let original_version = step.version();

         let rejected = step.reject(None, now).unwrap();

         assert_eq!(rejected.version().as_u32(), original_version.as_u32() + 1);
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
      fn test_差戻しで完了と差戻しになる(test_step: WorkflowStep, now: DateTime<Utc>) {
         let step = test_step.activated(now);

         let result = step.completed(
            StepDecision::RequestChanges,
            Some("修正してください".to_string()),
            now,
         );

         assert!(result.is_ok());
         let completed = result.unwrap();
         assert_eq!(completed.status(), WorkflowStepStatus::Completed);
         assert_eq!(completed.decision(), Some(StepDecision::RequestChanges));
         assert_eq!(completed.comment(), Some("修正してください"));
         assert_eq!(completed.completed_at(), Some(now));
      }
   }

   // =========================================================================
   // WorkflowDefinition のテスト
   // =========================================================================

   mod workflow_definition {
      use super::*;

      #[fixture]
      fn test_definition(now: DateTime<Utc>) -> WorkflowDefinition {
         WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id: TenantId::new(),
            name: WorkflowName::new("テスト定義").unwrap(),
            description: Some("テスト用".to_string()),
            definition: json!({"steps": []}),
            created_by: UserId::new(),
            now,
         })
      }

      #[rstest]
      fn test_公開でステータスが公開済みになる(
         test_definition: WorkflowDefinition,
         now: DateTime<Utc>,
      ) {
         let result = test_definition.published(now);

         assert!(result.is_ok());
         let published = result.unwrap();
         assert_eq!(published.status(), WorkflowDefinitionStatus::Published);
         assert_eq!(published.updated_at(), now);
      }

      #[rstest]
      fn test_公開済みの再公開はエラー(
         test_definition: WorkflowDefinition,
         now: DateTime<Utc>,
      ) {
         let published = test_definition.published(now).unwrap();

         let result = published.published(now);

         assert!(result.is_err());
      }

      #[rstest]
      fn test_アーカイブでステータスがアーカイブ済みになる(
         test_definition: WorkflowDefinition,
         now: DateTime<Utc>,
      ) {
         let published = test_definition.published(now).unwrap();

         let archived = published.archived(now);

         assert_eq!(archived.status(), WorkflowDefinitionStatus::Archived);
         assert_eq!(archived.updated_at(), now);
      }
   }
}
