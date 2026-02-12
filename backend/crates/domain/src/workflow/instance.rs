//! # ワークフローインスタンス
//!
//! ワークフロー定義から生成された実行中の案件を管理する。
//! フォームデータと進捗状態を保持し、申請・承認・却下のライフサイクルを持つ。

use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use strum::IntoStaticStr;
use uuid::Uuid;

use super::definition::WorkflowDefinitionId;
use crate::{
   DomainError,
   tenant::TenantId,
   user::UserId,
   value_objects::{DisplayNumber, Version},
};

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
   /// 要修正（差し戻し）
   ChangesRequested,
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
         "changes_requested" => Ok(Self::ChangesRequested),
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
   ///
   /// Submit 時の初期遷移（Pending → InProgress）用。
   /// 承認後の次ステップ遷移には `advance_to_next_step` を使用する。
   pub fn with_current_step(self, step_id: String, now: DateTime<Utc>) -> Self {
      Self {
         current_step_id: Some(step_id),
         status: WorkflowInstanceStatus::InProgress,
         version: self.version.next(),
         updated_at: now,
         ..self
      }
   }

   /// 次の承認ステップに遷移する
   ///
   /// InProgress 状態のインスタンスの current_step_id を次のステップに更新する。
   /// version をインクリメントして楽観的ロックに対応。
   ///
   /// # Errors
   ///
   /// - `DomainError::Validation`: InProgress 以外の状態で呼び出した場合
   pub fn advance_to_next_step(
      self,
      next_step_id: String,
      now: DateTime<Utc>,
   ) -> Result<Self, DomainError> {
      if self.status != WorkflowInstanceStatus::InProgress {
         return Err(DomainError::Validation(format!(
            "次ステップ遷移は処理中状態でのみ可能です（現在: {}）",
            self.status
         )));
      }

      Ok(Self {
         current_step_id: Some(next_step_id),
         version: self.version.next(),
         updated_at: now,
         ..self
      })
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

   /// ステップ差し戻しによる要修正遷移
   ///
   /// InProgress 状態のインスタンスを ChangesRequested に遷移させる。
   /// version をインクリメントして楽観的ロックに対応。
   /// ChangesRequested は中間状態のため completed_at は設定しない。
   ///
   /// # Errors
   ///
   /// - `DomainError::Validation`: InProgress 以外の状態で呼び出した場合
   pub fn complete_with_request_changes(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
      if self.status != WorkflowInstanceStatus::InProgress {
         return Err(DomainError::Validation(format!(
            "差し戻しは処理中状態でのみ可能です（現在: {}）",
            self.status
         )));
      }

      Ok(Self {
         status: WorkflowInstanceStatus::ChangesRequested,
         version: self.version.next(),
         updated_at: now,
         ..self
      })
   }

   /// 再申請
   ///
   /// ChangesRequested 状態のインスタンスを InProgress に遷移させる。
   /// フォームデータを更新し、新しいステップで再開する。
   /// version をインクリメントして楽観的ロックに対応。
   ///
   /// # Errors
   ///
   /// - `DomainError::Validation`: ChangesRequested 以外の状態で呼び出した場合
   pub fn resubmitted(
      self,
      form_data: JsonValue,
      step_id: String,
      now: DateTime<Utc>,
   ) -> Result<Self, DomainError> {
      if self.status != WorkflowInstanceStatus::ChangesRequested {
         return Err(DomainError::Validation(format!(
            "再申請は要修正状態でのみ可能です（現在: {}）",
            self.status
         )));
      }

      Ok(Self {
         status: WorkflowInstanceStatus::InProgress,
         form_data,
         current_step_id: Some(step_id),
         version: self.version.next(),
         completed_at: None,
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

   mod workflow_instance {
      use pretty_assertions::assert_eq;

      use super::*;

      #[rstest]
      fn test_新規作成の初期状態(test_instance: WorkflowInstance, now: DateTime<Utc>) {
         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: test_instance.id().clone(),
            tenant_id: test_instance.tenant_id().clone(),
            definition_id: test_instance.definition_id().clone(),
            definition_version: test_instance.definition_version(),
            display_number: test_instance.display_number(),
            title: test_instance.title().to_string(),
            form_data: test_instance.form_data().clone(),
            status: WorkflowInstanceStatus::Draft,
            version: Version::initial(),
            current_step_id: None,
            initiated_by: test_instance.initiated_by().clone(),
            submitted_at: None,
            completed_at: None,
            created_at: now,
            updated_at: now,
         });
         assert_eq!(test_instance, expected);
      }

      #[rstest]
      fn test_申請後の状態(test_instance: WorkflowInstance, now: DateTime<Utc>) {
         let before = test_instance.clone();
         let sut = test_instance.submitted(now).unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::Pending,
            version: before.version(),
            current_step_id: None,
            initiated_by: before.initiated_by().clone(),
            submitted_at: Some(now),
            completed_at: None,
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_承認完了後の状態(test_instance: WorkflowInstance, now: DateTime<Utc>) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let before = instance.clone();

         let sut = instance.complete_with_approval(now).unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::Approved,
            version: before.version().next(),
            current_step_id: before.current_step_id().map(|s| s.to_string()),
            initiated_by: before.initiated_by().clone(),
            submitted_at: before.submitted_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_却下完了後の状態(test_instance: WorkflowInstance, now: DateTime<Utc>) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let before = instance.clone();

         let sut = instance.complete_with_rejection(now).unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::Rejected,
            version: before.version().next(),
            current_step_id: before.current_step_id().map(|s| s.to_string()),
            initiated_by: before.initiated_by().clone(),
            submitted_at: before.submitted_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
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
      fn test_下書きからの取消後の状態(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let before = test_instance.clone();

         let sut = test_instance.cancelled(now).unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::Cancelled,
            version: before.version(),
            current_step_id: None,
            initiated_by: before.initiated_by().clone(),
            submitted_at: None,
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_申請済みからの取消後の状態(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance.submitted(now).unwrap();
         let before = instance.clone();

         let sut = instance.cancelled(now).unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::Cancelled,
            version: before.version(),
            current_step_id: None,
            initiated_by: before.initiated_by().clone(),
            submitted_at: before.submitted_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_処理中からの取消後の状態(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let before = instance.clone();

         let sut = instance.cancelled(now).unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::Cancelled,
            version: before.version(),
            current_step_id: before.current_step_id().map(|s| s.to_string()),
            initiated_by: before.initiated_by().clone(),
            submitted_at: before.submitted_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
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

      // --- advance_to_next_step() テスト ---

      #[rstest]
      fn test_次ステップ遷移_処理中で成功(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let before = instance.clone();

         let sut = instance
            .advance_to_next_step("step_2".to_string(), now)
            .unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::InProgress,
            version: before.version().next(),
            current_step_id: Some("step_2".to_string()),
            initiated_by: before.initiated_by().clone(),
            submitted_at: before.submitted_at(),
            completed_at: None,
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_次ステップ遷移_処理中以外ではエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         // Draft 状態
         let result = test_instance.advance_to_next_step("step_2".to_string(), now);

         assert!(result.is_err());
      }

      #[rstest]
      fn test_次ステップ遷移_versionがインクリメントされる(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let before_version = instance.version();

         let sut = instance
            .advance_to_next_step("step_2".to_string(), now)
            .unwrap();

         assert_eq!(sut.version(), before_version.next());
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

      // --- complete_with_request_changes() テスト ---

      #[rstest]
      fn test_差し戻し完了後の状態(test_instance: WorkflowInstance, now: DateTime<Utc>) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);
         let before = instance.clone();

         let sut = instance.complete_with_request_changes(now).unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::ChangesRequested,
            version: before.version().next(),
            current_step_id: before.current_step_id().map(|s| s.to_string()),
            initiated_by: before.initiated_by().clone(),
            submitted_at: before.submitted_at(),
            completed_at: None,
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_処理中以外で差し戻しするとエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         // Draft 状態からは差し戻し不可
         let result = test_instance.complete_with_request_changes(now);

         assert!(result.is_err());
      }

      // --- resubmitted() テスト ---

      #[rstest]
      fn test_再申請後の状態(test_instance: WorkflowInstance, now: DateTime<Utc>) {
         let new_form_data = json!({"field": "updated_value"});
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now)
            .complete_with_request_changes(now)
            .unwrap();
         let before = instance.clone();

         let sut = instance
            .resubmitted(new_form_data.clone(), "new_step_1".to_string(), now)
            .unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: new_form_data,
            status: WorkflowInstanceStatus::InProgress,
            version: before.version().next(),
            current_step_id: Some("new_step_1".to_string()),
            initiated_by: before.initiated_by().clone(),
            submitted_at: before.submitted_at(),
            completed_at: None,
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }

      #[rstest]
      fn test_要修正以外で再申請するとエラー(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         // InProgress 状態からは再申請不可
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now);

         let result = instance.resubmitted(json!({}), "new_step_1".to_string(), now);

         assert!(result.is_err());
      }

      // --- 要修正状態からの取消テスト ---

      #[rstest]
      fn test_要修正状態からの取消後の状態(
         test_instance: WorkflowInstance,
         now: DateTime<Utc>,
      ) {
         let instance = test_instance
            .submitted(now)
            .unwrap()
            .with_current_step("step_1".to_string(), now)
            .complete_with_request_changes(now)
            .unwrap();
         let before = instance.clone();

         let sut = instance.cancelled(now).unwrap();

         let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: before.id().clone(),
            tenant_id: before.tenant_id().clone(),
            definition_id: before.definition_id().clone(),
            definition_version: before.definition_version(),
            display_number: before.display_number(),
            title: before.title().to_string(),
            form_data: before.form_data().clone(),
            status: WorkflowInstanceStatus::Cancelled,
            version: before.version(),
            current_step_id: before.current_step_id().map(|s| s.to_string()),
            initiated_by: before.initiated_by().clone(),
            submitted_at: before.submitted_at(),
            completed_at: Some(now),
            created_at: before.created_at(),
            updated_at: now,
         });
         assert_eq!(sut, expected);
      }
   }
}
