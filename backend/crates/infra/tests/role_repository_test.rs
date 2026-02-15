//! RoleRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test role_repository_test
//! ```

mod common;

use common::setup_test_data;
use ringiflow_domain::{
    role::{Permission, Role, RoleId},
    tenant::TenantId,
};
use ringiflow_infra::repository::{PostgresRoleRepository, RoleRepository};
use sqlx::PgPool;
use uuid::Uuid;

/// テスト用カスタムロールを DB に作成するヘルパー
async fn insert_custom_role(
    pool: &PgPool,
    tenant_id: &TenantId,
    name: &str,
    permissions: &[&str],
) -> RoleId {
    let role_id = RoleId::new();
    let perm_json = serde_json::Value::Array(
        permissions
            .iter()
            .map(|p| serde_json::Value::String(p.to_string()))
            .collect(),
    );
    sqlx::query!(
        r#"
        INSERT INTO roles (id, tenant_id, name, description, permissions, is_system)
        VALUES ($1, $2, $3, $4, $5, false)
        "#,
        role_id.as_uuid(),
        tenant_id.as_uuid(),
        name,
        format!("{name} の説明"),
        perm_json,
    )
    .execute(pool)
    .await
    .expect("カスタムロール作成に失敗");
    role_id
}

// ===== find_all_by_tenant_with_user_count テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_all_by_tenant_with_user_countでシステムロールとテナントロールが取得できる(
    pool: PgPool,
) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    insert_custom_role(&pool, &tenant_id, "custom_viewer", &["workflow:read"]).await;

    let sut = PostgresRoleRepository::new(pool);
    let results = sut
        .find_all_by_tenant_with_user_count(&tenant_id)
        .await
        .unwrap();

    // system_admin 除外、残りのシステムロール + カスタムロール
    assert!(results.len() >= 2); // tenant_admin, user + custom_viewer の最低 3 つ
    let names: Vec<&str> = results.iter().map(|(r, _)| r.name()).collect();
    assert!(names.contains(&"custom_viewer"));
    // システムロールも含まれる
    assert!(names.contains(&"user"));
    assert!(names.contains(&"tenant_admin"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_all_by_tenant_with_user_countでsystem_adminが除外される(pool: PgPool) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let sut = PostgresRoleRepository::new(pool);

    let results = sut
        .find_all_by_tenant_with_user_count(&tenant_id)
        .await
        .unwrap();

    let names: Vec<&str> = results.iter().map(|(r, _)| r.name()).collect();
    assert!(
        !names.contains(&"system_admin"),
        "system_admin は一覧から除外されるべき"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_all_by_tenant_with_user_countでユーザー数が正しく集計される(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    common::assign_role(&pool, &user_id, &tenant_id).await;

    let sut = PostgresRoleRepository::new(pool);
    let results = sut
        .find_all_by_tenant_with_user_count(&tenant_id)
        .await
        .unwrap();

    // "user" ロールに 1 人割り当てられている
    let user_role = results.iter().find(|(r, _)| r.name() == "user").unwrap();
    assert_eq!(user_role.1, 1, "user ロールのユーザー数は 1");

    // "tenant_admin" には割り当てなし
    let admin_role = results
        .iter()
        .find(|(r, _)| r.name() == "tenant_admin")
        .unwrap();
    assert_eq!(admin_role.1, 0, "tenant_admin ロールのユーザー数は 0");
}

// ===== find_by_id テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_idでロールが取得できる(pool: PgPool) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let role_id = insert_custom_role(
        &pool,
        &tenant_id,
        "test_role",
        &["workflow:read", "task:read"],
    )
    .await;

    let sut = PostgresRoleRepository::new(pool);
    let result = sut.find_by_id(&role_id).await.unwrap();

    assert!(result.is_some());
    let role = result.unwrap();
    assert_eq!(role.name(), "test_role");
    assert_eq!(role.description(), Some("test_role の説明"));
    assert!(!role.is_system());
    assert_eq!(role.permissions().len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_idで存在しないidはnoneを返す(pool: PgPool) {
    let sut = PostgresRoleRepository::new(pool);
    let nonexistent_id = RoleId::from_uuid(Uuid::now_v7());

    let result = sut.find_by_id(&nonexistent_id).await.unwrap();

    assert!(result.is_none());
}

// ===== insert テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_insertでカスタムロールを作成できる(pool: PgPool) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let sut = PostgresRoleRepository::new(pool);

    let now = chrono::Utc::now();
    let role = Role::new_tenant(
        RoleId::new(),
        tenant_id,
        "new_custom_role".to_string(),
        Some("カスタムロールの説明".to_string()),
        vec![
            Permission::new("workflow:read"),
            Permission::new("task:read"),
        ],
        now,
    );

    sut.insert(&role).await.unwrap();

    let found = sut.find_by_id(role.id()).await.unwrap().unwrap();
    assert_eq!(found.name(), "new_custom_role");
    assert_eq!(found.description(), Some("カスタムロールの説明"));
    assert!(!found.is_system());
    assert_eq!(found.permissions().len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_insertでテナント内の同名ロールは重複エラー(pool: PgPool) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    insert_custom_role(&pool, &tenant_id, "duplicate_role", &["workflow:read"]).await;

    let sut = PostgresRoleRepository::new(pool);
    let now = chrono::Utc::now();
    let role = Role::new_tenant(
        RoleId::new(),
        tenant_id,
        "duplicate_role".to_string(),
        None,
        vec![Permission::new("task:read")],
        now,
    );

    let result = sut.insert(&role).await;
    assert!(result.is_err(), "同名ロールの INSERT はエラーになるべき");
}

// ===== update テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_updateでロール名と説明と権限を更新できる(pool: PgPool) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let role_id = insert_custom_role(&pool, &tenant_id, "old_name", &["workflow:read"]).await;

    let sut = PostgresRoleRepository::new(pool);
    let role = sut.find_by_id(&role_id).await.unwrap().unwrap();

    let now = chrono::Utc::now();
    let updated = role
        .with_name("new_name".to_string(), now)
        .with_description(Some("新しい説明".to_string()), now)
        .with_permissions(
            vec![
                Permission::new("workflow:read"),
                Permission::new("workflow:create"),
            ],
            now,
        );
    sut.update(&updated).await.unwrap();

    let found = sut.find_by_id(&role_id).await.unwrap().unwrap();
    assert_eq!(found.name(), "new_name");
    assert_eq!(found.description(), Some("新しい説明"));
    assert_eq!(found.permissions().len(), 2);
}

// ===== delete テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_deleteでカスタムロールを削除できる(pool: PgPool) {
    let (tenant_id, _user_id) = setup_test_data(&pool).await;
    let role_id = insert_custom_role(&pool, &tenant_id, "to_delete", &["workflow:read"]).await;

    let sut = PostgresRoleRepository::new(pool);

    sut.delete(&role_id).await.unwrap();

    let found = sut.find_by_id(&role_id).await.unwrap();
    assert!(found.is_none(), "削除後は取得できない");
}

// ===== count_users_with_role テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_count_users_with_roleでロールに割り当てられたユーザー数を返す(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;

    // テスト用カスタムロールを作成し、ユーザーに割り当て
    let role_id =
        insert_custom_role(&pool, &tenant_id, "count_test_role", &["workflow:read"]).await;
    sqlx::query!(
        r#"
        INSERT INTO user_roles (user_id, role_id, tenant_id)
        VALUES ($1, $2, $3)
        "#,
        user_id.as_uuid(),
        role_id.as_uuid(),
        tenant_id.as_uuid()
    )
    .execute(&pool)
    .await
    .expect("ロール割り当てに失敗");

    // 割り当てのないロール
    let empty_role_id = insert_custom_role(&pool, &tenant_id, "empty_role", &["task:read"]).await;

    let sut = PostgresRoleRepository::new(pool);

    let count = sut.count_users_with_role(&role_id).await.unwrap();
    assert_eq!(count, 1);

    let count = sut.count_users_with_role(&empty_role_id).await.unwrap();
    assert_eq!(count, 0);
}
