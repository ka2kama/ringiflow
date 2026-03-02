//! # ワークフローインスタンスの状態型
//!
//! 型安全ステートマシンの各状態を定義する。
//! 詳細: [ADR-054](../../../docs/70_ADR/054_型安全ステートマシンパターンの標準化.md)

use chrono::{DateTime, Utc};

/// ワークフローインスタンスの状態（型安全ステートマシン）
///
/// 各状態で有効なフィールドのみを持たせることで、不正な状態を型レベルで防止する。
/// 詳細: [ADR-054](../../../docs/70_ADR/054_型安全ステートマシンパターンの標準化.md),
/// [エンティティ影響マップ](../../../docs/40_詳細設計書/エンティティ影響マップ/WorkflowInstance.md) INV-I1〜I9
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

/// 取り消し状態
///
/// 遷移元に応じて保持するフィールドが異なる。
/// ADT で遷移元ごとのバリアントに分離し、不正なフィールド組み合わせを型レベルで防止する。
///
/// - FromDraft: 申請前のため submitted_at, current_step_id なし
/// - FromPending: 申請済みだがステップ未開始のため current_step_id なし
/// - FromActive: InProgress/ChangesRequested から取り消し（ステップ処理中）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelledState {
    /// Draft から取り消し
    FromDraft {
        /// 完了日時（INV-I9 を型で強制）
        completed_at: DateTime<Utc>,
    },
    /// Pending から取り消し
    FromPending {
        /// 申請日時
        submitted_at: DateTime<Utc>,
        /// 完了日時（INV-I9 を型で強制）
        completed_at: DateTime<Utc>,
    },
    /// InProgress/ChangesRequested から取り消し
    FromActive {
        /// 現在のステップ ID
        current_step_id: String,
        /// 申請日時
        submitted_at:    DateTime<Utc>,
        /// 完了日時（INV-I9 を型で強制）
        completed_at:    DateTime<Utc>,
    },
}

impl CancelledState {
    pub fn completed_at(&self) -> DateTime<Utc> {
        match self {
            Self::FromDraft { completed_at, .. }
            | Self::FromPending { completed_at, .. }
            | Self::FromActive { completed_at, .. } => *completed_at,
        }
    }

    pub fn current_step_id(&self) -> Option<&str> {
        match self {
            Self::FromActive {
                current_step_id, ..
            } => Some(current_step_id),
            Self::FromDraft { .. } | Self::FromPending { .. } => None,
        }
    }

    pub fn submitted_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::FromPending { submitted_at, .. } | Self::FromActive { submitted_at, .. } => {
                Some(*submitted_at)
            }
            Self::FromDraft { .. } => None,
        }
    }
}

/// ChangesRequested 状態の固有フィールド
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesRequestedState {
    /// 現在のステップ ID（INV-I8 を型で強制）
    pub current_step_id: String,
    /// 申請日時（INV-I8 を型で強制）
    pub submitted_at:    DateTime<Utc>,
}
