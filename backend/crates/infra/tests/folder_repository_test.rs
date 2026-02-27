//! FolderRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test folder_repository_test
//! ```

mod common;

use common::{setup_test_data, test_now};
use ringiflow_domain::folder::{Folder, FolderId, FolderName};
use ringiflow_infra::repository::{FolderRepository, PostgresFolderRepository};
use sqlx::PgPool;

// =============================================================================
// ヘルパー
// =============================================================================

/// テスト用ルートフォルダを作成する
fn create_root_folder(tenant_id: &ringiflow_domain::tenant::TenantId, name: &str) -> Folder {
    let name = FolderName::new(name).unwrap();
    Folder::new(
        FolderId::new(),
        tenant_id.clone(),
        name,
        None,
        None,
        None,
        None,
        test_now(),
    )
    .unwrap()
}

/// テスト用子フォルダを作成する
fn create_child_folder(
    tenant_id: &ringiflow_domain::tenant::TenantId,
    name: &str,
    parent: &Folder,
) -> Folder {
    let name = FolderName::new(name).unwrap();
    Folder::new(
        FolderId::new(),
        tenant_id.clone(),
        name,
        Some(parent.id().clone()),
        Some(parent.path()),
        Some(parent.depth()),
        None,
        test_now(),
    )
    .unwrap()
}

// =============================================================================
// max_subtree_depth テスト
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_max_subtree_depth_サブツリーの最大depth取得(pool: PgPool) {
    // Arrange
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let sut = PostgresFolderRepository::new(pool.clone());

    // depth 1: /root/
    // depth 2: /root/child/
    // depth 3: /root/child/grandchild/
    let root = create_root_folder(&tenant_id, "root");
    let child = create_child_folder(&tenant_id, "child", &root);
    let grandchild = create_child_folder(&tenant_id, "grandchild", &child);

    sut.insert(&root).await.unwrap();
    sut.insert(&child).await.unwrap();
    sut.insert(&grandchild).await.unwrap();

    // Act
    let max_depth = sut
        .max_subtree_depth(root.path(), &tenant_id)
        .await
        .unwrap();

    // Assert: サブツリー内の最大 depth は 3（grandchild）
    assert_eq!(max_depth, 3);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_max_subtree_depth_リーフノードの場合は自身のdepthを返す(
    pool: PgPool,
) {
    // Arrange
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let sut = PostgresFolderRepository::new(pool.clone());

    let root = create_root_folder(&tenant_id, "leaf");
    sut.insert(&root).await.unwrap();

    // Act
    let max_depth = sut
        .max_subtree_depth(root.path(), &tenant_id)
        .await
        .unwrap();

    // Assert: サブツリーが自身のみなので depth 1
    assert_eq!(max_depth, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_max_subtree_depth_別テナントのフォルダは含まない(pool: PgPool) {
    // Arrange
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let other_tenant_id = common::create_other_tenant(&pool).await;
    let sut = PostgresFolderRepository::new(pool.clone());

    // テナント A: depth 1 のみ
    let root_a = create_root_folder(&tenant_id, "root");
    sut.insert(&root_a).await.unwrap();

    // テナント B: 同じ名前で depth 3 まで（テナント分離検証）
    let root_b = create_root_folder(&other_tenant_id, "root");
    let child_b = create_child_folder(&other_tenant_id, "child", &root_b);
    let grandchild_b = create_child_folder(&other_tenant_id, "grandchild", &child_b);
    sut.insert(&root_b).await.unwrap();
    sut.insert(&child_b).await.unwrap();
    sut.insert(&grandchild_b).await.unwrap();

    // Act: テナント A の max_subtree_depth
    let max_depth = sut
        .max_subtree_depth(root_a.path(), &tenant_id)
        .await
        .unwrap();

    // Assert: テナント A のサブツリーのみ（depth 1）
    assert_eq!(max_depth, 1);
}
