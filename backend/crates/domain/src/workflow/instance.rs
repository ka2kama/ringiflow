//! # ワークフローインスタンス
//!
//! ワークフロー定義から生成された実行中の案件を管理する。
//! フォームデータと進捗状態を保持し、申請・承認・却下のライフサイクルを持つ。
//!
//! 状態遷移は ADT（代数的データ型）で表現し、不正な状態を型レベルで防止する。
//! 詳細: [ADR-054: ADT ベースステートマシンパターンの標準化](../../docs/05_ADR/054_ADTベースステートマシンパターンの標準化.md)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use strum::IntoStaticStr;

use super::definition::WorkflowDefinitionId;
use crate::{
    DomainError,
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayNumber, Version},
};

define_uuid_id! {
    /// ワークフローインスタンス ID
    pub struct WorkflowInstanceId;
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

/// ワークフローインスタンスの状態（ADT ベースステートマシン）
///
/// 各状態で有効なフィールドのみを持たせることで、不正な状態を型レベルで防止する。
/// 詳細: [ADR-054](../../docs/05_ADR/054_ADTベースステートマシンパターンの標準化.md),
/// [エンティティ影響マップ](../../docs/03_詳細設計書/エンティティ影響マップ/WorkflowInstance.md) INV-I1〜I9
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowInstanceState {
    /// 下書き
    Draft,
    /// 承認待ち
    Pending(PendingState),
    /// 処理中
    InProgress(InProgressState),
    /// 承認完了
    Approved(CompletedState),
    /// 却下
    Rejected(CompletedState),
    /// 取り消し
    Cancelled(CancelledState),
    /// 要修正（差し戻し）
    ChangesRequested(ChangesRequestedState),
}

/// Pending 状態の固有フィールド
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingState {
    /// 申請日時（INV-I5 を型で強制）
    pub submitted_at: DateTime<Utc>,
}

/// InProgress 状態の固有フィールド
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InProgressState {
    /// 現在のステップ ID（INV-I3, I6 を型で強制）
    pub current_step_id: String,
    /// 申請日時（INV-I6 を型で強制）
    pub submitted_at:    DateTime<Utc>,
}

/// Approved/Rejected 共通の完了状態フィールド
///
/// 両方とも InProgress からのみ遷移可能（INV-I1, I2, I7）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedState {
    /// 現在のステップ ID（INV-I7 を型で強制）
    pub current_step_id: String,
    /// 申請日時（INV-I7 を型で強制）
    pub submitted_at:    DateTime<Utc>,
    /// 完了日時（INV-I1, I2 を型で強制）
    pub completed_at:    DateTime<Utc>,
}

/// Cancelled 状態の固有フィールド
///
/// Draft/Pending/InProgress/ChangesRequested から遷移可能。
/// 前状態に依存するフィールドは Option で表現する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelledState {
    /// 現在のステップ ID（InProgress/ChangesRequested から遷移時のみ設定）
    pub current_step_id: Option<String>,
    /// 申請日時（Draft から遷移時は None）
    pub submitted_at:    Option<DateTime<Utc>>,
    /// 完了日時（INV-I9 を型で強制）
    pub completed_at:    DateTime<Utc>,
}

/// ChangesRequested 状態の固有フィールド
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesRequestedState {
    /// 現在のステップ ID（INV-I8 を型で強制）
    pub current_step_id: String,
    /// 申請日時（INV-I8 を型で強制）
    pub submitted_at:    DateTime<Utc>,
}

