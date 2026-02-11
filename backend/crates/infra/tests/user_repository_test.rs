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
   user::{Email, User, UserId, UserStatus},
   value_objects::{DisplayNumber, UserName},
};
use ringiflow_infra::repository::{PostgresUserRepository, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

/// リポジトリ経由でテストユーザーを作成するヘルパー
async fn insert_test_user(
   sut: &PostgresUserRepository,
   tenant_id: &TenantId,
   display_number: i64,
   email: &str,
   name: &str,
) -> User {
   let now = chrono::Utc::now();
   let user = User::new(
      UserId::new(),
      tenant_id.clone(),
      DisplayNumber::new(display_number).unwrap(),
      Email::new(email).unwrap(),
      UserName::new(name).unwrap(),
      now,
   );
   sut.insert(&user).await.expect("ユーザー挿入に失敗");
   user
}

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

// ===== insert テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_insertでユーザーを挿入しfind_by_idで取得できる(pool: PgPool) {
   // テナント作成（setup_test_data はユーザーも作るので、テナントだけ別に作る）
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   sqlx::query!(
      r#"
        INSERT INTO tenants (id, name, subdomain, plan, status)
        VALUES ($1, 'Test Tenant', 'test-insert', 'free', 'active')
        "#,
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("テナント作成に失敗");

   let sut = PostgresUserRepository::new(pool);
   let user = insert_test_user(&sut, &tenant_id, 100, "new@example.com", "New User").await;

   let result = sut.find_by_id(user.id()).await;

   assert!(result.is_ok());
   let found = result.unwrap().unwrap();
   assert_eq!(found.id(), user.id());
   assert_eq!(found.email().as_str(), "new@example.com");
   assert_eq!(found.name().as_str(), "New User");
   assert_eq!(found.display_number().as_i64(), 100);
   assert!(found.is_active());
}

// ===== find_by_display_number テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_表示用連番でテナント内のユーザーを検索できる(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   let result = sut
      .find_by_display_number(&tenant_id, DisplayNumber::new(1).unwrap())
      .await;

   assert!(result.is_ok());
   let user = result.unwrap().unwrap();
   assert_eq!(user.email().as_str(), "test@example.com");
   assert_eq!(user.display_number().as_i64(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_存在しない表示用連番はnoneを返す(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   let result = sut
      .find_by_display_number(&tenant_id, DisplayNumber::new(999).unwrap())
      .await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

// ===== find_all_by_tenant テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_all_by_tenantでステータスフィルタが機能する(pool: PgPool) {
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

   // active のみ
   let active_users = sut
      .find_all_by_tenant(&tenant_id, Some(UserStatus::Active))
      .await
      .unwrap();
   assert_eq!(active_users.len(), 1);
   assert_eq!(active_users[0].status(), UserStatus::Active);

   // inactive のみ
   let inactive_users = sut
      .find_all_by_tenant(&tenant_id, Some(UserStatus::Inactive))
      .await
      .unwrap();
   assert_eq!(inactive_users.len(), 1);
   assert_eq!(inactive_users[0].status(), UserStatus::Inactive);

   // フィルタなし（deleted 以外すべて）
   let all_users = sut.find_all_by_tenant(&tenant_id, None).await.unwrap();
   assert_eq!(all_users.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_all_by_tenantでdeletedユーザーは除外される(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;

   // 削除済みユーザーを追加
   sqlx::query!(
      r#"
        INSERT INTO users (id, tenant_id, display_number, email, name, status)
        VALUES ($1, $2, 2, 'deleted@example.com', 'Deleted User', 'deleted')
        "#,
      Uuid::now_v7(),
      tenant_id.as_uuid()
   )
   .execute(&pool)
   .await
   .expect("削除済みユーザー作成に失敗");

   let sut = PostgresUserRepository::new(pool);

   let users = sut.find_all_by_tenant(&tenant_id, None).await.unwrap();
   // deleted は除外される
   assert_eq!(users.len(), 1);
   assert_eq!(users[0].email().as_str(), "test@example.com");
}

// ===== update テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_updateでユーザー名が更新される(pool: PgPool) {
   let (tenant_id, _user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   let user = sut
      .find_by_display_number(&tenant_id, DisplayNumber::new(1).unwrap())
      .await
      .unwrap()
      .unwrap();

   let now = chrono::Utc::now();
   let updated_user = user.with_name(UserName::new("Updated Name").unwrap(), now);
   sut.update(&updated_user).await.unwrap();

   let found = sut.find_by_id(updated_user.id()).await.unwrap().unwrap();
   assert_eq!(found.name().as_str(), "Updated Name");
}

// ===== update_status テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_statusでステータスが更新される(pool: PgPool) {
   let (_tenant_id, user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   let user = sut.find_by_id(&user_id).await.unwrap().unwrap();
   assert_eq!(user.status(), UserStatus::Active);

   let now = chrono::Utc::now();
   let deactivated = user.with_status(UserStatus::Inactive, now);
   sut.update_status(&deactivated).await.unwrap();

   let found = sut.find_by_id(&user_id).await.unwrap().unwrap();
   assert_eq!(found.status(), UserStatus::Inactive);
}

// ===== insert_user_role テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_insert_user_roleでロールを割り当てられる(pool: PgPool) {
   let (tenant_id, user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   // システムロール "user" を取得
   let role = sut.find_role_by_name("user").await.unwrap().unwrap();

   sut.insert_user_role(&user_id, role.id(), &tenant_id)
      .await
      .unwrap();

   // find_with_roles で確認
   let (_, roles) = sut.find_with_roles(&user_id).await.unwrap().unwrap();
   assert_eq!(roles.len(), 1);
   assert_eq!(roles[0].name(), "user");
}

// ===== replace_user_roles テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_replace_user_rolesでロールが置き換わる(pool: PgPool) {
   let (tenant_id, user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   // まず "user" ロールを割り当て
   let user_role = sut.find_role_by_name("user").await.unwrap().unwrap();
   sut.insert_user_role(&user_id, user_role.id(), &tenant_id)
      .await
      .unwrap();

   // "tenant_admin" に置き換え
   let admin_role = sut
      .find_role_by_name("tenant_admin")
      .await
      .unwrap()
      .unwrap();
   sut.replace_user_roles(&user_id, admin_role.id(), &tenant_id)
      .await
      .unwrap();

   let (_, roles) = sut.find_with_roles(&user_id).await.unwrap().unwrap();
   assert_eq!(roles.len(), 1);
   assert_eq!(roles[0].name(), "tenant_admin");
}

// ===== find_role_by_name テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_role_by_nameでシステムロールを検索できる(pool: PgPool) {
   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_role_by_name("user").await;

   assert!(result.is_ok());
   let role = result.unwrap().unwrap();
   assert_eq!(role.name(), "user");
   assert!(role.is_system());
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_role_by_name存在しないロール名はnoneを返す(pool: PgPool) {
   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_role_by_name("nonexistent").await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

// ===== count_active_users_with_role テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_count_active_users_with_roleが正しくカウントする(pool: PgPool) {
   let (tenant_id, user_id) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   // "tenant_admin" ロールを割り当て
   let admin_role = sut
      .find_role_by_name("tenant_admin")
      .await
      .unwrap()
      .unwrap();
   sut.insert_user_role(&user_id, admin_role.id(), &tenant_id)
      .await
      .unwrap();

   // カウント（除外なし）
   let count = sut
      .count_active_users_with_role(&tenant_id, "tenant_admin", None)
      .await
      .unwrap();
   assert_eq!(count, 1);

   // 自身を除外するとカウント 0
   let count = sut
      .count_active_users_with_role(&tenant_id, "tenant_admin", Some(&user_id))
      .await
      .unwrap();
   assert_eq!(count, 0);
}

// ===== find_roles_for_users テスト =====

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_roles_for_usersで複数ユーザーのロールを一括取得できる(
   pool: PgPool,
) {
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

   // user1 に "tenant_admin"、user2 に "user" を割り当て
   let admin_role = sut
      .find_role_by_name("tenant_admin")
      .await
      .unwrap()
      .unwrap();
   let user_role = sut.find_role_by_name("user").await.unwrap().unwrap();
   sut.insert_user_role(&user_id1, admin_role.id(), &tenant_id)
      .await
      .unwrap();
   sut.insert_user_role(&user_id2, user_role.id(), &tenant_id)
      .await
      .unwrap();

   let roles_map = sut
      .find_roles_for_users(&[user_id1.clone(), user_id2.clone()], &tenant_id)
      .await
      .unwrap();

   assert_eq!(roles_map.len(), 2);
   assert_eq!(roles_map[&user_id1], vec!["tenant_admin"]);
   assert_eq!(roles_map[&user_id2], vec!["user"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_roles_for_usersで空配列を渡すと空mapを返す(pool: PgPool) {
   let (tenant_id, _) = setup_test_data(&pool).await;
   let sut = PostgresUserRepository::new(pool);

   let result = sut.find_roles_for_users(&[], &tenant_id).await.unwrap();

   assert!(result.is_empty());
}
