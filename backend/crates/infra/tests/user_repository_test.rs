//! UserRepository 統合テスト
//!
//! データベースを使用したテスト。sqlx::test マクロを使用して、
//! テストごとにトランザクションを作成しロールバックする。
//!
//! 実行方法:
//! ```bash
//! just setup-db
//! cd backend && cargo test -p ringiflow-infra --test user_repository_test
//! ```

use ringiflow_domain::{
   tenant::TenantId,
   user::{Email, UserId},
};
use ringiflow_infra::repository::{PostgresUserRepository, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

/// テスト用のテナントとユーザーをセットアップ
async fn setup_test_data(pool: &PgPool) -> (TenantId, UserId) {
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   let user_id = UserId::from_uuid(Uuid::now_v7());

   // テナント作成
   sqlx::query!(
      r#"
        INSERT INTO tenants (id, name, subdomain, plan, status)
        VALUES ($1, 'Test Tenant', 'test', 'free', 'active')
        "#,
      tenant_id.as_uuid()
   )
   .execute(pool)
   .await
   .expect("テナント作成に失敗");

   // ユーザー作成
   sqlx::query!(
        r#"
        INSERT INTO users (id, tenant_id, email, name, password_hash, status)
        VALUES ($1, $2, 'test@example.com', 'Test User', '$argon2id$v=19$m=65536,t=1,p=1$dGVzdA$dGVzdA', 'active')
        "#,
        user_id.as_uuid(),
        tenant_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("ユーザー作成に失敗");

   (tenant_id, user_id)
}

/// ロールをユーザーに割り当て
async fn assign_role(pool: &PgPool, user_id: &UserId) {
   // システムロール（user）を取得して割り当て
   sqlx::query!(
      r#"
        INSERT INTO user_roles (user_id, role_id)
        SELECT $1, id FROM roles WHERE name = 'user' AND is_system = true
        "#,
      user_id.as_uuid()
   )
   .execute(pool)
   .await
   .expect("ロール割り当てに失敗");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_メールアドレスでユーザーを取得できる(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;
   let repo = PostgresUserRepository::new(pool);
   let email = Email::new("test@example.com").unwrap();

   let result = repo.find_by_email(&tenant_id, &email).await;

   assert!(result.is_ok());
   let user = result.unwrap();
   assert!(user.is_some());
   let user = user.unwrap();
   assert_eq!(user.email().as_str(), "test@example.com");
   assert_eq!(user.name(), "Test User");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_存在しないメールアドレスの場合noneを返す(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;
   let repo = PostgresUserRepository::new(pool);
   let email = Email::new("nonexistent@example.com").unwrap();

   let result = repo.find_by_email(&tenant_id, &email).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_別テナントのユーザーは取得できない(pool: PgPool) {
   let (_, _user_id) = setup_test_data(&pool).await;
   let other_tenant_id = TenantId::from_uuid(Uuid::now_v7());

   // 別テナントを作成
   sqlx::query!(
      r#"
        INSERT INTO tenants (id, name, subdomain, plan, status)
        VALUES ($1, 'Other Tenant', 'other', 'free', 'active')
        "#,
      other_tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("別テナント作成に失敗");

   let repo = PostgresUserRepository::new(pool);
   let email = Email::new("test@example.com").unwrap();

   // 別テナントからは取得できない
   let result = repo.find_by_email(&other_tenant_id, &email).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_idでユーザーを取得できる(pool: PgPool) {
   let (_tenant_id, user_id) = setup_test_data(&pool).await;
   let repo = PostgresUserRepository::new(pool);

   let result = repo.find_by_id(&user_id).await;

   assert!(result.is_ok());
   let user = result.unwrap();
   assert!(user.is_some());
   let user = user.unwrap();
   assert_eq!(user.id(), &user_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ユーザーとロールを一緒に取得できる(pool: PgPool) {
   let (_tenant_id, user_id) = setup_test_data(&pool).await;
   assign_role(&pool, &user_id).await;
   let repo = PostgresUserRepository::new(pool);

   let result = repo.find_with_roles(&user_id).await;

   assert!(result.is_ok());
   let data = result.unwrap();
   assert!(data.is_some());
   let (user, roles) = data.unwrap();
   assert_eq!(user.id(), &user_id);
   assert!(!roles.is_empty());
   assert_eq!(roles[0].name(), "user");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_最終ログイン日時を更新できる(pool: PgPool) {
   let (_tenant_id, user_id) = setup_test_data(&pool).await;
   let repo = PostgresUserRepository::new(pool.clone());

   // 更新前は last_login_at が None
   let user_before = repo.find_by_id(&user_id).await.unwrap().unwrap();
   assert!(user_before.last_login_at().is_none());

   // 更新
   let result = repo.update_last_login(&user_id).await;
   assert!(result.is_ok());

   // 更新後は last_login_at が Some
   let user_after = repo.find_by_id(&user_id).await.unwrap().unwrap();
   assert!(user_after.last_login_at().is_some());
}
