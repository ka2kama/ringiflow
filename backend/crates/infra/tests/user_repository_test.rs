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

mod common;

use common::{assign_role, setup_test_data};
use ringiflow_domain::{
   tenant::TenantId,
   user::{Email, UserId},
};
use ringiflow_infra::repository::{PostgresUserRepository, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn test_メールアドレスでユーザーを取得できる(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);
   let email = Email::new("test@example.com").unwrap();

   let result = sut.find_by_email(&tenant_id, &email).await;

   assert!(result.is_ok());
   let user = result.unwrap();
   assert!(user.is_some());
   let user = user.unwrap();
   assert_eq!(user.email().as_str(), "test@example.com");
   assert_eq!(user.name().as_str(), "Test User");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_存在しないメールアドレスの場合noneを返す(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);
   let email = Email::new("nonexistent@example.com").unwrap();

   let result = sut.find_by_email(&tenant_id, &email).await;

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

   let sut = PostgresUserRepository::new(pool);
   let email = Email::new("test@example.com").unwrap();

   // 別テナントからは取得できない
   let result = sut.find_by_email(&other_tenant_id, &email).await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_idでユーザーを取得できる(pool: PgPool) {
   let (_tenant_id, user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_by_id(&user_id).await;

   assert!(result.is_ok());
   let user = result.unwrap();
   assert!(user.is_some());
   let user = user.unwrap();
   assert_eq!(user.id(), &user_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_ユーザーとロールを一緒に取得できる(pool: PgPool) {
   let (tenant_id, user_id) = setup_test_data(&pool).await;
   assign_role(&pool, &user_id, &tenant_id).await;
   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_with_roles(&user_id).await;

   assert!(result.is_ok());
   let data = result.unwrap();
   assert!(data.is_some());
   let (user, roles) = data.unwrap();
   assert_eq!(user.id(), &user_id);
   assert!(!roles.is_empty());
   assert_eq!(roles[0].name(), "user");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_with_roles_別テナントのロール割り当ては含まれない(
   pool: PgPool,
) {
   let (tenant_id, user_id) = setup_test_data(&pool).await;

   // 自テナントでロールを割り当て
   assign_role(&pool, &user_id, &tenant_id).await;

   // 別テナントを作成
   let other_tenant_id = TenantId::from_uuid(Uuid::now_v7());
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

   // 別テナント用のロールを作成し、同じユーザーに割り当て
   // （一意制約 (user_id, role_id) があるため別ロールが必要）
   let other_role_id = Uuid::now_v7();
   sqlx::query!(
      r#"
        INSERT INTO roles (id, tenant_id, name, description, permissions, is_system)
        VALUES ($1, $2, 'other-admin', 'Other tenant admin', '["admin"]'::jsonb, false)
        "#,
      other_role_id,
      other_tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("別テナントロール作成に失敗");

   sqlx::query!(
      r#"
        INSERT INTO user_roles (user_id, role_id, tenant_id)
        VALUES ($1, $2, $3)
        "#,
      user_id.as_uuid(),
      other_role_id,
      other_tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("別テナントロール割り当てに失敗");

   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_with_roles(&user_id).await;

   assert!(result.is_ok());
   let (user, roles) = result.unwrap().unwrap();
   assert_eq!(user.id(), &user_id);
   // 自テナントのロールのみ取得（別テナントのロール割り当ては含まれない）
   assert_eq!(roles.len(), 1);
   assert_eq!(roles[0].name(), "user");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_複数idでユーザーを一括取得できる(pool: PgPool) {
   let (tenant_id, user_id1) = setup_test_data(&pool).await;

   // 2人目のユーザーを追加
   let user_id2 = UserId::from_uuid(Uuid::now_v7());
   sqlx::query!(
      r#"
        INSERT INTO users (id, tenant_id, display_number, email, name, status)
        VALUES ($1, $2, 2, 'user2@example.com', 'User Two', 'active')
        "#,
      user_id2.as_uuid(),
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("ユーザー2作成に失敗");

   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_by_ids(&[user_id1.clone(), user_id2.clone()]).await;

   assert!(result.is_ok());
   let users = result.unwrap();
   assert_eq!(users.len(), 2);
   let ids: Vec<&UserId> = users.iter().map(|u| u.id()).collect();
   assert!(ids.contains(&&user_id1));
   assert!(ids.contains(&&user_id2));
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_存在しないidが含まれても取得できるものだけ返す(pool: PgPool) {
   let (_tenant_id, user_id) = setup_test_data(&pool).await;
   let nonexistent_id = UserId::from_uuid(Uuid::now_v7());
   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_by_ids(&[user_id.clone(), nonexistent_id]).await;

   assert!(result.is_ok());
   let users = result.unwrap();
   assert_eq!(users.len(), 1);
   assert_eq!(users[0].id(), &user_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_空のid配列を渡すと空vecを返す(pool: PgPool) {
   let (_tenant_id, _user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_by_ids(&[]).await;

   assert!(result.is_ok());
   let users = result.unwrap();
   assert!(users.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_最終ログイン日時を更新できる(pool: PgPool) {
   let (_tenant_id, user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool.clone());

   // 更新前は last_login_at が None
   let user_before = sut.find_by_id(&user_id).await.unwrap().unwrap();
   assert!(user_before.last_login_at().is_none());

   // 更新
   let result = sut.update_last_login(&user_id).await;
   assert!(result.is_ok());

   // 更新後は last_login_at が Some
   let user_after = sut.find_by_id(&user_id).await.unwrap().unwrap();
   assert!(user_after.last_login_at().is_some());
}

// ===== find_all_active_by_tenant テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_テナント内のアクティブユーザー一覧を取得できる(pool: PgPool) {
   let (tenant_id, user_id1) = setup_test_data(&pool).await;

   // 2人目のユーザーを追加
   let user_id2 = UserId::from_uuid(Uuid::now_v7());
   sqlx::query!(
      r#"
        INSERT INTO users (id, tenant_id, display_number, email, name, status)
        VALUES ($1, $2, 2, 'user2@example.com', 'User Two', 'active')
        "#,
      user_id2.as_uuid(),
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("ユーザー2作成に失敗");

   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_all_active_by_tenant(&tenant_id).await;

   assert!(result.is_ok());
   let users = result.unwrap();
   assert_eq!(users.len(), 2);
   let ids: Vec<&UserId> = users.iter().map(|u| u.id()).collect();
   assert!(ids.contains(&&user_id1));
   assert!(ids.contains(&&user_id2));
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_非アクティブユーザーは除外される(pool: PgPool) {
   let (tenant_id, _active_user_id) = setup_test_data(&pool).await;

   // 非アクティブユーザーを追加
   let inactive_user_id = UserId::from_uuid(Uuid::now_v7());
   sqlx::query!(
      r#"
        INSERT INTO users (id, tenant_id, display_number, email, name, status)
        VALUES ($1, $2, 2, 'inactive@example.com', 'Inactive User', 'inactive')
        "#,
      inactive_user_id.as_uuid(),
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("非アクティブユーザー作成に失敗");

   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_all_active_by_tenant(&tenant_id).await;

   assert!(result.is_ok());
   let users = result.unwrap();
   // アクティブユーザーのみ
   assert_eq!(users.len(), 1);
   assert_eq!(users[0].status().to_string(), "active");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_他テナントのユーザーは含まれない(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;

   // 別テナントを作成
   let other_tenant_id = TenantId::from_uuid(Uuid::now_v7());
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

   // 別テナントのユーザーを追加
   let other_user_id = UserId::from_uuid(Uuid::now_v7());
   sqlx::query!(
      r#"
        INSERT INTO users (id, tenant_id, display_number, email, name, status)
        VALUES ($1, $2, 1, 'other@example.com', 'Other User', 'active')
        "#,
      other_user_id.as_uuid(),
      other_tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("別テナントユーザー作成に失敗");

   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_all_active_by_tenant(&tenant_id).await;

   assert!(result.is_ok());
   let users = result.unwrap();
   // 自テナントのユーザーのみ
   assert_eq!(users.len(), 1);
   assert_eq!(users[0].email().as_str(), "test@example.com");
}
