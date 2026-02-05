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
//!     WorkflowDefinition, WorkflowDefinitionId, WorkflowDefinitionStatus
//! };
//! use ringiflow_domain::{tenant::TenantId, user::UserId, value_objects::WorkflowName};
//! use serde_json::json;
//!
//! // ワークフロー定義の作成
//! let definition = WorkflowDefinition::new(
//!     WorkflowDefinitionId::new(),
//!     TenantId::new(),
//!     WorkflowName::new("汎用申請").unwrap(),
//!     Some("シンプルな1段階承認".to_string()),
//!     json!({"steps": []}),
//!     UserId::new(),
//!     chrono::Utc::now(),
//! );
//! assert_eq!(definition.status(), WorkflowDefinitionStatus::Draft);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for WorkflowDefinitionId {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
   }
}

/// ワークフロー定義ステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowDefinitionStatus {
   /// 下書き（編集中）
   Draft,
   /// 公開済み（利用可能）
   Published,
   /// アーカイブ済み（非表示）
   Archived,
}

impl WorkflowDefinitionStatus {
   pub fn as_str(&self) -> &'static str {
      match self {
         Self::Draft => "draft",
         Self::Published => "published",
         Self::Archived => "archived",
      }
   }
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

impl WorkflowDefinition {
   /// 新しいワークフロー定義を作成する
   pub fn new(
      id: WorkflowDefinitionId,
      tenant_id: TenantId,
      name: WorkflowName,
      description: Option<String>,
      definition: JsonValue,
      created_by: UserId,
      now: DateTime<Utc>,
   ) -> Self {
      Self {
         id,
         tenant_id,
         name,
         description,
         version: Version::initial(),
         definition,
         status: WorkflowDefinitionStatus::Draft,
         created_by,
         created_at: now,
         updated_at: now,
      }
   }

