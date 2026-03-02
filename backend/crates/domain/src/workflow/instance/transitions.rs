//! # ワークフローインスタンスの状態遷移
//!
//! 申請・承認・却下・差し戻し・再申請・取り消しの状態遷移メソッド。

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

use super::{
    WorkflowInstance,
    state::{
        CancelledState,
        ChangesRequestedState,
        CompletedState,
        InProgressState,
        PendingState,
        WorkflowInstanceState,
    },
};
use crate::DomainError;

impl WorkflowInstance {
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
                state: WorkflowInstanceState::Cancelled(CancelledState::FromDraft {
                    completed_at: now,
                }),
                updated_at: now,
                ..self
            }),
            WorkflowInstanceState::Pending(pending) => Ok(Self {
                state: WorkflowInstanceState::Cancelled(CancelledState::FromPending {
                    submitted_at: pending.submitted_at,
                    completed_at: now,
                }),
                updated_at: now,
                ..self
            }),
            WorkflowInstanceState::InProgress(in_progress) => Ok(Self {
                state: WorkflowInstanceState::Cancelled(CancelledState::FromActive {
                    current_step_id: in_progress.current_step_id,
                    submitted_at:    in_progress.submitted_at,
                    completed_at:    now,
                }),
                updated_at: now,
                ..self
            }),
            WorkflowInstanceState::ChangesRequested(changes) => Ok(Self {
                state: WorkflowInstanceState::Cancelled(CancelledState::FromActive {
                    current_step_id: changes.current_step_id,
                    submitted_at:    changes.submitted_at,
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
    pub fn with_current_step(
        self,
        step_id: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        match self.state {
            WorkflowInstanceState::Pending(pending) => Ok(Self {
                state: WorkflowInstanceState::InProgress(InProgressState {
                    current_step_id: step_id,
                    submitted_at:    pending.submitted_at,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "ステップ設定は承認待ち状態でのみ可能です（現在: {}）",
                self.status()
            ))),
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
