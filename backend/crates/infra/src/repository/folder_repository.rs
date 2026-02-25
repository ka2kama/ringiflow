//! # FolderRepository
//!
//! フォルダの永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **materialized path パターン**: サブツリーの path 一括更新を
//!   `update_subtree_paths` メソッドで提供
//! - **RLS 二重防御**: WHERE 句で明示的にテナント条件を指定
//!
//! 詳細: [ドキュメント管理設計](../../../../docs/03_詳細設計書/17_ドキュメント管理設計.md)

use async_trait::async_trait;
use ringiflow_domain::{
    folder::{Folder, FolderId, FolderName},
    tenant::TenantId,
    user::UserId,
};
use sqlx::PgPool;

use crate::error::InfraError;

/// フォルダリポジトリトレイト
///
/// フォルダの CRUD 操作と materialized path のサブツリー更新を定義する。
#[async_trait]
pub trait FolderRepository: Send + Sync {
    /// テナント内の全フォルダを path 順で取得する
    async fn find_all_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Folder>, InfraError>;

    /// ID でフォルダを検索する
    async fn find_by_id(
        &self,
        id: &FolderId,
        tenant_id: &TenantId,
    ) -> Result<Option<Folder>, InfraError>;

    /// フォルダを挿入する
    async fn insert(&self, folder: &Folder) -> Result<(), InfraError>;

    /// フォルダを更新する（名前変更・移動後の状態を反映）
    async fn update(&self, folder: &Folder) -> Result<(), InfraError>;

    /// フォルダとサブツリーの path/depth を一括更新する（名前変更・移動時）
    ///
    /// `old_path` で始まるすべてのフォルダの path を `new_path` に置換し、
    /// depth に `depth_delta` を加算する。
    async fn update_subtree_paths(
        &self,
        old_path: &str,
        new_path: &str,
        depth_delta: i32,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError>;

    /// フォルダを削除する
    async fn delete(&self, id: &FolderId) -> Result<(), InfraError>;

    /// 指定フォルダの直接の子フォルダ数をカウントする
    async fn count_children(&self, parent_id: &FolderId) -> Result<i64, InfraError>;
}

/// PostgreSQL 実装の FolderRepository
#[derive(Debug, Clone)]
pub struct PostgresFolderRepository {
    pool: PgPool,
}

impl PostgresFolderRepository {
    /// 新しいリポジトリインスタンスを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FolderRepository for PostgresFolderRepository {
    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn find_all_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Folder>, InfraError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id as "tenant_id!",
                name as "name!",
                parent_id,
                path as "path!",
                depth as "depth!",
                created_by,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM folders
            WHERE tenant_id = $1
            ORDER BY path ASC
            "#,
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        let folders = rows
            .into_iter()
            .map(|row| {
                // DB の NOT NULL 制約と CHECK 制約により name は常に有効
                let name =
                    FolderName::new(row.name).expect("DB に格納された FolderName は常に有効");
                Folder::from_db(
                    FolderId::from_uuid(row.id),
                    TenantId::from_uuid(row.tenant_id),
                    name,
                    row.parent_id.map(FolderId::from_uuid),
                    row.path,
                    row.depth,
                    row.created_by.map(UserId::from_uuid),
                    row.created_at,
                    row.updated_at,
                )
            })
            .collect();

        Ok(folders)
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%id, %tenant_id))]
    async fn find_by_id(
        &self,
        id: &FolderId,
        tenant_id: &TenantId,
    ) -> Result<Option<Folder>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id as "tenant_id!",
                name as "name!",
                parent_id,
                path as "path!",
                depth as "depth!",
                created_by,
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM folders
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

        let name = FolderName::new(row.name).expect("DB に格納された FolderName は常に有効");
        Ok(Some(Folder::from_db(
            FolderId::from_uuid(row.id),
            TenantId::from_uuid(row.tenant_id),
            name,
            row.parent_id.map(FolderId::from_uuid),
            row.path,
            row.depth,
            row.created_by.map(UserId::from_uuid),
            row.created_at,
            row.updated_at,
        )))
    }

    #[tracing::instrument(skip_all, level = "debug")]
    async fn insert(&self, folder: &Folder) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            INSERT INTO folders (id, tenant_id, name, parent_id, path, depth, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            folder.id().as_uuid(),
            folder.tenant_id().as_uuid(),
            folder.name().as_str(),
            folder.parent_id().map(|p| *p.as_uuid()),
            folder.path(),
            folder.depth(),
            folder.created_by().map(|u| *u.as_uuid()),
            folder.created_at(),
            folder.updated_at()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug")]
    async fn update(&self, folder: &Folder) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            UPDATE folders
            SET name = $2, parent_id = $3, path = $4, depth = $5, updated_at = $6
            WHERE id = $1
            "#,
            folder.id().as_uuid(),
            folder.name().as_str(),
            folder.parent_id().map(|p| *p.as_uuid()),
            folder.path(),
            folder.depth(),
            folder.updated_at()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn update_subtree_paths(
        &self,
        old_path: &str,
        new_path: &str,
        depth_delta: i32,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        // old_path で始まるすべてのフォルダの path を new_path に置換し、
        // depth に depth_delta を加算する。
        // SUBSTRING(path FROM LENGTH($1) + 1) で old_path 以降の部分を取得し、
        // new_path と結合して新しい path を構築する。
        sqlx::query!(
            r#"
            UPDATE folders
            SET path = $2 || SUBSTRING(path FROM LENGTH($1) + 1),
                depth = depth + $3,
                updated_at = NOW()
            WHERE tenant_id = $4
              AND path LIKE $1 || '%'
            "#,
            old_path,
            new_path,
            depth_delta,
            tenant_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%id))]
    async fn delete(&self, id: &FolderId) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            DELETE FROM folders
            WHERE id = $1
            "#,
            id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%parent_id))]
    async fn count_children(&self, parent_id: &FolderId) -> Result<i64, InfraError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)::bigint as "count!"
            FROM folders
            WHERE parent_id = $1
            "#,
            parent_id.as_uuid()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PostgresFolderRepository>();
    }
}
