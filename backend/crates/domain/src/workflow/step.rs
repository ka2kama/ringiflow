//! # ワークフローステップ
//!
//! ワークフローインスタンス内の個々の承認タスクを管理する。
//! 担当者への割り当てと判断結果を保持し、承認・却下の状態遷移を持つ。
//!
//! 状態遷移は ADT（代数的データ型）で表現し、不正な状態を型レベルで防止する。
//! 詳細: [ADR-054: ADT ベースステートマシンパターンの標準化](../../docs/05_ADR/054_ADTベースステートマシンパターンの標準化.md)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use super::instance::WorkflowInstanceId;
use crate::{
    DomainError,
    user::UserId,
    value_objects::{DisplayNumber, Version},
};

define_uuid_id! {
    /// ワークフローステップ ID
    pub struct WorkflowStepId;
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

/// ワークフローステップの状態（ADT ベースステートマシン）
///
/// 各状態で有効なフィールドのみを持たせることで、不正な状態を型レベルで防止する。
/// 詳細: [ADR-054](../../docs/05_ADR/054_ADTベースステートマシンパターンの標準化.md), [エンティティ影響マップ](../../docs/03_詳細設計書/エンティティ影響マップ/WorkflowStep.md) INV-S2〜S4
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowStepState {
    /// 待機中
    Pending,
    /// アクティブ（処理中）
    Active(ActiveStepState),
    /// 完了
    Completed(CompletedStepState),
    /// スキップ
    Skipped,
}

/// Active 状態の固有フィールド
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveStepState {
    /// 開始日時（INV-S4 を型で強制）
    pub started_at: DateTime<Utc>,
}

/// Completed 状態の固有フィールド
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedStepState {
    /// 判断（INV-S2 を型で強制）
    pub decision:     StepDecision,
    /// コメント
    pub comment:      Option<String>,
    /// 開始日時（Active から引き継ぎ）
    pub started_at:   DateTime<Utc>,
    /// 完了日時（INV-S3 を型で強制）
    pub completed_at: DateTime<Utc>,
}

/// ワークフローステップエンティティ
///
/// ワークフローインスタンス内の個々の承認タスク。
/// 担当者への割り当てと判断結果を保持する。
///
/// [ADR-054](../../docs/05_ADR/054_ADTベースステートマシンパターンの標準化.md) Pattern A: 共通フィールドを外側に、状態固有フィールドを `state` enum に分離。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStep {
    id: WorkflowStepId,
    instance_id: WorkflowInstanceId,
    display_number: DisplayNumber,
    step_id: String,
    step_name: String,
    step_type: String,
    version: Version,
    assigned_to: Option<UserId>,
    due_date: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    state: WorkflowStepState,
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
///
/// DB スキーマのフラット構造を表現する。`from_db()` で不変条件を検証して ADT に変換する。
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
            version: Version::initial(),
            assigned_to: params.assigned_to,
            due_date: None,
            created_at: params.now,
            updated_at: params.now,
            state: WorkflowStepState::Pending,
        }
    }

    /// 既存のデータから復元する
    ///
    /// DB のフラット構造から ADT に変換し、不変条件（INV-S2〜S4）を検証する。
    ///
    /// # Errors
    ///
    /// - `DomainError::Validation`: 不変条件違反（例: Completed で decision が None）
    pub fn from_db(record: WorkflowStepRecord) -> Result<Self, DomainError> {
        let state = match record.status {
            WorkflowStepStatus::Pending => WorkflowStepState::Pending,
            WorkflowStepStatus::Active => {
                let started_at = record.started_at.ok_or_else(|| {
                    DomainError::Validation("Active ステップには started_at が必要です".to_string())
                })?;
                WorkflowStepState::Active(ActiveStepState { started_at })
            }
            WorkflowStepStatus::Completed => {
                let decision = record.decision.ok_or_else(|| {
                    DomainError::Validation(
                        "Completed ステップには decision が必要です".to_string(),
                    )
                })?;
                let started_at = record.started_at.ok_or_else(|| {
                    DomainError::Validation(
                        "Completed ステップには started_at が必要です".to_string(),
                    )
                })?;
                let completed_at = record.completed_at.ok_or_else(|| {
                    DomainError::Validation(
                        "Completed ステップには completed_at が必要です".to_string(),
                    )
                })?;
                WorkflowStepState::Completed(CompletedStepState {
                    decision,
                    comment: record.comment,
                    started_at,
                    completed_at,
                })
            }
            WorkflowStepStatus::Skipped => WorkflowStepState::Skipped,
        };

        Ok(Self {
            id: record.id,
            instance_id: record.instance_id,
            display_number: record.display_number,
            step_id: record.step_id,
            step_name: record.step_name,
            step_type: record.step_type,
            version: record.version,
            assigned_to: record.assigned_to,
            due_date: record.due_date,
            created_at: record.created_at,
            updated_at: record.updated_at,
            state,
        })
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
        match &self.state {
            WorkflowStepState::Pending => WorkflowStepStatus::Pending,
            WorkflowStepState::Active(_) => WorkflowStepStatus::Active,
            WorkflowStepState::Completed(_) => WorkflowStepStatus::Completed,
            WorkflowStepState::Skipped => WorkflowStepStatus::Skipped,
        }
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn assigned_to(&self) -> Option<&UserId> {
        self.assigned_to.as_ref()
    }

    pub fn decision(&self) -> Option<StepDecision> {
        match &self.state {
            WorkflowStepState::Completed(c) => Some(c.decision),
            _ => None,
        }
    }

    pub fn comment(&self) -> Option<&str> {
        match &self.state {
            WorkflowStepState::Completed(c) => c.comment.as_deref(),
            _ => None,
        }
    }

    pub fn due_date(&self) -> Option<DateTime<Utc>> {
        self.due_date
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        match &self.state {
            WorkflowStepState::Active(a) => Some(a.started_at),
            WorkflowStepState::Completed(c) => Some(c.started_at),
            _ => None,
        }
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        match &self.state {
            WorkflowStepState::Completed(c) => Some(c.completed_at),
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
    pub fn state(&self) -> &WorkflowStepState {
        &self.state
    }

    // ビジネスロジックメソッド

    /// ステップをアクティブにした新しいインスタンスを返す
    pub fn activated(self, now: DateTime<Utc>) -> Self {
        Self {
            state: WorkflowStepState::Active(ActiveStepState { started_at: now }),
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
        match self.state {
            WorkflowStepState::Active(active) => Ok(Self {
                state: WorkflowStepState::Completed(CompletedStepState {
                    decision,
                    comment,
                    started_at: active.started_at,
                    completed_at: now,
                }),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(
                "アクティブ状態でのみ完了できます".to_string(),
            )),
        }
    }

    /// ステップをスキップした新しいインスタンスを返す
    ///
    /// Pending 状態のステップのみスキップ可能。
    /// 却下時に残りの待機中ステップをスキップするために使用する。
    ///
    /// # Errors
    ///
    /// - `DomainError::Validation`: Pending 以外の状態で呼び出した場合
    pub fn skipped(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
        match self.state {
            WorkflowStepState::Pending => Ok(Self {
                state: WorkflowStepState::Skipped,
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "スキップは待機中状態でのみ可能です（現在: {}）",
                self.status()
            ))),
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
        match self.state {
            WorkflowStepState::Active(active) => Ok(Self {
                state: WorkflowStepState::Completed(CompletedStepState {
                    decision: StepDecision::Approved,
                    comment,
                    started_at: active.started_at,
                    completed_at: now,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "承認はアクティブ状態でのみ可能です（現在: {}）",
                self.status()
            ))),
        }
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
        match self.state {
            WorkflowStepState::Active(active) => Ok(Self {
                state: WorkflowStepState::Completed(CompletedStepState {
                    decision: StepDecision::Rejected,
                    comment,
                    started_at: active.started_at,
                    completed_at: now,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "却下はアクティブ状態でのみ可能です（現在: {}）",
                self.status()
            ))),
        }
    }

    /// ステップを差し戻す
    ///
    /// Active 状態のステップを Completed (RequestChanges) に遷移させる。
    /// version をインクリメントして楽観的ロックに対応。
    ///
    /// # Errors
    ///
    /// - `DomainError::Validation`: Active 以外の状態で呼び出した場合
    pub fn request_changes(
        self,
        comment: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        match self.state {
            WorkflowStepState::Active(active) => Ok(Self {
                state: WorkflowStepState::Completed(CompletedStepState {
                    decision: StepDecision::RequestChanges,
                    comment,
                    started_at: active.started_at,
                    completed_at: now,
                }),
                version: self.version.next(),
                updated_at: now,
                ..self
            }),
            _ => Err(DomainError::Validation(format!(
                "差し戻しはアクティブ状態でのみ可能です（現在: {}）",
                self.status()
            ))),
        }
    }

    /// ステップが期限切れかチェックする
    pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
        if let Some(due) = self.due_date
            && self.completed_at().is_none()
        {
            return now > due;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

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

    mod workflow_step {
        use pretty_assertions::assert_eq;

        use super::*;

        /// WorkflowStep の getter から WorkflowStepRecord を構築するヘルパー。
        /// 構造体更新構文 `..record_from(&step)` と組み合わせて、
        /// テストで差異のあるフィールドだけを指定するために使用する。
        fn record_from(step: &WorkflowStep) -> WorkflowStepRecord {
            WorkflowStepRecord {
                id: step.id().clone(),
                instance_id: step.instance_id().clone(),
                display_number: step.display_number(),
                step_id: step.step_id().to_string(),
                step_name: step.step_name().to_string(),
                step_type: step.step_type().to_string(),
                status: step.status(),
                version: step.version(),
                assigned_to: step.assigned_to().cloned(),
                decision: step.decision(),
                comment: step.comment().map(String::from),
                due_date: step.due_date(),
                started_at: step.started_at(),
                completed_at: step.completed_at(),
                created_at: step.created_at(),
                updated_at: step.updated_at(),
            }
        }

        #[rstest]
        fn test_新規作成の初期状態(test_step: WorkflowStep) {
            let expected = WorkflowStep::from_db(record_from(&test_step)).unwrap();
            assert_eq!(test_step, expected);
        }

        #[rstest]
        fn test_アクティブ化後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
            let before = test_step.clone();
            let sut = test_step.activated(now);

            let expected = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Active,
                started_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        #[rstest]
        fn test_承認後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
            let step = test_step.activated(now);
            let before = step.clone();

            let sut = step.approve(None, now).unwrap();

            let expected = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Completed,
                version: before.version().next(),
                decision: Some(StepDecision::Approved),
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        #[rstest]
        fn test_コメント付き承認後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
            let step = test_step.activated(now);
            let before = step.clone();

            let sut = step.approve(Some("承認します".to_string()), now).unwrap();

            let expected = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Completed,
                version: before.version().next(),
                decision: Some(StepDecision::Approved),
                comment: Some("承認します".to_string()),
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        #[rstest]
        fn test_却下後の状態(test_step: WorkflowStep, now: DateTime<Utc>) {
            let step = test_step.activated(now);
            let before = step.clone();

            let sut = step.reject(None, now).unwrap();

            let expected = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Completed,
                version: before.version().next(),
                decision: Some(StepDecision::Rejected),
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        #[rstest]
        fn test_is_overdue_期限切れの場合trueを返す(
            test_step: WorkflowStep,
            now: DateTime<Utc>,
        ) {
            let past = DateTime::from_timestamp(1_699_999_000, 0).unwrap();
            let step = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Active,
                due_date: Some(past),
                started_at: Some(past),
                created_at: past,
                updated_at: past,
                ..record_from(&test_step)
            })
            .unwrap();
            assert!(step.is_overdue(now));
        }

        #[rstest]
        fn test_is_overdue_期限内の場合falseを返す(
            test_step: WorkflowStep,
            now: DateTime<Utc>,
        ) {
            let future = DateTime::from_timestamp(1_700_100_000, 0).unwrap();
            let step = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Active,
                due_date: Some(future),
                started_at: Some(now),
                ..record_from(&test_step)
            })
            .unwrap();
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

        // --- skipped() テスト ---

        #[rstest]
        fn test_スキップ_待機中から成功(test_step: WorkflowStep, now: DateTime<Utc>) {
            let before = test_step.clone();

            let sut = test_step.skipped(now).unwrap();

            let expected = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Skipped,
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        #[rstest]
        fn test_スキップ_待機中以外ではエラー(
            test_step: WorkflowStep,
            now: DateTime<Utc>,
        ) {
            let step = test_step.activated(now);

            let result = step.skipped(now);

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
                status: WorkflowStepStatus::Completed,
                decision: Some(StepDecision::RequestChanges),
                comment: Some("修正してください".to_string()),
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        // --- request_changes() テスト ---

        #[rstest]
        fn test_差し戻しステップの状態(test_step: WorkflowStep, now: DateTime<Utc>) {
            let step = test_step.activated(now);
            let before = step.clone();

            let sut = step.request_changes(None, now).unwrap();

            let expected = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Completed,
                version: before.version().next(),
                decision: Some(StepDecision::RequestChanges),
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        #[rstest]
        fn test_コメント付き差し戻しステップの状態(
            test_step: WorkflowStep,
            now: DateTime<Utc>,
        ) {
            let step = test_step.activated(now);
            let before = step.clone();

            let sut = step
                .request_changes(Some("金額を修正してください".to_string()), now)
                .unwrap();

            let expected = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Completed,
                version: before.version().next(),
                decision: Some(StepDecision::RequestChanges),
                comment: Some("金額を修正してください".to_string()),
                completed_at: Some(now),
                updated_at: now,
                ..record_from(&before)
            })
            .unwrap();
            assert_eq!(sut, expected);
        }

        #[rstest]
        fn test_アクティブ以外で差し戻しするとエラー(
            test_step: WorkflowStep,
            now: DateTime<Utc>,
        ) {
            // Pending 状態からは差し戻し不可
            let result = test_step.request_changes(None, now);

            assert!(result.is_err());
        }

        // --- from_db() 不変条件バリデーション ---

        #[rstest]
        fn test_from_db_completedでdecision欠損はエラー(
            test_step: WorkflowStep,
            now: DateTime<Utc>,
        ) {
            let step = test_step.activated(now);
            let result = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Completed,
                decision: None,
                completed_at: Some(now),
                ..record_from(&step)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_completedでcompleted_at欠損はエラー(
            test_step: WorkflowStep,
            now: DateTime<Utc>,
        ) {
            let step = test_step.activated(now);
            let result = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Completed,
                decision: Some(StepDecision::Approved),
                completed_at: None,
                ..record_from(&step)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_activeでstarted_at欠損はエラー(test_step: WorkflowStep) {
            let result = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Active,
                started_at: None,
                ..record_from(&test_step)
            });

            assert!(result.is_err());
        }

        #[rstest]
        fn test_from_db_completedでstarted_at欠損はエラー(
            test_step: WorkflowStep,
            now: DateTime<Utc>,
        ) {
            let result = WorkflowStep::from_db(WorkflowStepRecord {
                status: WorkflowStepStatus::Completed,
                decision: Some(StepDecision::Approved),
                started_at: None,
                completed_at: Some(now),
                ..record_from(&test_step)
            });

            assert!(result.is_err());
        }
    }
}
