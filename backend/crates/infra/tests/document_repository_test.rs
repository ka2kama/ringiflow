//! DocumentRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test document_repository_test
//! ```

mod common;

use common::{create_other_tenant, setup_test_data, test_now};
use ringiflow_domain::{
    document::{Document, DocumentId, DocumentStatus, UploadContext},
    folder::FolderId,
    tenant::TenantId,
    user::UserId,
    workflow::WorkflowInstanceId,
};
use ringiflow_infra::repository::{DocumentRepository, PostgresDocumentRepository};
use sqlx::PgPool;
use uuid::Uuid;

// =============================================================================
// ヘルパー
// =============================================================================

/// テスト用フォルダを直接 SQL で作成する
async fn insert_test_folder(pool: &PgPool, tenant_id: &TenantId) -> FolderId {
    let folder_id = FolderId::from_uuid(Uuid::now_v7());
    sqlx::query!(
        r#"
        INSERT INTO folders (id, tenant_id, name, path, depth)
        VALUES ($1, $2, 'テストフォルダ', '/テストフォルダ/', 1)
        "#,
        folder_id.as_uuid(),
        tenant_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("フォルダ作成に失敗");
    folder_id
}

/// テスト用ワークフローインスタンスを直接 SQL で作成する
async fn insert_test_workflow_instance(
    pool: &PgPool,
    tenant_id: &TenantId,
    user_id: &UserId,
) -> WorkflowInstanceId {
    let instance_id = WorkflowInstanceId::from_uuid(Uuid::now_v7());
    // シードデータの定義 ID を使用
    let definition_id: Uuid = "00000000-0000-0000-0000-000000000001".parse().unwrap();
    sqlx::query!(
        r#"
        INSERT INTO workflow_instances (id, tenant_id, definition_id, definition_version, display_number, title, form_data, status, initiated_by)
        VALUES ($1, $2, $3, 1, 1, 'テスト申請', '{}'::jsonb, 'draft', $4)
        "#,
        instance_id.as_uuid(),
        tenant_id.as_uuid(),
        definition_id,
        user_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("ワークフローインスタンス作成に失敗");
    instance_id
}

/// テスト用ドキュメントを作成するヘルパー
fn create_test_document(
    tenant_id: &TenantId,
    upload_context: UploadContext,
    uploaded_by: &UserId,
) -> Document {
    let now = test_now();
    let document_id = DocumentId::new();
    let s3_key = format!(
        "{}/test/{}_test.pdf",
        tenant_id.as_uuid(),
        document_id.as_uuid()
    );
    Document::new_uploading(
        document_id,
        tenant_id.clone(),
        "test.pdf".to_string(),
        "application/pdf".to_string(),
        1024,
        s3_key,
        upload_context,
        Some(uploaded_by.clone()),
        now,
    )
}

/// サイズ指定でテスト用ドキュメントを作成するヘルパー
fn create_test_document_with_size(
    tenant_id: &TenantId,
    upload_context: UploadContext,
    uploaded_by: &UserId,
    size: i64,
) -> Document {
    let now = test_now();
    let document_id = DocumentId::new();
    let s3_key = format!(
        "{}/test/{}_test.pdf",
        tenant_id.as_uuid(),
        document_id.as_uuid()
    );
    Document::new_uploading(
        document_id,
        tenant_id.clone(),
        "test.pdf".to_string(),
        "application/pdf".to_string(),
        size,
        s3_key,
        upload_context,
        Some(uploaded_by.clone()),
        now,
    )
}

// ===== insert + find_by_id テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_フォルダコンテキストのドキュメントを挿入し取得できる(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let folder_id = insert_test_folder(&pool, &tenant_id).await;
    let sut = PostgresDocumentRepository::new(pool);

    let document = create_test_document(
        &tenant_id,
        UploadContext::Folder(folder_id.clone()),
        &user_id,
    );
    sut.insert(&document).await.expect("ドキュメント挿入に失敗");

    let found = sut
        .find_by_id(document.id(), &tenant_id)
        .await
        .expect("検索に失敗")
        .expect("ドキュメントが見つからない");

    assert_eq!(found.id(), document.id());
    assert_eq!(found.tenant_id(), &tenant_id);
    assert_eq!(found.filename(), "test.pdf");
    assert_eq!(found.content_type(), "application/pdf");
    assert_eq!(found.size(), 1024);
    assert_eq!(found.status(), DocumentStatus::Uploading);
    assert_eq!(found.upload_context(), &UploadContext::Folder(folder_id));
    assert!(found.uploaded_by().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ワークフローコンテキストのドキュメントを挿入し取得できる(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let instance_id = insert_test_workflow_instance(&pool, &tenant_id, &user_id).await;
    let sut = PostgresDocumentRepository::new(pool);

    let document = create_test_document(
        &tenant_id,
        UploadContext::Workflow(instance_id.clone()),
        &user_id,
    );
    sut.insert(&document).await.expect("ドキュメント挿入に失敗");

    let found = sut
        .find_by_id(document.id(), &tenant_id)
        .await
        .expect("検索に失敗")
        .expect("ドキュメントが見つからない");

    assert_eq!(found.id(), document.id());
    assert_eq!(
        found.upload_context(),
        &UploadContext::Workflow(instance_id)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_存在しないidの場合noneを返す(pool: PgPool) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let sut = PostgresDocumentRepository::new(pool);

    let result = sut
        .find_by_id(&DocumentId::new(), &tenant_id)
        .await
        .expect("検索に失敗");

    assert!(result.is_none());
}

// ===== テナント分離テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_別テナントのドキュメントは取得できない(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let folder_id = insert_test_folder(&pool, &tenant_id).await;
    let other_tenant_id = create_other_tenant(&pool).await;
    let sut = PostgresDocumentRepository::new(pool);

    let document = create_test_document(&tenant_id, UploadContext::Folder(folder_id), &user_id);
    sut.insert(&document).await.expect("ドキュメント挿入に失敗");

    // 別テナントからは取得できない
    let result = sut
        .find_by_id(document.id(), &other_tenant_id)
        .await
        .expect("検索に失敗");

    assert!(result.is_none());
}

// ===== update_status テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_ステータスをuploadingからactiveに更新できる(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let folder_id = insert_test_folder(&pool, &tenant_id).await;
    let sut = PostgresDocumentRepository::new(pool.clone());

    let document = create_test_document(&tenant_id, UploadContext::Folder(folder_id), &user_id);
    sut.insert(&document).await.expect("ドキュメント挿入に失敗");

    let now = chrono::Utc::now();
    sut.update_status(document.id(), DocumentStatus::Active, &tenant_id, now)
        .await
        .expect("ステータス更新に失敗");

    let found = sut
        .find_by_id(document.id(), &tenant_id)
        .await
        .expect("検索に失敗")
        .expect("ドキュメントが見つからない");

    assert_eq!(found.status(), DocumentStatus::Active);
}

// ===== count_and_total_size_by_folder テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_フォルダ内のドキュメント数と合計サイズを取得できる(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let folder_id = insert_test_folder(&pool, &tenant_id).await;
    let sut = PostgresDocumentRepository::new(pool);

    // 2 つのドキュメントを挿入
    let doc1 = create_test_document_with_size(
        &tenant_id,
        UploadContext::Folder(folder_id.clone()),
        &user_id,
        1000,
    );
    let doc2 = create_test_document_with_size(
        &tenant_id,
        UploadContext::Folder(folder_id.clone()),
        &user_id,
        2000,
    );
    sut.insert(&doc1).await.expect("ドキュメント1挿入に失敗");
    sut.insert(&doc2).await.expect("ドキュメント2挿入に失敗");

    let (count, total_size) = sut
        .count_and_total_size_by_folder(&folder_id, &tenant_id)
        .await
        .expect("集計に失敗");

    assert_eq!(count, 2);
    assert_eq!(total_size, 3000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_フォルダ集計でdeletedドキュメントは除外される(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let folder_id = insert_test_folder(&pool, &tenant_id).await;
    let sut = PostgresDocumentRepository::new(pool.clone());

    // ドキュメントを挿入してから deleted にする
    let doc = create_test_document_with_size(
        &tenant_id,
        UploadContext::Folder(folder_id.clone()),
        &user_id,
        1000,
    );
    sut.insert(&doc).await.expect("ドキュメント挿入に失敗");

    let now = chrono::Utc::now();
    sut.update_status(doc.id(), DocumentStatus::Deleted, &tenant_id, now)
        .await
        .expect("ステータス更新に失敗");

    let (count, total_size) = sut
        .count_and_total_size_by_folder(&folder_id, &tenant_id)
        .await
        .expect("集計に失敗");

    assert_eq!(count, 0);
    assert_eq!(total_size, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_空フォルダの集計はゼロを返す(pool: PgPool) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let folder_id = insert_test_folder(&pool, &tenant_id).await;
    let sut = PostgresDocumentRepository::new(pool);

    let (count, total_size) = sut
        .count_and_total_size_by_folder(&folder_id, &tenant_id)
        .await
        .expect("集計に失敗");

    assert_eq!(count, 0);
    assert_eq!(total_size, 0);
}

// ===== count_and_total_size_by_workflow テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_ワークフロー内のドキュメント数と合計サイズを取得できる(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let instance_id = insert_test_workflow_instance(&pool, &tenant_id, &user_id).await;
    let sut = PostgresDocumentRepository::new(pool);

    // 2 つのドキュメントを挿入
    let doc1 = create_test_document_with_size(
        &tenant_id,
        UploadContext::Workflow(instance_id.clone()),
        &user_id,
        500,
    );
    let doc2 = create_test_document_with_size(
        &tenant_id,
        UploadContext::Workflow(instance_id.clone()),
        &user_id,
        1500,
    );
    sut.insert(&doc1).await.expect("ドキュメント1挿入に失敗");
    sut.insert(&doc2).await.expect("ドキュメント2挿入に失敗");

    let (count, total_size) = sut
        .count_and_total_size_by_workflow(&instance_id, &tenant_id)
        .await
        .expect("集計に失敗");

    assert_eq!(count, 2);
    assert_eq!(total_size, 2000);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ワークフロー集計でdeletedドキュメントは除外される(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let instance_id = insert_test_workflow_instance(&pool, &tenant_id, &user_id).await;
    let sut = PostgresDocumentRepository::new(pool.clone());

    let doc = create_test_document_with_size(
        &tenant_id,
        UploadContext::Workflow(instance_id.clone()),
        &user_id,
        1000,
    );
    sut.insert(&doc).await.expect("ドキュメント挿入に失敗");

    let now = chrono::Utc::now();
    sut.update_status(doc.id(), DocumentStatus::Deleted, &tenant_id, now)
        .await
        .expect("ステータス更新に失敗");

    let (count, total_size) = sut
        .count_and_total_size_by_workflow(&instance_id, &tenant_id)
        .await
        .expect("集計に失敗");

    assert_eq!(count, 0);
    assert_eq!(total_size, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_空ワークフローの集計はゼロを返す(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let instance_id = insert_test_workflow_instance(&pool, &tenant_id, &user_id).await;
    let sut = PostgresDocumentRepository::new(pool);

    let (count, total_size) = sut
        .count_and_total_size_by_workflow(&instance_id, &tenant_id)
        .await
        .expect("集計に失敗");

    assert_eq!(count, 0);
    assert_eq!(total_size, 0);
}