/// ワークフローインスタンスエンティティ
///
/// 定義から生成された実行中の申請案件。
/// フォームデータと進捗状態を保持する。
///
/// [ADR-054](../../docs/05_ADR/054_ADTベースステートマシンパターンの標準化.md) Pattern A:
/// 共通フィールドを外側に、状態固有フィールドを `state` enum に分離。
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
    version: Version,
    initiated_by: UserId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    state: WorkflowInstanceState,
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
///
/// DB スキーマのフラット構造を表現する。`from_db()` で不変条件を検証して ADT に変換する。
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
            version: Version::initial(),
            initiated_by: params.initiated_by,
            created_at: params.now,
            updated_at: params.now,
            state: WorkflowInstanceState::Draft,
        }
    }

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
                WorkflowInstanceState::Cancelled(CancelledState {
                    current_step_id: record.current_step_id,
                    submitted_at: record.submitted_at,
                    completed_at,
                })
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
        match &self.state {
            WorkflowInstanceState::Draft => WorkflowInstanceStatus::Draft,
            WorkflowInstanceState::Pending(_) => WorkflowInstanceStatus::Pending,
            WorkflowInstanceState::InProgress(_) => WorkflowInstanceStatus::InProgress,
            WorkflowInstanceState::Approved(_) => WorkflowInstanceStatus::Approved,
            WorkflowInstanceState::Rejected(_) => WorkflowInstanceStatus::Rejected,
            WorkflowInstanceState::Cancelled(_) => WorkflowInstanceStatus::Cancelled,
            WorkflowInstanceState::ChangesRequested(_) => WorkflowInstanceStatus::ChangesRequested,
        }
    }

    pub fn current_step_id(&self) -> Option<&str> {
        match &self.state {
            WorkflowInstanceState::InProgress(s) => Some(&s.current_step_id),
            WorkflowInstanceState::Approved(s) | WorkflowInstanceState::Rejected(s) => {
                Some(&s.current_step_id)
            }
            WorkflowInstanceState::Cancelled(s) => s.current_step_id.as_deref(),
            WorkflowInstanceState::ChangesRequested(s) => Some(&s.current_step_id),
            WorkflowInstanceState::Draft | WorkflowInstanceState::Pending(_) => None,
        }
    }

    pub fn initiated_by(&self) -> &UserId {
        &self.initiated_by
    }

    pub fn submitted_at(&self) -> Option<DateTime<Utc>> {
        match &self.state {
            WorkflowInstanceState::Draft => None,
            WorkflowInstanceState::Pending(s) => Some(s.submitted_at),
            WorkflowInstanceState::InProgress(s) => Some(s.submitted_at),
            WorkflowInstanceState::Approved(s) | WorkflowInstanceState::Rejected(s) => {
                Some(s.submitted_at)
            }
            WorkflowInstanceState::Cancelled(s) => s.submitted_at,
            WorkflowInstanceState::ChangesRequested(s) => Some(s.submitted_at),
        }
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        match &self.state {
            WorkflowInstanceState::Approved(s) | WorkflowInstanceState::Rejected(s) => {
                Some(s.completed_at)
            }
            WorkflowInstanceState::Cancelled(s) => Some(s.completed_at),
            _ => None,
        }
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// 状態への直接アクセス（パターンマッチ用）
    pub fn state(&self) -> &WorkflowInstanceState {
        &self.state
    }

    // ビジネスロジックメソッド

    /// インスタンスが編集可能かチェックする
    pub fn can_edit(&self) -> Result<(), DomainError> {
        match &self.state {
            WorkflowInstanceState::Draft => Ok(()),
            _ => Err(DomainError::Validation(
                "下書き状態でのみ編集可能です".to_string(),
            )),
        }
    }

    /// インスタンスを申請した新しいインスタンスを返す
    pub fn submitted(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
        match self.state {
            WorkflowInstanceState::Draft => Ok(Self {
                state: WorkflowInstanceState::Pending(PendingState { submitted_at: now }),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(
                "下書き状態でのみ申請可能です".to_string(),
            )),
        }
    }

    /// インスタンスを取り消した新しいインスタンスを返す
    pub fn cancelled(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
        match self.state {
            WorkflowInstanceState::Draft => Ok(Self {
                state: WorkflowInstanceState::Cancelled(CancelledState {
                    current_step_id: None,
                    submitted_at:    None,
                    completed_at:    now,
                }),
                updated_at: now,
                ..self
            }),
            WorkflowInstanceState::Pending(pending) => Ok(Self {
                state: WorkflowInstanceState::Cancelled(CancelledState {
                    current_step_id: None,
                    submitted_at:    Some(pending.submitted_at),
                    completed_at:    now,
                }),
                updated_at: now,
                ..self
            }),
            WorkflowInstanceState::InProgress(in_progress) => Ok(Self {
                state: WorkflowInstanceState::Cancelled(CancelledState {
                    current_step_id: Some(in_progress.current_step_id),
                    submitted_at:    Some(in_progress.submitted_at),
                    completed_at:    now,
                }),
                updated_at: now,
                ..self
            }),
            WorkflowInstanceState::ChangesRequested(changes) => Ok(Self {
                state: WorkflowInstanceState::Cancelled(CancelledState {
                    current_step_id: Some(changes.current_step_id),
                    submitted_at:    Some(changes.submitted_at),
                    completed_at:    now,
                }),
                updated_at: now,
                ..self
            }),
            WorkflowInstanceState::Approved(_)
            | WorkflowInstanceState::Rejected(_)
            | WorkflowInstanceState::Cancelled(_) => Err(DomainError::Validation(
                "完了済みのワークフローは取り消せません".to_string(),
            )),
        }
    }

    /// 現在のステップを更新した新しいインスタンスを返す
    ///
    /// Submit 時の初期遷移（Pending → InProgress）用。
    /// 承認後の次ステップ遷移には `advance_to_next_step` を使用する。
    pub fn with_current_step(self, step_id: String, now: DateTime<Utc>) -> Self {
        match self.state {
            WorkflowInstanceState::Pending(pending) => Self {
                state: WorkflowInstanceState::InProgress(InProgressState {
                    current_step_id: step_id,
                    submitted_at:    pending.submitted_at,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            },
            // 本来 Pending からのみ遷移すべきだが、後方互換のため他の状態でも動作する
            _ => Self {
                state: WorkflowInstanceState::InProgress(InProgressState {
                    current_step_id: step_id,
                    submitted_at:    self.submitted_at().unwrap_or(now),
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            },
        }
    }

    /// 次の承認ステップに遷移する
    ///
    /// InProgress 状態のインスタンスの current_step_id
    /// を次のステップに更新する。
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
        match self.state {
            WorkflowInstanceState::InProgress(in_progress) => Ok(Self {
                state: WorkflowInstanceState::InProgress(InProgressState {
                    current_step_id: next_step_id,
                    submitted_at:    in_progress.submitted_at,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "次ステップ遷移は処理中状態でのみ可能です（現在: {}）",
                self.status()
            ))),
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
        match self.state {
            WorkflowInstanceState::InProgress(in_progress) => Ok(Self {
                state: WorkflowInstanceState::Approved(CompletedState {
                    current_step_id: in_progress.current_step_id,
                    submitted_at:    in_progress.submitted_at,
                    completed_at:    now,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "承認完了は処理中状態でのみ可能です（現在: {}）",
                self.status()
            ))),
        }
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
        match self.state {
            WorkflowInstanceState::InProgress(in_progress) => Ok(Self {
                state: WorkflowInstanceState::ChangesRequested(ChangesRequestedState {
                    current_step_id: in_progress.current_step_id,
                    submitted_at:    in_progress.submitted_at,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "差し戻しは処理中状態でのみ可能です（現在: {}）",
                self.status()
            ))),
        }
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
        match self.state {
            WorkflowInstanceState::ChangesRequested(changes) => Ok(Self {
                state: WorkflowInstanceState::InProgress(InProgressState {
                    current_step_id: step_id,
                    submitted_at:    changes.submitted_at,
                }),
                form_data,
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "再申請は要修正状態でのみ可能です（現在: {}）",
                self.status()
            ))),
        }
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
        match self.state {
            WorkflowInstanceState::InProgress(in_progress) => Ok(Self {
                state: WorkflowInstanceState::Rejected(CompletedState {
                    current_step_id: in_progress.current_step_id,
                    submitted_at:    in_progress.submitted_at,
                    completed_at:    now,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "却下完了は処理中状態でのみ可能です（現在: {}）",
                self.status()
            ))),
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

        /// WorkflowInstance の getter から WorkflowInstanceRecord を構築するヘルパー。
        /// 構造体更新構文 `..record_from(&instance)` と組み合わせて、
        /// テストで差異のあるフィールドだけを指定するために使用する。
        fn record_from(instance: &WorkflowInstance) -> WorkflowInstanceRecord {
            WorkflowInstanceRecord {
                id: instance.id().clone(),
                tenant_id: instance.tenant_id().clone(),
                definition_id: instance.definition_id().clone(),
                definition_version: instance.definition_version(),
                display_number: instance.display_number(),
                title: instance.title().to_string(),
                form_data: instance.form_data().clone(),
                status: instance.status(),
                version: instance.version(),
                current_step_id: instance.current_step_id().map(String::from),
                initiated_by: instance.initiated_by().clone(),
                submitted_at: instance.submitted_at(),
                completed_at: instance.completed_at(),
                created_at: instance.created_at(),
                updated_at: instance.updated_at(),
            }
        }

        #[rstest]
        fn test_新規作成の初期状態(test_instance: WorkflowInstance) {
            let expected = WorkflowInstance::from_db(record_from(&test_instance)).unwrap();
            assert_eq!(test_instance, expected);
        }

        #[rstest]
        fn test_申請後の状態(test_instance: WorkflowInstance, now: DateTime<Utc>) {
            let before = test_instance.clone();
            let sut = test_instance.submitted(now).unwrap();

            let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::Pending,
                submitted_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
                status: WorkflowInstanceStatus::Approved,
                version: before.version().next(),
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
                status: WorkflowInstanceStatus::Rejected,
                version: before.version().next(),
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
                status: WorkflowInstanceStatus::Cancelled,
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
                status: WorkflowInstanceStatus::Cancelled,
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
                status: WorkflowInstanceStatus::Cancelled,
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
                version: before.version().next(),
                current_step_id: Some("step_2".to_string()),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
        fn test_差し戻し完了後の状態(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            let instance = test_instance
                .submitted(now)
                .unwrap()
                .with_current_step("step_1".to_string(), now);
            let before = instance.clone();

            let sut = instance.complete_with_request_changes(now).unwrap();

            let expected = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::ChangesRequested,
                version: before.version().next(),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
                form_data: new_form_data,
                status: WorkflowInstanceStatus::InProgress,
                version: before.version().next(),
                current_step_id: Some("new_step_1".to_string()),
                completed_at: None,
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
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
                status: WorkflowInstanceStatus::Cancelled,
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        // --- from_db() 不変条件バリデーション ---

        #[rstest]
        fn test_from_db_pendingでsubmitted_at欠損はエラー(test_instance: WorkflowInstance) {
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::Pending,
                submitted_at: None,
                ..record_from(&test_instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_in_progressでcurrent_step_id欠損はエラー(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            let instance = test_instance.submitted(now).unwrap();
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::InProgress,
                current_step_id: None,
                ..record_from(&instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_in_progressでsubmitted_at欠損はエラー(
            test_instance: WorkflowInstance,
        ) {
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::InProgress,
                current_step_id: Some("step_1".to_string()),
                submitted_at: None,
                ..record_from(&test_instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_approvedでcompleted_at欠損はエラー(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            let instance = test_instance
                .submitted(now)
                .unwrap()
                .with_current_step("step_1".to_string(), now);
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::Approved,
                completed_at: None,
                ..record_from(&instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_approvedでsubmitted_at欠損はエラー(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::Approved,
                current_step_id: Some("step_1".to_string()),
                submitted_at: None,
                completed_at: Some(now),
                ..record_from(&test_instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_approvedでcurrent_step_id欠損はエラー(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::Approved,
                current_step_id: None,
                submitted_at: Some(now),
                completed_at: Some(now),
                ..record_from(&test_instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_rejectedでcompleted_at欠損はエラー(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            let instance = test_instance
                .submitted(now)
                .unwrap()
                .with_current_step("step_1".to_string(), now);
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::Rejected,
                completed_at: None,
                ..record_from(&instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_changes_requestedでcurrent_step_id欠損はエラー(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            let instance = test_instance.submitted(now).unwrap();
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::ChangesRequested,
                current_step_id: None,
                ..record_from(&instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_changes_requestedでsubmitted_at欠損はエラー(
            test_instance: WorkflowInstance,
        ) {
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::ChangesRequested,
                current_step_id: Some("step_1".to_string()),
                submitted_at: None,
                ..record_from(&test_instance)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_cancelledでcompleted_at欠損はエラー(
            test_instance: WorkflowInstance,
        ) {
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::Cancelled,
                completed_at: None,
                ..record_from(&test_instance)
            });

            assert!(result.is_err());
        }
    }
}
