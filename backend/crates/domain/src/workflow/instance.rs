//! # ワークフローインスタンス
//!
//! ワークフロー定義から生成された実行中の案件を管理する。
//! フォームデータと進捗状態を保持し、申請・承認・却下のライフサイクルを持つ。
//!
//! 状態遷移は型安全ステートマシンで管理し、不正な状態を型レベルで防止する。
//! 詳細: [ADR-054: 型安全ステートマシンパターンの標準化](../../docs/70_ADR/054_型安全ステートマシンパターンの標準化.md)
//!
//! ## モジュール構成
//!
//! - `state`: 型安全ステートマシンの各状態型
//! - `transitions`: 状態遷移メソッド
//! - `record`: DB レコード変換

mod record;
mod state;
mod transitions;

use chrono::{DateTime, Utc};
pub use record::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
pub use state::*;
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

/// ワークフローインスタンスエンティティ
///
/// 定義から生成された実行中の申請案件。
/// フォームデータと進捗状態を保持する。
///
/// [ADR-054](../../docs/70_ADR/054_型安全ステートマシンパターンの標準化.md) Pattern A:
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
            WorkflowInstanceState::Cancelled(s) => s.current_step_id(),
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
            WorkflowInstanceState::Cancelled(s) => s.submitted_at(),
            WorkflowInstanceState::ChangesRequested(s) => Some(s.submitted_at),
        }
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        match &self.state {
            WorkflowInstanceState::Approved(s) | WorkflowInstanceState::Rejected(s) => {
                Some(s.completed_at)
            }
            WorkflowInstanceState::Cancelled(s) => Some(s.completed_at()),
            WorkflowInstanceState::Draft
            | WorkflowInstanceState::Pending(_)
            | WorkflowInstanceState::InProgress(_)
            | WorkflowInstanceState::ChangesRequested(_) => None,
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
                .with_current_step("step_1".to_string(), now)
                .unwrap();
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
                .with_current_step("step_1".to_string(), now)
                .unwrap();
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
                .with_current_step("step_1".to_string(), now)
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

        #[rstest]
        fn test_承認済みからの取消はエラー(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            let instance = test_instance
                .submitted(now)
                .unwrap()
                .with_current_step("step_1".to_string(), now)
                .unwrap()
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
                .unwrap()
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
                .with_current_step("step_1".to_string(), now)
                .unwrap();
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
                .with_current_step("step_1".to_string(), now)
                .unwrap();
            let before_version = instance.version();

            let sut = instance
                .advance_to_next_step("step_2".to_string(), now)
                .unwrap();

            assert_eq!(sut.version(), before_version.next());
        }

        // --- with_current_step() 異常系テスト ---

        #[rstest]
        fn test_下書きからのステップ設定はエラー(
            test_instance: WorkflowInstance,
            now: DateTime<Utc>,
        ) {
            // Draft 状態からはステップ設定不可
            let result = test_instance.with_current_step("step_1".to_string(), now);

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
                .with_current_step("step_1".to_string(), now)
                .unwrap();

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
                .with_current_step("step_1".to_string(), now)
                .unwrap();
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
                .unwrap()
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
                .with_current_step("step_1".to_string(), now)
                .unwrap();

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
                .unwrap()
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
                .with_current_step("step_1".to_string(), now)
                .unwrap();
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
                .with_current_step("step_1".to_string(), now)
                .unwrap();
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

        #[rstest]
        fn test_from_db_cancelledでcurrent_step_idありsubmitted_atなしはエラー(
            test_instance: WorkflowInstance,
        ) {
            let result = WorkflowInstance::from_db(WorkflowInstanceRecord {
                status: WorkflowInstanceStatus::Cancelled,
                current_step_id: Some("step_1".to_string()),
                submitted_at: None,
                completed_at: Some(Utc::now()),
                ..record_from(&test_instance)
            });

            assert!(result.is_err());
        }
    }
}
