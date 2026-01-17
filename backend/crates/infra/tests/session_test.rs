//! SessionManager 統合テスト
//!
//! Redis を使用したテスト。テストごとにキーをクリーンアップする。
//!
//! 実行方法:
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-infra --test session_test
//! ```

use chrono::Utc;
use ringiflow_domain::{tenant::TenantId, user::UserId};
use ringiflow_infra::session::{RedisSessionManager, SessionData, SessionManager};
use uuid::Uuid;

/// テスト用の Redis URL
fn redis_url() -> String {
   std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:16379".to_string())
}

/// テスト用のセッションデータを作成
fn test_session_data(tenant_id: &TenantId, user_id: &UserId) -> SessionData {
   SessionData::new(
      user_id.clone(),
      tenant_id.clone(),
      "test@example.com".to_string(),
      "Test User".to_string(),
      vec!["user".to_string()],
   )
}

/// テスト後にセッションをクリーンアップ
async fn cleanup_session(manager: &impl SessionManager, tenant_id: &TenantId, session_id: &str) {
   let _ = manager.delete(tenant_id, session_id).await;
}

#[tokio::test]
async fn test_セッションを作成できる() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   let user_id = UserId::from_uuid(Uuid::now_v7());
   let data = test_session_data(&tenant_id, &user_id);

   let result = manager.create(&data).await;

   assert!(result.is_ok());
   let session_id = result.unwrap();
   assert!(!session_id.is_empty());

   cleanup_session(&manager, &tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_セッションを取得できる() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   let user_id = UserId::from_uuid(Uuid::now_v7());
   let data = test_session_data(&tenant_id, &user_id);

   let session_id = manager.create(&data).await.unwrap();
   let result = manager.get(&tenant_id, &session_id).await;

   assert!(result.is_ok());
   let retrieved = result.unwrap();
   assert!(retrieved.is_some());
   let retrieved = retrieved.unwrap();
   assert_eq!(retrieved.user_id(), &user_id);
   assert_eq!(retrieved.email(), "test@example.com");
   assert_eq!(retrieved.name(), "Test User");
   assert_eq!(retrieved.roles(), &["user".to_string()]);

   cleanup_session(&manager, &tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_存在しないセッションはnoneを返す() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());

   let result = manager.get(&tenant_id, "nonexistent-session-id").await;

   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_セッションを削除できる() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   let user_id = UserId::from_uuid(Uuid::now_v7());
   let data = test_session_data(&tenant_id, &user_id);

   let session_id = manager.create(&data).await.unwrap();
   let result = manager.delete(&tenant_id, &session_id).await;

   assert!(result.is_ok());
}

#[tokio::test]
async fn test_削除後のセッションはnoneを返す() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   let user_id = UserId::from_uuid(Uuid::now_v7());
   let data = test_session_data(&tenant_id, &user_id);

   let session_id = manager.create(&data).await.unwrap();
   manager.delete(&tenant_id, &session_id).await.unwrap();

   let result = manager.get(&tenant_id, &session_id).await;
   assert!(result.is_ok());
   assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_テナント単位で全セッションを削除できる() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   let user_id1 = UserId::from_uuid(Uuid::now_v7());
   let user_id2 = UserId::from_uuid(Uuid::now_v7());

   // 同一テナントに複数セッションを作成
   let data1 = test_session_data(&tenant_id, &user_id1);
   let data2 = test_session_data(&tenant_id, &user_id2);
   let session_id1 = manager.create(&data1).await.unwrap();
   let session_id2 = manager.create(&data2).await.unwrap();

   // テナント単位で削除
   let result = manager.delete_all_for_tenant(&tenant_id).await;
   assert!(result.is_ok());

   // 両方のセッションが削除されている
   assert!(
      manager
         .get(&tenant_id, &session_id1)
         .await
         .unwrap()
         .is_none()
   );
   assert!(
      manager
         .get(&tenant_id, &session_id2)
         .await
         .unwrap()
         .is_none()
   );
}

#[tokio::test]
async fn test_別テナントのセッションは削除されない() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id1 = TenantId::from_uuid(Uuid::now_v7());
   let tenant_id2 = TenantId::from_uuid(Uuid::now_v7());
   let user_id1 = UserId::from_uuid(Uuid::now_v7());
   let user_id2 = UserId::from_uuid(Uuid::now_v7());

   // 異なるテナントにセッションを作成
   let data1 = test_session_data(&tenant_id1, &user_id1);
   let data2 = test_session_data(&tenant_id2, &user_id2);
   let session_id1 = manager.create(&data1).await.unwrap();
   let session_id2 = manager.create(&data2).await.unwrap();

   // テナント1のセッションのみ削除
   manager.delete_all_for_tenant(&tenant_id1).await.unwrap();

   // テナント1のセッションは削除、テナント2は残っている
   assert!(
      manager
         .get(&tenant_id1, &session_id1)
         .await
         .unwrap()
         .is_none()
   );
   assert!(
      manager
         .get(&tenant_id2, &session_id2)
         .await
         .unwrap()
         .is_some()
   );

   cleanup_session(&manager, &tenant_id2, &session_id2).await;
}

#[tokio::test]
async fn test_セッションに有効期限が設定される() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   let user_id = UserId::from_uuid(Uuid::now_v7());
   let data = test_session_data(&tenant_id, &user_id);

   let session_id = manager.create(&data).await.unwrap();

   // TTL が設定されていることを確認（28800秒 = 8時間）
   // 正確な値ではなく、TTL が存在することを確認
   let ttl = manager.get_ttl(&tenant_id, &session_id).await.unwrap();
   assert!(ttl.is_some());
   let ttl = ttl.unwrap();
   // TTL は 28800 秒以下で、0 より大きい
   assert!(ttl > 0);
   assert!(ttl <= 28800);

   cleanup_session(&manager, &tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_created_atとlast_accessed_atが設定される() {
   let manager = RedisSessionManager::new(&redis_url()).await.unwrap();
   let tenant_id = TenantId::from_uuid(Uuid::now_v7());
   let user_id = UserId::from_uuid(Uuid::now_v7());
   let before = Utc::now();
   let data = test_session_data(&tenant_id, &user_id);

   let session_id = manager.create(&data).await.unwrap();
   let retrieved = manager.get(&tenant_id, &session_id).await.unwrap().unwrap();
   let after = Utc::now();

   // created_at と last_accessed_at が適切な範囲内
   assert!(retrieved.created_at() >= before);
   assert!(retrieved.created_at() <= after);
   assert!(retrieved.last_accessed_at() >= before);
   assert!(retrieved.last_accessed_at() <= after);

   cleanup_session(&manager, &tenant_id, &session_id).await;
}
