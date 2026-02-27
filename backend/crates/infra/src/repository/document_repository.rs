//! # DocumentRepository
//!
//! ドキュメントの永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **UploadContext 変換**: DB の nullable カラム + CHECK 制約から
//!   `UploadContext` enum へ変換（Repository 層の責務）
//! - **RLS 二重防御**: WHERE 句で明示的にテナント条件を指定
//!
//! 詳細: [ドキュメント管理設計](../../../../docs/03_詳細設計書/17_ドキュメント管理設計.md)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ringiflow_domain::{
    document::{Document, DocumentId, DocumentStatus, UploadContext},
    folder::FolderId,
    tenant::TenantId,
    user::UserId,
    workflow::WorkflowInstanceId,
};
use sqlx::PgPool;

use crate::error::InfraError;

/// ドキュメントリポジトリトレイト
///
/// ドキュメントの CRUD 操作とコンテキスト別の集計を定義する。
#[async_trait]
pub trait DocumentRepository: Send + Sync {
    /// ID でドキュメントを検索する
    async fn find_by_id(
        &self,
        id: &DocumentId,
        tenant_id: &TenantId,
    ) -> Result<Option<Document>, InfraError>;

    /// ドキュメントを挿入する
    async fn insert(&self, document: &Document) -> Result<(), InfraError>;

    /// ドキュメントのステータスを更新する
    async fn update_status(
        &self,
        id: &DocumentId,
        status: DocumentStatus,
        tenant_id: &TenantId,
        now: DateTime<Utc>,
    ) -> Result<(), InfraError>;

    /// フォルダ内のドキュメント数と合計サイズを取得する（deleted 除外）
    async fn count_and_total_size_by_folder(
        &self,
        folder_id: &FolderId,
        tenant_id: &TenantId,
    ) -> Result<(usize, i64), InfraError>;

