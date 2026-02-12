//! # WorkflowCommentRepository
//!
//! ワークフローコメントの永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **テナント分離**: すべてのクエリでテナント ID を考慮
//! - **時系列ソート**: コメント一覧は created_at ASC で返す
//! - **型安全なクエリ**: sqlx のコンパイル時検証を活用
//!
//! 詳細: [データベース設計](../../../../docs/03_詳細設計書/02_データベース設計.md)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   workflow::{
      CommentBody,
      WorkflowComment,
      WorkflowCommentId,
      WorkflowCommentRecord,
      WorkflowInstanceId,
   },
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::InfraError;

/// ワークフローコメントリポジトリトレイト
///
/// ワークフローコメントの永続化操作を定義する。
#[async_trait]
pub trait WorkflowCommentRepository: Send + Sync {
   /// 新規コメントを作成する
   async fn insert(
      &self,
      comment: &WorkflowComment,
      tenant_id: &TenantId,
   ) -> Result<(), InfraError>;

   /// インスタンス ID でコメント一覧を取得する（created_at ASC）
   async fn find_by_instance(
      &self,
      instance_id: &WorkflowInstanceId,
      tenant_id: &TenantId,
   ) -> Result<Vec<WorkflowComment>, InfraError>;
}

/// DB の workflow_comments テーブルの行を表す中間構造体
struct WorkflowCommentRow {
   id:          Uuid,
   tenant_id:   Uuid,
   instance_id: Uuid,
   posted_by:   Uuid,
   body:        String,
   created_at:  DateTime<Utc>,
   updated_at:  DateTime<Utc>,
}

impl TryFrom<WorkflowCommentRow> for WorkflowComment {
   type Error = InfraError;

   fn try_from(row: WorkflowCommentRow) -> Result<Self, Self::Error> {
      Ok(WorkflowComment::from_db(WorkflowCommentRecord {
         id:          WorkflowCommentId::from_uuid(row.id),
         tenant_id:   TenantId::from_uuid(row.tenant_id),
         instance_id: WorkflowInstanceId::from_uuid(row.instance_id),
         posted_by:   UserId::from_uuid(row.posted_by),
         body:        CommentBody::new(row.body)
            .map_err(|e| InfraError::Unexpected(e.to_string()))?,
         created_at:  row.created_at,
         updated_at:  row.updated_at,
      }))
   }
}

/// PostgreSQL 実装の WorkflowCommentRepository
#[derive(Debug, Clone)]
pub struct PostgresWorkflowCommentRepository {
   pool: PgPool,
}

impl PostgresWorkflowCommentRepository {
   /// 新しいリポジトリインスタンスを作成
   pub fn new(pool: PgPool) -> Self {
      Self { pool }
   }
}

#[async_trait]
impl WorkflowCommentRepository for PostgresWorkflowCommentRepository {
   async fn insert(
      &self,
      comment: &WorkflowComment,
      tenant_id: &TenantId,
   ) -> Result<(), InfraError> {
      sqlx::query!(
         r#"
            INSERT INTO workflow_comments (
                id, tenant_id, instance_id, posted_by, body,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
         comment.id().as_uuid(),
         tenant_id.as_uuid(),
         comment.instance_id().as_uuid(),
         comment.posted_by().as_uuid(),
         comment.body().as_str(),
         comment.created_at(),
         comment.updated_at()
      )
      .execute(&self.pool)
      .await?;

      Ok(())
   }

   async fn find_by_instance(
      &self,
      instance_id: &WorkflowInstanceId,
      tenant_id: &TenantId,
   ) -> Result<Vec<WorkflowComment>, InfraError> {
      let rows = sqlx::query_as!(
         WorkflowCommentRow,
         r#"
            SELECT
                id, tenant_id, instance_id, posted_by, body,
                created_at, updated_at
            FROM workflow_comments
            WHERE instance_id = $1 AND tenant_id = $2
            ORDER BY created_at ASC
            "#,
         instance_id.as_uuid(),
         tenant_id.as_uuid()
      )
      .fetch_all(&self.pool)
      .await?;

      rows.into_iter().map(WorkflowComment::try_from).collect()
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   /// トレイトオブジェクトとして使用できることを確認
   #[test]
   fn test_トレイトはsendとsyncを実装している() {
      fn assert_send_sync<T: Send + Sync>() {}
      assert_send_sync::<Box<dyn WorkflowCommentRepository>>();
   }
}
