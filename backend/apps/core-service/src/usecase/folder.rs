//! フォルダ管理ユースケース

use std::sync::Arc;

use ringiflow_domain::{
    clock::Clock,
    folder::{Folder, FolderId, FolderName, MAX_FOLDER_DEPTH},
    tenant::TenantId,
    user::UserId,
};
use ringiflow_infra::{TransactionManager, repository::FolderRepository};
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
    tx_manager: Arc<dyn TransactionManager>,
}

impl FolderUseCaseImpl {
    pub fn new(
        folder_repository: Arc<dyn FolderRepository>,
        clock: Arc<dyn Clock>,
        tx_manager: Arc<dyn TransactionManager>,
    ) -> Self {
        Self {
            folder_repository,
            clock,
            tx_manager,
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
            if let ringiflow_infra::InfraErrorKind::Database(db_err) = e.kind()
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

        // サブツリーの最大 depth 事前検証（CHECK 制約違反のユーザーフレンドリーなエラー化）
        if old_path != folder.path() {
            let depth_delta = folder.depth() - old_depth;
            if depth_delta > 0 {
                let max_subtree_depth = self
                    .folder_repository
                    .max_subtree_depth(&old_path, &input.tenant_id)
                    .await?;
                if max_subtree_depth + depth_delta > MAX_FOLDER_DEPTH {
                    return Err(CoreError::BadRequest(
                        "移動先ではサブツリーの階層が上限（5 階層）を超えます".to_string(),
                    ));
                }
            }
        }

        // トランザクション内で DB 更新
        let mut tx = self
            .tx_manager
            .begin()
            .await
            .map_err(|e| CoreError::Internal(format!("トランザクション開始に失敗: {}", e)))?;

        self.folder_repository
            .update(&mut tx, &folder)
            .await
            .map_err(|e| {
                if let ringiflow_infra::InfraErrorKind::Database(db_err) = e.kind()
                    && let Some(constraint) =
                        db_err.as_database_error().and_then(|d| d.constraint())
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
                .update_subtree_paths(
                    &mut tx,
                    &old_path,
                    folder.path(),
                    depth_delta,
                    &input.tenant_id,
                )
                .await?;
        }

        tx.commit()
            .await
            .map_err(|e| CoreError::Internal(format!("トランザクションコミットに失敗: {}", e)))?;

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

#[cfg(test)]
mod tests {
    use ringiflow_domain::{clock::FixedClock, folder::FolderName};
    use ringiflow_infra::fake::{FakeFolderRepository, FakeTransactionManager};

    use super::*;

    fn fixed_now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn create_sut(repo: FakeFolderRepository) -> FolderUseCaseImpl {
        FolderUseCaseImpl::new(
            Arc::new(repo),
            Arc::new(FixedClock::new(fixed_now())),
            Arc::new(FakeTransactionManager),
        )
    }

    /// テスト用ルートフォルダを作成する
    fn create_root_folder(tenant_id: &TenantId, name: &str) -> Folder {
        let name = FolderName::new(name).unwrap();
        Folder::new(
            FolderId::new(),
            tenant_id.clone(),
            name,
            None,
            None,
            None,
            None,
            fixed_now(),
        )
        .unwrap()
    }

    /// テスト用子フォルダを作成する
    fn create_child_folder(tenant_id: &TenantId, name: &str, parent: &Folder) -> Folder {
        let name = FolderName::new(name).unwrap();
        Folder::new(
            FolderId::new(),
            tenant_id.clone(),
            name,
            Some(parent.id().clone()),
            Some(parent.path()),
            Some(parent.depth()),
            None,
            fixed_now(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_update_folder_サブツリーdepth超過時にエラーを返す() {
        // Arrange:
        // depth 1: /l1/         (移動対象)
        // depth 2: /l1/l2/      (サブツリー)
        // depth 3: /l1/l2/l3/   (サブツリー)
        // depth 1: /target/     (移動先 — depth 4 に位置)
        // depth 2: /target/l2/
        // depth 3: /target/l2/l3/
        // depth 4: /target/l2/l3/l4/
        //
        // /l1/ を /target/l2/l3/l4/ の下に移動すると:
        // /l1/ → depth 5, /l1/l2/ → depth 6, /l1/l2/l3/ → depth 7
        // depth_delta = +4, max_subtree_depth = 3, 3 + 4 = 7 > 5 → エラー
        let tenant_id = TenantId::new();
        let repo = FakeFolderRepository::new();

        // 移動対象のフォルダツリー（3 階層）
        let l1 = create_root_folder(&tenant_id, "l1");
        let l2 = create_child_folder(&tenant_id, "l2", &l1);
        let l3 = create_child_folder(&tenant_id, "l3", &l2);

        // 移動先のフォルダツリー（4 階層）
        let target = create_root_folder(&tenant_id, "target");
        let t2 = create_child_folder(&tenant_id, "t2", &target);
        let t3 = create_child_folder(&tenant_id, "t3", &t2);
        let t4 = create_child_folder(&tenant_id, "t4", &t3);

        let l1_id = l1.id().clone();
        let t4_id = *t4.id().as_uuid();

        repo.add_folder(l1);
        repo.add_folder(l2);
        repo.add_folder(l3);
        repo.add_folder(target);
        repo.add_folder(t2);
        repo.add_folder(t3);
        repo.add_folder(t4);

        let sut = create_sut(repo);

        // Act
        let result = sut
            .update_folder(UpdateFolderInput {
                folder_id: l1_id,
                tenant_id,
                name: None,
                parent_id: Some(Some(t4_id)),
            })
            .await;

        // Assert: サブツリー depth 超過でエラー
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, CoreError::BadRequest(ref msg) if msg.contains("階層")),
            "expected BadRequest with 階層 message, got: {:?}",
            err
        );
    }
}