   /// 既存のデータから復元する
   #[allow(clippy::too_many_arguments)]
   pub fn from_db(
      id: WorkflowDefinitionId,
      tenant_id: TenantId,
      name: WorkflowName,
      description: Option<String>,
      version: Version,
      definition: JsonValue,
      status: WorkflowDefinitionStatus,
      created_by: UserId,
      created_at: DateTime<Utc>,
      updated_at: DateTime<Utc>,
   ) -> Self {
      Self {
         id,
         tenant_id,
         name,
         description,
         version,
         definition,
         status,
         created_by,
         created_at,
         updated_at,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for WorkflowInstanceId {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
   }
}

/// ワークフローインスタンスステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

impl WorkflowInstanceStatus {
   pub fn as_str(&self) -> &'static str {
      match self {
         Self::Draft => "draft",
         Self::Pending => "pending",
         Self::InProgress => "in_progress",
         Self::Approved => "approved",
         Self::Rejected => "rejected",
         Self::Cancelled => "cancelled",
      }
   }
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

impl WorkflowInstance {
   /// 新しいワークフローインスタンスを作成する
   pub fn new(
      id: WorkflowInstanceId,
      tenant_id: TenantId,
      definition_id: WorkflowDefinitionId,
      definition_version: Version,
      display_number: DisplayNumber,
      title: String,
      form_data: JsonValue,
      initiated_by: UserId,
      now: DateTime<Utc>,
   ) -> Self {
      Self {
         id,
         tenant_id,
         definition_id,
         definition_version,
         display_number,
         title,
         form_data,
         status: WorkflowInstanceStatus::Draft,
         version: Version::initial(),
         current_step_id: None,
         initiated_by,
         submitted_at: None,
         completed_at: None,
         created_at: now,
         updated_at: now,
      }
   }

   /// 既存のデータから復元する
   #[allow(clippy::too_many_arguments)]
   pub fn from_db(
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
   ) -> Self {
      Self {
         id,
         tenant_id,
         definition_id,
         definition_version,
         display_number,
         title,
         form_data,
         status,
         version,
         current_step_id,
         initiated_by,
         submitted_at,
         completed_at,
         created_at,
         updated_at,
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
            self.status.as_str()
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
            self.status.as_str()
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for WorkflowStepId {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
   }
}

/// ワークフローステップステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

impl WorkflowStepStatus {
   pub fn as_str(&self) -> &'static str {
      match self {
         Self::Pending => "pending",
         Self::Active => "active",
         Self::Completed => "completed",
         Self::Skipped => "skipped",
      }
   }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepDecision {
   /// 承認
   Approved,
   /// 却下
   Rejected,
   /// 修正依頼
   RequestChanges,
}

impl StepDecision {
   pub fn as_str(&self) -> &'static str {
      match self {
         Self::Approved => "approved",
         Self::Rejected => "rejected",
         Self::RequestChanges => "request_changes",
      }
   }
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
   id:           WorkflowStepId,
   instance_id:  WorkflowInstanceId,
   step_id:      String,
   step_name:    String,
   step_type:    String,
   status:       WorkflowStepStatus,
   version:      Version,
   assigned_to:  Option<UserId>,
   decision:     Option<StepDecision>,
   comment:      Option<String>,
   due_date:     Option<DateTime<Utc>>,
   started_at:   Option<DateTime<Utc>>,
   completed_at: Option<DateTime<Utc>>,
   created_at:   DateTime<Utc>,
   updated_at:   DateTime<Utc>,
}

impl WorkflowStep {
   /// 新しいワークフローステップを作成する
   pub fn new(
      instance_id: WorkflowInstanceId,
      step_id: String,
      step_name: String,
      step_type: String,
      assigned_to: Option<UserId>,
   ) -> Self {
      let now = Utc::now();
      Self {
         id: WorkflowStepId::new(),
         instance_id,
         step_id,
         step_name,
         step_type,
         status: WorkflowStepStatus::Pending,
         version: Version::initial(),
         assigned_to,
         decision: None,
         comment: None,
         due_date: None,
         started_at: None,
         completed_at: None,
         created_at: now,
         updated_at: now,
      }
   }

   /// 既存のデータから復元する
   #[allow(clippy::too_many_arguments)]
   pub fn from_db(
      id: WorkflowStepId,
      instance_id: WorkflowInstanceId,
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
   ) -> Self {
      Self {
         id,
         instance_id,
         step_id,
         step_name,
         step_type,
         status,
         version,
         assigned_to,
         decision,
         comment,
         due_date,
         started_at,
         completed_at,
         created_at,
         updated_at,
      }
   }

   // Getter メソッド

   pub fn id(&self) -> &WorkflowStepId {
      &self.id
   }

   pub fn instance_id(&self) -> &WorkflowInstanceId {
      &self.instance_id
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
   pub fn activated(self) -> Self {
      Self {
         status: WorkflowStepStatus::Active,
         started_at: Some(Utc::now()),
         updated_at: Utc::now(),
         ..self
      }
   }

   /// ステップを完了した新しいインスタンスを返す
   pub fn completed(
      self,
      decision: StepDecision,
      comment: Option<String>,
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
         completed_at: Some(Utc::now()),
         updated_at: Utc::now(),
         ..self
      })
   }

   /// ステップをスキップした新しいインスタンスを返す
   pub fn skipped(self) -> Self {
      Self {
         status: WorkflowStepStatus::Skipped,
         updated_at: Utc::now(),
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
   pub fn approve(self, comment: Option<String>) -> Result<Self, DomainError> {
      if self.status != WorkflowStepStatus::Active {
         return Err(DomainError::Validation(format!(
            "承認はアクティブ状態でのみ可能です（現在: {}）",
            self.status.as_str()
         )));
      }

      Ok(Self {
         status: WorkflowStepStatus::Completed,
         version: self.version.next(),
         decision: Some(StepDecision::Approved),
         comment,
         completed_at: Some(Utc::now()),
         updated_at: Utc::now(),
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
   pub fn reject(self, comment: Option<String>) -> Result<Self, DomainError> {
      if self.status != WorkflowStepStatus::Active {
         return Err(DomainError::Validation(format!(
            "却下はアクティブ状態でのみ可能です（現在: {}）",
            self.status.as_str()
         )));
      }

      Ok(Self {
         status: WorkflowStepStatus::Completed,
         version: self.version.next(),
         decision: Some(StepDecision::Rejected),
         comment,
         completed_at: Some(Utc::now()),
         updated_at: Utc::now(),
         ..self
      })
   }

   /// ステップが期限切れかチェックする
   pub fn is_overdue(&self) -> bool {
      if let Some(due) = self.due_date
         && self.completed_at.is_none()
      {
         return Utc::now() > due;
      }
      false
   }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
   use serde_json::json;

   use super::*;

   /// テスト用の固定タイムスタンプ
   fn test_now() -> DateTime<Utc> {
      DateTime::from_timestamp(1_700_000_000, 0).unwrap()
   }

   // ヘルパー関数
   fn create_test_instance() -> WorkflowInstance {
      WorkflowInstance::new(
         WorkflowInstanceId::new(),
         TenantId::new(),
         WorkflowDefinitionId::new(),
         Version::initial(),
         DisplayNumber::new(1).unwrap(),
         "テスト申請".to_string(),
         json!({"field": "value"}),
         UserId::new(),
         test_now(),
      )
   }

   fn create_test_step(instance_id: WorkflowInstanceId) -> WorkflowStep {
      WorkflowStep::new(
         instance_id,
         "step_1".to_string(),
         "承認".to_string(),
         "approval".to_string(),
         Some(UserId::new()),
      )
   }

   // =========================================================================
   // WorkflowInstance のテスト
   // =========================================================================

   #[allow(non_snake_case)]
   mod workflow_instance {
      use super::*;

      #[test]
      fn test_新規作成時にversionは1() {
         let instance = create_test_instance();
         assert_eq!(instance.version().as_u32(), 1);
      }

      #[test]
      fn test_新規作成時のcreated_atとupdated_atは注入された値と一致する() {
         let instance = create_test_instance();
         assert_eq!(instance.created_at(), test_now());
         assert_eq!(instance.updated_at(), test_now());
      }

      #[test]
      fn test_submitted後のsubmitted_atは注入された値と一致する() {
         let now = test_now();
         let submitted = create_test_instance().submitted(now).unwrap();
         assert_eq!(submitted.submitted_at(), Some(now));
         assert_eq!(submitted.updated_at(), now);
      }

      #[test]
      fn test_承認完了でステータスがApprovedになる() {
         let now = test_now();
         let instance = create_test_instance()
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);

         let result = instance.complete_with_approval(now);

         assert!(result.is_ok());
         let approved = result.unwrap();
         assert_eq!(approved.status(), WorkflowInstanceStatus::Approved);
         assert_eq!(approved.completed_at(), Some(now));
      }

      #[test]
      fn test_承認完了でversionがインクリメントされる() {
         let now = test_now();
         let instance = create_test_instance()
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let original_version = instance.version();

         let approved = instance.complete_with_approval(now).unwrap();

         assert_eq!(approved.version().as_u32(), original_version.as_u32() + 1);
      }

      #[test]
      fn test_却下完了でステータスがRejectedになる() {
         let now = test_now();
         let instance = create_test_instance()
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);

         let result = instance.complete_with_rejection(now);

         assert!(result.is_ok());
         let rejected = result.unwrap();
         assert_eq!(rejected.status(), WorkflowInstanceStatus::Rejected);
         assert_eq!(rejected.completed_at(), Some(now));
      }

      #[test]
      fn test_却下完了でversionがインクリメントされる() {
         let now = test_now();
         let instance = create_test_instance()
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let original_version = instance.version();

         let rejected = instance.complete_with_rejection(now).unwrap();

         assert_eq!(rejected.version().as_u32(), original_version.as_u32() + 1);
      }

      #[test]
      fn test_InProgress以外で承認完了するとエラー() {
         let instance = create_test_instance(); // Draft 状態

         let result = instance.complete_with_approval(test_now());

         assert!(result.is_err());
      }

      #[test]
      fn test_InProgress以外で却下完了するとエラー() {
         let instance = create_test_instance(); // Draft 状態

         let result = instance.complete_with_rejection(test_now());

         assert!(result.is_err());
      }
   }

   // =========================================================================
   // WorkflowStep のテスト
   // =========================================================================

   #[allow(non_snake_case)]
   mod workflow_step {
      use super::*;

      #[test]
      fn test_新規作成時にversionは1() {
         let step = create_test_step(WorkflowInstanceId::new());
         assert_eq!(step.version().as_u32(), 1);
      }

      #[test]
      fn test_approveでCompletedとApprovedになる() {
         let step = create_test_step(WorkflowInstanceId::new()).activated();

         let result = step.approve(None);

         assert!(result.is_ok());
         let approved = result.unwrap();
         assert_eq!(approved.status(), WorkflowStepStatus::Completed);
         assert_eq!(approved.decision(), Some(StepDecision::Approved));
      }

      #[test]
      fn test_approveでversionがインクリメントされる() {
         let step = create_test_step(WorkflowInstanceId::new()).activated();
         let original_version = step.version();

         let approved = step.approve(None).unwrap();

         assert_eq!(approved.version().as_u32(), original_version.as_u32() + 1);
      }

      #[test]
      fn test_approveでコメントが設定される() {
         let step = create_test_step(WorkflowInstanceId::new()).activated();

         let approved = step.approve(Some("承認します".to_string())).unwrap();

         assert_eq!(approved.comment(), Some("承認します"));
      }

      #[test]
      fn test_rejectでCompletedとRejectedになる() {
         let step = create_test_step(WorkflowInstanceId::new()).activated();

         let result = step.reject(None);

         assert!(result.is_ok());
         let rejected = result.unwrap();
         assert_eq!(rejected.status(), WorkflowStepStatus::Completed);
         assert_eq!(rejected.decision(), Some(StepDecision::Rejected));
      }

      #[test]
      fn test_rejectでversionがインクリメントされる() {
         let step = create_test_step(WorkflowInstanceId::new()).activated();
         let original_version = step.version();

         let rejected = step.reject(None).unwrap();

         assert_eq!(rejected.version().as_u32(), original_version.as_u32() + 1);
      }

      #[test]
      fn test_Active以外でapproveするとエラー() {
         let step = create_test_step(WorkflowInstanceId::new()); // Pending 状態

         let result = step.approve(None);

         assert!(result.is_err());
      }

      #[test]
      fn test_Active以外でrejectするとエラー() {
         let step = create_test_step(WorkflowInstanceId::new()); // Pending 状態

         let result = step.reject(None);

         assert!(result.is_err());
      }
   }
}
