//! # ワークフローインスタンスの DB レコード変換
//!
//! DB のフラット構造と型安全ステートマシンの ADT 間の変換を行う。
//! `from_db()` で不変条件（INV-I1〜I9）を検証する。
//!
//! ## 使用例
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use chrono::Utc;
//! use ringiflow_domain::{
//!     tenant::TenantId,
//!     user::UserId,
//!     value_objects::{DisplayNumber, Version},
//!     workflow::{
//!         WorkflowDefinitionId,
//!         WorkflowInstance,
//!         WorkflowInstanceId,
//!         WorkflowInstanceRecord,
//!         WorkflowInstanceStatus,
//!     },
//! };
//! use serde_json::json;
//!
//! let now = Utc::now();
//! let record = WorkflowInstanceRecord {
//!     id: WorkflowInstanceId::new(),
//!     tenant_id: TenantId::new(),
//!     definition_id: WorkflowDefinitionId::new(),
//!     definition_version: Version::initial(),
//!     display_number: DisplayNumber::new(1)?,
//!     title: "テスト申請".to_string(),
//!     form_data: json!({}),
//!     status: WorkflowInstanceStatus::Draft,
//!     version: Version::initial(),
//!     current_step_id: None,
//!     initiated_by: UserId::new(),
//!     submitted_at: None,
//!     completed_at: None,
//!     created_at: now,
//!     updated_at: now,
//! };
//!
//! let instance = WorkflowInstance::from_db(record)?;
//! assert_eq!(instance.status(), WorkflowInstanceStatus::Draft);
//! # Ok(())
//! # }
//! ```
//!
//! 詳細: [エンティティ影響マップ](../../../docs/40_詳細設計書/エンティティ影響マップ/WorkflowInstance.md)（INV-I1〜I9）

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

use super::{
    WorkflowInstance,
    WorkflowInstanceStatus,
    state::{
        CancelledState,
        ChangesRequestedState,
        CompletedState,
        InProgressState,
        PendingState,
        WorkflowInstanceState,
    },
};
use crate::{
    DomainError,
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayNumber, Version},
    workflow::definition::WorkflowDefinitionId,
};

/// ワークフローインスタンスの DB 復元パラメータ
///
/// DB スキーマのフラット構造を表現する。`from_db()` で不変条件を検証して ADT に変換する。
pub struct WorkflowInstanceRecord {
    pub id: super::WorkflowInstanceId,
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
    /// 既存のデータから復元する
    ///
    /// DB のフラット構造から ADT に変換し、不変条件（INV-I1〜I9）を検証する。
    ///
    /// # Errors
    ///
    /// - `DomainError::Validation`: 不変条件違反（例: InProgress で current_step_id が None）
    pub fn from_db(record: WorkflowInstanceRecord) -> Result<Self, DomainError> {
        let state = match record.status {
            WorkflowInstanceStatus::Draft => WorkflowInstanceState::Draft,
            WorkflowInstanceStatus::Pending => {
                let submitted_at = record.submitted_at.ok_or_else(|| {
                    DomainError::Validation(
                        "Pending インスタンスには submitted_at が必要です".to_string(),
                    )
                })?;
                WorkflowInstanceState::Pending(PendingState { submitted_at })
            }
            WorkflowInstanceStatus::InProgress => {
                let current_step_id = record.current_step_id.ok_or_else(|| {
                    DomainError::Validation(
                        "InProgress インスタンスには current_step_id が必要です".to_string(),
                    )
                })?;
                let submitted_at = record.submitted_at.ok_or_else(|| {
                    DomainError::Validation(
                        "InProgress インスタンスには submitted_at が必要です".to_string(),
                    )
                })?;
                WorkflowInstanceState::InProgress(InProgressState {
                    current_step_id,
                    submitted_at,
                })
            }
            WorkflowInstanceStatus::Approved => {
                let current_step_id = record.current_step_id.ok_or_else(|| {
                    DomainError::Validation(
                        "Approved インスタンスには current_step_id が必要です".to_string(),
                    )
                })?;
                let submitted_at = record.submitted_at.ok_or_else(|| {
                    DomainError::Validation(
                        "Approved インスタンスには submitted_at が必要です".to_string(),
                    )
                })?;
                let completed_at = record.completed_at.ok_or_else(|| {
                    DomainError::Validation(
                        "Approved インスタンスには completed_at が必要です".to_string(),
                    )
                })?;
                WorkflowInstanceState::Approved(CompletedState {
                    current_step_id,
                    submitted_at,
                    completed_at,
                })
            }
            WorkflowInstanceStatus::Rejected => {
                let current_step_id = record.current_step_id.ok_or_else(|| {
                    DomainError::Validation(
                        "Rejected インスタンスには current_step_id が必要です".to_string(),
                    )
                })?;
                let submitted_at = record.submitted_at.ok_or_else(|| {
                    DomainError::Validation(
                        "Rejected インスタンスには submitted_at が必要です".to_string(),
                    )
                })?;
                let completed_at = record.completed_at.ok_or_else(|| {
                    DomainError::Validation(
                        "Rejected インスタンスには completed_at が必要です".to_string(),
                    )
                })?;
                WorkflowInstanceState::Rejected(CompletedState {
                    current_step_id,
                    submitted_at,
                    completed_at,
                })
            }
            WorkflowInstanceStatus::Cancelled => {
                let completed_at = record.completed_at.ok_or_else(|| {
                    DomainError::Validation(
                        "Cancelled インスタンスには completed_at が必要です".to_string(),
                    )
                })?;
                let cancelled_state = match (record.current_step_id, record.submitted_at) {
                    (None, None) => CancelledState::FromDraft { completed_at },
                    (None, Some(submitted_at)) => CancelledState::FromPending {
                        submitted_at,
                        completed_at,
                    },
                    (Some(current_step_id), Some(submitted_at)) => CancelledState::FromActive {
                        current_step_id,
                        submitted_at,
                        completed_at,
                    },
                    (Some(_), None) => {
                        return Err(DomainError::Validation(
                                "Cancelled インスタンスで current_step_id がある場合は submitted_at が必要です"
                                    .to_string(),
                            ));
                    }
                };
                WorkflowInstanceState::Cancelled(cancelled_state)
            }
            WorkflowInstanceStatus::ChangesRequested => {
                let current_step_id = record.current_step_id.ok_or_else(|| {
                    DomainError::Validation(
                        "ChangesRequested インスタンスには current_step_id が必要です".to_string(),
                    )
                })?;
                let submitted_at = record.submitted_at.ok_or_else(|| {
                    DomainError::Validation(
                        "ChangesRequested インスタンスには submitted_at が必要です".to_string(),
                    )
                })?;
                WorkflowInstanceState::ChangesRequested(ChangesRequestedState {
                    current_step_id,
                    submitted_at,
                })
            }
        };

        Ok(Self {
            id: record.id,
            tenant_id: record.tenant_id,
            definition_id: record.definition_id,
            definition_version: record.definition_version,
            display_number: record.display_number,
            title: record.title,
            form_data: record.form_data,
            version: record.version,
            initiated_by: record.initiated_by,
            created_at: record.created_at,
            updated_at: record.updated_at,
            state,
        })
    }
}
