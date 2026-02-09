//! # ワークフロー定義
//!
//! ワークフローのテンプレートを管理する。
//! 再利用可能な定義を作成し、公開・アーカイブのライフサイクルを持つ。

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
   value_objects::{Version, WorkflowName},
};

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
#[derive(Debug, Clone, PartialEq, Eq)]
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

   mod workflow_definition {
      use pretty_assertions::assert_eq;

      use super::*;

      #[rstest]
      fn test_公開後の状態(test_definition: WorkflowDefinition, now: DateTime<Utc>) {
         let before = test_definition.clone();

         let sut = test_definition.published(now).unwrap();

         let expected = WorkflowDefinition::from_db(WorkflowDefinitionRecord {
            id:          before.id().clone(),
            tenant_id:   before.tenant_id().clone(),
            name:        before.name().clone(),
            description: before.description().map(|s| s.to_string()),
            version:     before.version(),
            definition:  before.definition().clone(),
            status:      WorkflowDefinitionStatus::Published,
            created_by:  before.created_by().clone(),
            created_at:  before.created_at(),
            updated_at:  now,
         });
         assert_eq!(sut, expected);
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
      fn test_アーカイブ後の状態(test_definition: WorkflowDefinition, now: DateTime<Utc>) {
         let published = test_definition.published(now).unwrap();
         let before = published.clone();

         let sut = published.archived(now);

         let expected = WorkflowDefinition::from_db(WorkflowDefinitionRecord {
            id:          before.id().clone(),
            tenant_id:   before.tenant_id().clone(),
            name:        before.name().clone(),
            description: before.description().map(|s| s.to_string()),
            version:     before.version(),
            definition:  before.definition().clone(),
            status:      WorkflowDefinitionStatus::Archived,
            created_by:  before.created_by().clone(),
            created_at:  before.created_at(),
            updated_at:  now,
         });
         assert_eq!(sut, expected);
      }
   }
}
