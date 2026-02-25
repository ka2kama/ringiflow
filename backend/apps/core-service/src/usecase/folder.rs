//! フォルダ管理ユースケース

use std::sync::Arc;

use ringiflow_domain::{
    clock::Clock,
    folder::{Folder, FolderId, FolderName},
    tenant::TenantId,
    user::UserId,
};
use ringiflow_infra::repository::FolderRepository;
use uuid::Uuid;

use crate::error::CoreError;

/// フォルダ作成の入力
pub struct CreateFolderInput {
    pub tenant_id:  TenantId,
    pub name:       String,
    pub parent_id:  Option<Uuid>,
    pub created_by: Uuid,
}

/// フォルダ更新の入力
///
/// - `name`: 変更なしは `None`
/// - `parent_id`: 変更なしは `None`、ルートに移動は `Some(None)`、
///   別フォルダに移動は `Some(Some(id))`
pub struct UpdateFolderInput {
    pub folder_id: FolderId,
    pub tenant_id: TenantId,
    pub name:      Option<String>,
    pub parent_id: Option<Option<Uuid>>,
}

/// フォルダ管理ユースケース
pub struct FolderUseCaseImpl {
    folder_repository: Arc<dyn FolderRepository>,
    clock: Arc<dyn Clock>,
}

impl FolderUseCaseImpl {
    pub fn new(folder_repository: Arc<dyn FolderRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            folder_repository,
            clock,
        }
    }

    /// フォルダ一覧を取得する（path 順）
    pub async fn list_folders(&self, tenant_id: &TenantId) -> Result<Vec<Folder>, CoreError> {
        let folders = self.folder_repository.find_all_by_tenant(tenant_id).await?;
        Ok(folders)
    }

    /// フォルダを作成する
    ///
    /// 1. FolderName バリデーション
    /// 2. 親フォルダの存在確認と depth チェック
    /// 3. Folder エンティティ生成・挿入
    /// 4. UNIQUE 制約違反は Conflict にマッピング
    pub async fn create_folder(&self, input: CreateFolderInput) -> Result<Folder, CoreError> {
        let name = FolderName::new(input.name).map_err(|e| CoreError::BadRequest(e.to_string()))?;
        let now = self.clock.now();

        let (parent_id, parent_path, parent_depth) = match input.parent_id {
            Some(pid) => {
                let parent_folder_id = FolderId::from_uuid(pid);
                let parent = self
                    .folder_repository
                    .find_by_id(&parent_folder_id, &input.tenant_id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("親フォルダが見つかりません".to_string()))?;
                // depth チェック（child_depth で MAX_FOLDER_DEPTH を超えないか確認）
                parent
                    .child_depth()
                    .map_err(|e| CoreError::BadRequest(e.to_string()))?;
                (
                    Some(parent_folder_id),
                    Some(parent.path().to_string()),
                    Some(parent.depth()),
                )
            }
            None => (None, None, None),
        };

        let folder = Folder::new(
            FolderId::new(),
            input.tenant_id,
            name,
            parent_id,
            parent_path.as_deref(),
            parent_depth,
            Some(UserId::from_uuid(input.created_by)),
            now,
        )
        .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        self.folder_repository.insert(&folder).await.map_err(|e| {
            // UNIQUE 制約違反（tenant_id, parent_id, name）の場合は Conflict
            if let ringiflow_infra::InfraError::Database(ref db_err) = e
                && let Some(constraint) = db_err.as_database_error().and_then(|d| d.constraint())
                && constraint == "folders_tenant_id_parent_id_name_key"
            {
                return CoreError::Conflict("同名のフォルダが既に存在します".to_string());
            }
            CoreError::Database(e)
        })?;

        Ok(folder)
    }

    /// フォルダを更新する（名前変更・移動）
    ///
    /// 名前変更と移動は同時に指定可能。
    /// サブツリーの path/depth 更新も実施する。
    pub async fn update_folder(&self, input: UpdateFolderInput) -> Result<Folder, CoreError> {
        let folder = self
            .folder_repository
            .find_by_id(&input.folder_id, &input.tenant_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("フォルダが見つかりません".to_string()))?;

        let now = self.clock.now();
        let old_path = folder.path().to_string();
        let old_depth = folder.depth();

        // 移動処理
        let folder = if let Some(new_parent_id_opt) = input.parent_id {
            let (new_parent_folder_id, new_parent_path, new_parent_depth) = match new_parent_id_opt
            {
                Some(pid) => {
                    let new_parent_folder_id = FolderId::from_uuid(pid);

                    // 自分自身への移動を拒否
                    if new_parent_folder_id == *folder.id() {
                        return Err(CoreError::BadRequest(
                            "フォルダを自分自身に移動することはできません".to_string(),
                        ));
                    }

                    let parent = self
                        .folder_repository
                        .find_by_id(&new_parent_folder_id, &input.tenant_id)
                        .await?
                        .ok_or_else(|| {
                            CoreError::NotFound("移動先の親フォルダが見つかりません".to_string())
                        })?;

                    // 循環検出: 移動先が自身のサブツリー内かチェック
                    if parent.path().starts_with(&old_path) {
                        return Err(CoreError::BadRequest(
                            "フォルダを自身の子孫に移動することはできません".to_string(),
                        ));
                    }

                    // depth チェック
                    parent
                        .child_depth()
                        .map_err(|e| CoreError::BadRequest(e.to_string()))?;

                    (
                        Some(new_parent_folder_id),
                        Some(parent.path().to_string()),
                        Some(parent.depth()),
                    )
                }
                None => (None, None, None), // ルートに移動
            };

            folder
                .move_to(
                    new_parent_folder_id,
                    new_parent_path.as_deref(),
                    new_parent_depth,
                    now,
                )
                .map_err(|e| CoreError::BadRequest(e.to_string()))?
        } else {
            folder
        };

        // 名前変更処理
        let folder = if let Some(new_name) = input.name {
            let new_name =
                FolderName::new(new_name).map_err(|e| CoreError::BadRequest(e.to_string()))?;
            folder.rename(new_name, now)
        } else {
            folder
        };

        // DB 更新
        self.folder_repository.update(&folder).await.map_err(|e| {
            if let ringiflow_infra::InfraError::Database(ref db_err) = e
                && let Some(constraint) = db_err.as_database_error().and_then(|d| d.constraint())
                && constraint == "folders_tenant_id_parent_id_name_key"
            {
                return CoreError::Conflict("同名のフォルダが既に存在します".to_string());
            }
            CoreError::Database(e)
        })?;

        // サブツリーの path/depth 更新（path が変わった場合のみ）
        if old_path != folder.path() {
            let depth_delta = folder.depth() - old_depth;
            self.folder_repository
                .update_subtree_paths(&old_path, folder.path(), depth_delta, &input.tenant_id)
                .await?;
        }

        Ok(folder)
    }

    /// フォルダを削除する
    ///
    /// 子フォルダがある場合はエラーを返す。
    pub async fn delete_folder(
        &self,
        folder_id: &FolderId,
        tenant_id: &TenantId,
    ) -> Result<(), CoreError> {
        let _folder = self
            .folder_repository
            .find_by_id(folder_id, tenant_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("フォルダが見つかりません".to_string()))?;

        // 子フォルダチェック
        let child_count = self
            .folder_repository
            .count_children(folder_id, tenant_id)
            .await?;
        if child_count > 0 {
            return Err(CoreError::BadRequest(
                "子フォルダが存在するため削除できません".to_string(),
            ));
        }

        self.folder_repository.delete(folder_id, tenant_id).await?;

        Ok(())
    }
}