    /// ワークフローインスタンスのドキュメント数と合計サイズを取得する（deleted 除外）
    async fn count_and_total_size_by_workflow(
        &self,
        workflow_instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<(usize, i64), InfraError>;

    /// ドキュメントをソフトデリートする（status, updated_at, deleted_at を更新）
    async fn soft_delete(
        &self,
        id: &DocumentId,
        tenant_id: &TenantId,
        now: DateTime<Utc>,
    ) -> Result<(), InfraError>;

    /// フォルダ内の active ドキュメント一覧を取得する
    async fn list_by_folder(
        &self,
        folder_id: &FolderId,
        tenant_id: &TenantId,
    ) -> Result<Vec<Document>, InfraError>;

    /// ワークフローインスタンスの active ドキュメント一覧を取得する
    async fn list_by_workflow(
        &self,
        workflow_instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Vec<Document>, InfraError>;
}

/// PostgreSQL 実装の DocumentRepository
#[derive(Debug, Clone)]
pub struct PostgresDocumentRepository {
    pool: PgPool,
}

impl PostgresDocumentRepository {
    /// 新しいリポジトリインスタンスを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// DB 行からドキュメントエンティティを構築する
///
/// `find_by_id` と `list_by_*` で共通の変換ロジック。
fn row_to_document(row: &DocumentRow) -> Result<Document, InfraError> {
    let upload_context = match (row.folder_id, row.workflow_instance_id) {
        (Some(fid), None) => UploadContext::Folder(FolderId::from_uuid(fid)),
        (None, Some(wid)) => UploadContext::Workflow(WorkflowInstanceId::from_uuid(wid)),
        // CHECK 制約が XOR を保証するため到達しない
        _ => {
            return Err(InfraError::Unexpected(
                "documents テーブルの folder_id/workflow_instance_id が不正な状態です".to_string(),
            ));
        }
    };

    let status: DocumentStatus = row
        .status
        .parse()
        .map_err(|e| InfraError::Unexpected(format!("DocumentStatus のパースに失敗: {}", e)))?;

    Ok(Document::from_db(
        DocumentId::from_uuid(row.id),
        TenantId::from_uuid(row.tenant_id),
        row.filename.clone(),
        row.content_type.clone(),
        row.size,
        row.s3_key.clone(),
        upload_context,
        status,
        row.uploaded_by.map(UserId::from_uuid),
        row.created_at,
        row.updated_at,
        row.deleted_at,
    ))
}

/// sqlx::query! が返す行の共通構造
///
/// `sqlx::query!` はクエリごとに匿名構造体を返すため、
/// ヘルパー関数で受け取るための中間型。
struct DocumentRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    filename: String,
    content_type: String,
    size: i64,
    s3_key: String,
    folder_id: Option<uuid::Uuid>,
    workflow_instance_id: Option<uuid::Uuid>,
    status: String,
    uploaded_by: Option<uuid::Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl DocumentRepository for PostgresDocumentRepository {
    #[tracing::instrument(skip_all, level = "debug", fields(%id, %tenant_id))]
    async fn find_by_id(
        &self,
        id: &DocumentId,
        tenant_id: &TenantId,
    ) -> Result<Option<Document>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id as "tenant_id!",
                filename as "filename!",
                content_type as "content_type!",
                size as "size!",
                s3_key as "s3_key!",
                folder_id,
                workflow_instance_id,
                status as "status!",
                uploaded_by,
                created_at as "created_at!",
                updated_at as "updated_at!",
                deleted_at
            FROM documents
            WHERE id = $1 AND tenant_id = $2
            "#,
            id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let doc_row = DocumentRow {
            id: row.id,
            tenant_id: row.tenant_id,
            filename: row.filename,
            content_type: row.content_type,
            size: row.size,
            s3_key: row.s3_key,
            folder_id: row.folder_id,
            workflow_instance_id: row.workflow_instance_id,
            status: row.status,
            uploaded_by: row.uploaded_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
        };

        Ok(Some(row_to_document(&doc_row)?))
    }

    #[tracing::instrument(skip_all, level = "debug")]
    async fn insert(&self, document: &Document) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            INSERT INTO documents (
                id, tenant_id, filename, content_type, size, s3_key,
                folder_id, workflow_instance_id, status, uploaded_by,
                created_at, updated_at, deleted_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            document.id().as_uuid(),
            document.tenant_id().as_uuid(),
            document.filename(),
            document.content_type(),
            document.size(),
            document.s3_key(),
            document
                .upload_context()
                .folder_id()
                .map(|id| *id.as_uuid()),
            document
                .upload_context()
                .workflow_instance_id()
                .map(|id| *id.as_uuid()),
            <DocumentStatus as Into<&str>>::into(document.status()),
            document.uploaded_by().map(|id| *id.as_uuid()),
            document.created_at(),
            document.updated_at(),
            document.deleted_at()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%id, %tenant_id))]
    async fn update_status(
        &self,
        id: &DocumentId,
        status: DocumentStatus,
        tenant_id: &TenantId,
        now: DateTime<Utc>,
    ) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            UPDATE documents
            SET status = $2, updated_at = $3
            WHERE id = $1 AND tenant_id = $4
            "#,
            id.as_uuid(),
            <DocumentStatus as Into<&str>>::into(status),
            now,
            tenant_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%folder_id, %tenant_id))]
    async fn count_and_total_size_by_folder(
        &self,
        folder_id: &FolderId,
        tenant_id: &TenantId,
    ) -> Result<(usize, i64), InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                COUNT(*)::bigint as "count!",
                COALESCE(SUM(size), 0)::bigint as "total_size!"
            FROM documents
            WHERE folder_id = $1 AND tenant_id = $2 AND status != 'deleted'
            "#,
            folder_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((row.count as usize, row.total_size))
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%workflow_instance_id, %tenant_id))]
    async fn count_and_total_size_by_workflow(
        &self,
        workflow_instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<(usize, i64), InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                COUNT(*)::bigint as "count!",
                COALESCE(SUM(size), 0)::bigint as "total_size!"
            FROM documents
            WHERE workflow_instance_id = $1 AND tenant_id = $2 AND status != 'deleted'
            "#,
            workflow_instance_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((row.count as usize, row.total_size))
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%id, %tenant_id))]
    async fn soft_delete(
        &self,
        id: &DocumentId,
        tenant_id: &TenantId,
        now: DateTime<Utc>,
    ) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            UPDATE documents
            SET status = 'deleted', updated_at = $2, deleted_at = $3
            WHERE id = $1 AND tenant_id = $4
            "#,
            id.as_uuid(),
            now,
            now,
            tenant_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%folder_id, %tenant_id))]
    async fn list_by_folder(
        &self,
        folder_id: &FolderId,
        tenant_id: &TenantId,
    ) -> Result<Vec<Document>, InfraError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id as "tenant_id!",
                filename as "filename!",
                content_type as "content_type!",
                size as "size!",
                s3_key as "s3_key!",
                folder_id,
                workflow_instance_id,
                status as "status!",
                uploaded_by,
                created_at as "created_at!",
                updated_at as "updated_at!",
                deleted_at
            FROM documents
            WHERE folder_id = $1 AND tenant_id = $2 AND status = 'active'
            ORDER BY created_at DESC, id ASC
            "#,
            folder_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let doc_row = DocumentRow {
                    id: row.id,
                    tenant_id: row.tenant_id,
                    filename: row.filename.clone(),
                    content_type: row.content_type.clone(),
                    size: row.size,
                    s3_key: row.s3_key.clone(),
                    folder_id: row.folder_id,
                    workflow_instance_id: row.workflow_instance_id,
                    status: row.status.clone(),
                    uploaded_by: row.uploaded_by,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    deleted_at: row.deleted_at,
                };
                row_to_document(&doc_row)
            })
            .collect()
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%workflow_instance_id, %tenant_id))]
    async fn list_by_workflow(
        &self,
        workflow_instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Vec<Document>, InfraError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id as "tenant_id!",
                filename as "filename!",
                content_type as "content_type!",
                size as "size!",
                s3_key as "s3_key!",
                folder_id,
                workflow_instance_id,
                status as "status!",
                uploaded_by,
                created_at as "created_at!",
                updated_at as "updated_at!",
                deleted_at
            FROM documents
            WHERE workflow_instance_id = $1 AND tenant_id = $2 AND status = 'active'
            ORDER BY created_at DESC, id ASC
            "#,
            workflow_instance_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let doc_row = DocumentRow {
                    id: row.id,
                    tenant_id: row.tenant_id,
                    filename: row.filename.clone(),
                    content_type: row.content_type.clone(),
                    size: row.size,
                    s3_key: row.s3_key.clone(),
                    folder_id: row.folder_id,
                    workflow_instance_id: row.workflow_instance_id,
                    status: row.status.clone(),
                    uploaded_by: row.uploaded_by,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    deleted_at: row.deleted_at,
                };
                row_to_document(&doc_row)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PostgresDocumentRepository>();
    }
}
