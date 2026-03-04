//! CredentialsRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test credentials_repository_test
//! ```

mod common;

use common::{create_other_tenant, insert_user_raw, setup_test_data};
use ringiflow_infra::repository::{
    CredentialType,
    CredentialsRepository,
    PostgresCredentialsRepository,
};
use sqlx::PgPool;

// ===== create テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_createでcredentialを作成しdbに正しく保存される(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let sut = PostgresCredentialsRepository::new(pool);

    let id = sut
        .create(
            &user_id,
            &tenant_id,
            CredentialType::Password,
            "$argon2id$test_hash",
        )
        .await
        .unwrap();

    let credential = sut
        .find_by_user_and_type(&tenant_id, &user_id, CredentialType::Password)
        .await
        .unwrap()
        .expect("作成した credential が見つかるべき");

    assert_eq!(credential.id, id);
    assert_eq!(credential.user_id, user_id);
    assert_eq!(credential.tenant_id, tenant_id);
    assert_eq!(credential.credential_type, CredentialType::Password);
    assert_eq!(credential.credential_data, "$argon2id$test_hash");
    assert!(credential.is_active, "デフォルトで is_active = true");
    assert!(
        credential.last_used_at.is_none(),
        "初期状態で last_used_at = None"
    );
}

// ===== find_by_user_and_type テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_user_and_typeで存在しないcredentialはnoneを返す(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let sut = PostgresCredentialsRepository::new(pool);

    let result = sut
        .find_by_user_and_type(&tenant_id, &user_id, CredentialType::Password)
        .await
        .unwrap();

    assert!(result.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_user_and_typeで別テナントのcredentialは取得できない(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let other_tenant_id = create_other_tenant(&pool).await;
    let sut = PostgresCredentialsRepository::new(pool);

    sut.create(
        &user_id,
        &tenant_id,
        CredentialType::Password,
        "$argon2id$hash",
    )
    .await
    .unwrap();

    // 別テナントからは取得できない
    let result = sut
        .find_by_user_and_type(&other_tenant_id, &user_id, CredentialType::Password)
        .await
        .unwrap();

    assert!(result.is_none(), "別テナントの credential は取得できない");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_user_and_typeで異なるcredential_typeでは取得できない(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let sut = PostgresCredentialsRepository::new(pool);

    sut.create(
        &user_id,
        &tenant_id,
        CredentialType::Password,
        "$argon2id$hash",
    )
    .await
    .unwrap();

    // 異なる credential_type で検索 → None
    let result = sut
        .find_by_user_and_type(&tenant_id, &user_id, CredentialType::Totp)
        .await
        .unwrap();

    assert!(result.is_none(), "異なる credential_type では取得できない");
}

// ===== delete_by_user テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_by_userでユーザーの全credentialsを削除できる(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let sut = PostgresCredentialsRepository::new(pool);

    sut.create(
        &user_id,
        &tenant_id,
        CredentialType::Password,
        "$argon2id$hash",
    )
    .await
    .unwrap();

    sut.delete_by_user(&tenant_id, &user_id).await.unwrap();

    let result = sut
        .find_by_user_and_type(&tenant_id, &user_id, CredentialType::Password)
        .await
        .unwrap();

    assert!(result.is_none(), "削除後は取得できない");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_by_userで別テナントのcredentialsは削除されない(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let other_tenant_id = create_other_tenant(&pool).await;
    let other_user_id = insert_user_raw(
        &pool,
        &other_tenant_id,
        1,
        "other@example.com",
        "Other",
        "active",
    )
    .await;
    let sut = PostgresCredentialsRepository::new(pool);

    sut.create(
        &user_id,
        &tenant_id,
        CredentialType::Password,
        "$argon2id$hash_a",
    )
    .await
    .unwrap();
    sut.create(
        &other_user_id,
        &other_tenant_id,
        CredentialType::Password,
        "$argon2id$hash_b",
    )
    .await
    .unwrap();

    // テナント A のユーザーの credentials を削除
    sut.delete_by_user(&tenant_id, &user_id).await.unwrap();

    // テナント B の credential は残っている
    let result = sut
        .find_by_user_and_type(&other_tenant_id, &other_user_id, CredentialType::Password)
        .await
        .unwrap();

    assert!(result.is_some(), "別テナントの credential は影響を受けない");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_by_userでcredentialsがないユーザーの削除はエラーにならない(
    pool: PgPool,
) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let sut = PostgresCredentialsRepository::new(pool);

    let result = sut.delete_by_user(&tenant_id, &user_id).await;

    assert!(result.is_ok());
}

// ===== update_last_used テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_last_usedでlast_used_atが更新される(pool: PgPool) {
    let (tenant_id, user_id) = setup_test_data(&pool).await;
    let sut = PostgresCredentialsRepository::new(pool);

    let id = sut
        .create(
            &user_id,
            &tenant_id,
            CredentialType::Password,
            "$argon2id$hash",
        )
        .await
        .unwrap();

    // 初期状態: last_used_at = None
    let before = sut
        .find_by_user_and_type(&tenant_id, &user_id, CredentialType::Password)
        .await
        .unwrap()
        .unwrap();
    assert!(before.last_used_at.is_none());

    sut.update_last_used(id).await.unwrap();

    let after = sut
        .find_by_user_and_type(&tenant_id, &user_id, CredentialType::Password)
        .await
        .unwrap()
        .unwrap();
    assert!(
        after.last_used_at.is_some(),
        "last_used_at が更新されるべき"
    );
}
