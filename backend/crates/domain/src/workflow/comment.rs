//! # ワークフローコメント
//!
//! ワークフローインスタンスに対するコメントスレッドを管理する。
//! 承認プロセス中に申請者と承認者がコメントでやり取りするために使用する。
//!
//! ## 既存の `workflow_steps.comment` との違い
//!
//! - `workflow_steps.comment`: ステップの判定コメント（承認/却下時に入力）
//! - `workflow_comments`: ワークフロー単位のコメントスレッド（自由なやり取り）

use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::instance::WorkflowInstanceId;
use crate::{DomainError, tenant::TenantId, user::UserId};

/// ワークフローコメント ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
pub struct WorkflowCommentId(Uuid);

impl WorkflowCommentId {
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

impl Default for WorkflowCommentId {
    fn default() -> Self {
        Self::new()
    }
}

/// コメント本文
///
/// 1〜2,000 文字のバリデーションを型レベルで強制する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentBody(String);

/// コメント本文の最大文字数
const COMMENT_BODY_MAX_LENGTH: usize = 2000;

impl CommentBody {
    /// コメント本文を作成する
    ///
    /// # Errors
    ///
    /// - 空文字列の場合
    /// - 2,000 文字を超える場合
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.is_empty() {
            return Err(DomainError::Validation(
                "コメント本文は必須です".to_string(),
            ));
        }
        if value.chars().count() > COMMENT_BODY_MAX_LENGTH {
            return Err(DomainError::Validation(format!(
                "コメント本文は{}文字以内で入力してください",
                COMMENT_BODY_MAX_LENGTH
            )));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// ワークフローコメントエンティティ
///
/// ワークフローインスタンスに対するコメント。
/// 承認プロセス中の申請者と承認者のやり取りを記録する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowComment {
    id:          WorkflowCommentId,
    tenant_id:   TenantId,
    instance_id: WorkflowInstanceId,
    posted_by:   UserId,
    body:        CommentBody,
    created_at:  DateTime<Utc>,
    updated_at:  DateTime<Utc>,
}

/// ワークフローコメントの新規作成パラメータ
pub struct NewWorkflowComment {
    pub id:          WorkflowCommentId,
    pub tenant_id:   TenantId,
    pub instance_id: WorkflowInstanceId,
    pub posted_by:   UserId,
    pub body:        CommentBody,
    pub now:         DateTime<Utc>,
}

/// ワークフローコメントの DB 復元パラメータ
pub struct WorkflowCommentRecord {
    pub id:          WorkflowCommentId,
    pub tenant_id:   TenantId,
    pub instance_id: WorkflowInstanceId,
    pub posted_by:   UserId,
    pub body:        CommentBody,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

impl WorkflowComment {
    /// 新しいワークフローコメントを作成する
    pub fn new(params: NewWorkflowComment) -> Self {
        Self {
            id:          params.id,
            tenant_id:   params.tenant_id,
            instance_id: params.instance_id,
            posted_by:   params.posted_by,
            body:        params.body,
            created_at:  params.now,
            updated_at:  params.now,
        }
    }

    /// 既存のデータから復元する
    pub fn from_db(record: WorkflowCommentRecord) -> Self {
        Self {
            id:          record.id,
            tenant_id:   record.tenant_id,
            instance_id: record.instance_id,
            posted_by:   record.posted_by,
            body:        record.body,
            created_at:  record.created_at,
            updated_at:  record.updated_at,
        }
    }

    // Getter メソッド

    pub fn id(&self) -> &WorkflowCommentId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn instance_id(&self) -> &WorkflowInstanceId {
        &self.instance_id
    }

    pub fn posted_by(&self) -> &UserId {
        &self.posted_by
    }

    pub fn body(&self) -> &CommentBody {
        &self.body
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
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

    mod comment_body {
        use super::*;

        #[rstest]
        fn test_1文字で成功() {
            let result = CommentBody::new("a");
            assert!(result.is_ok());
            assert_eq!(result.unwrap().as_str(), "a");
        }

        #[rstest]
        fn test_2000文字で成功() {
            let body: String = "あ".repeat(2000);
            let result = CommentBody::new(body.clone());
            assert!(result.is_ok());
            assert_eq!(result.unwrap().as_str(), body);
        }

        #[rstest]
        fn test_空文字列でエラー() {
            let result = CommentBody::new("");
            assert!(result.is_err());
        }

        #[rstest]
        fn test_2001文字でエラー() {
            let body: String = "あ".repeat(2001);
            let result = CommentBody::new(body);
            assert!(result.is_err());
        }
    }

    mod workflow_comment {
        use pretty_assertions::assert_eq;

        use super::*;

        #[rstest]
        fn test_新規作成の初期状態(now: DateTime<Utc>) {
            let id = WorkflowCommentId::new();
            let tenant_id = TenantId::new();
            let instance_id = WorkflowInstanceId::new();
            let posted_by = UserId::new();
            let body = CommentBody::new("テストコメント").unwrap();

            let sut = WorkflowComment::new(NewWorkflowComment {
                id: id.clone(),
                tenant_id: tenant_id.clone(),
                instance_id: instance_id.clone(),
                posted_by: posted_by.clone(),
                body: body.clone(),
                now,
            });

            let expected = WorkflowComment::from_db(WorkflowCommentRecord {
                id,
                tenant_id,
                instance_id,
                posted_by,
                body,
                created_at: now,
                updated_at: now,
            });
            assert_eq!(sut, expected);
        }
    }
}
