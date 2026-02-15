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
///
/// 優先順位:
/// 1. `REDIS_URL`（CI で明示的に設定）
/// 2. `REDIS_PORT` から構築（justfile が root `.env` から渡す）
/// 3. フォールバック: `redis://localhost:16379`
fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| {
        let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "16379".to_string());
        format!("redis://localhost:{port}")
    })
}

/// テスト用のセッションデータを作成
fn test_session_data(tenant_id: &TenantId, user_id: &UserId) -> SessionData {
    SessionData::new(
        user_id.clone(),
        tenant_id.clone(),
        "test@example.com".to_string(),
        "Test User".to_string(),
        vec!["user".to_string()],
        vec!["workflow:read".to_string()],
    )
}

/// テスト後にセッションをクリーンアップ
async fn cleanup_session(sm: &impl SessionManager, tenant_id: &TenantId, session_id: &str) {
    let _ = sm.delete(tenant_id, session_id).await;
}

#[tokio::test]
async fn test_セッションを作成できる() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let user_id = UserId::from_uuid(Uuid::now_v7());
    let data = test_session_data(&tenant_id, &user_id);

    let result = sut.create(&data).await;

    assert!(result.is_ok());
    let session_id = result.unwrap();
    assert!(!session_id.is_empty());

    cleanup_session(&sut, &tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_セッションを取得できる() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let user_id = UserId::from_uuid(Uuid::now_v7());
    let data = test_session_data(&tenant_id, &user_id);

    let session_id = sut.create(&data).await.unwrap();
    let result = sut.get(&tenant_id, &session_id).await;

    assert!(result.is_ok());
    let retrieved = result.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.user_id(), &user_id);
    assert_eq!(retrieved.email(), "test@example.com");
    assert_eq!(retrieved.name(), "Test User");
    assert_eq!(retrieved.roles(), &["user".to_string()]);

    cleanup_session(&sut, &tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_存在しないセッションはnoneを返す() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());

    let result = sut.get(&tenant_id, "nonexistent-session-id").await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_セッションを削除できる() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let user_id = UserId::from_uuid(Uuid::now_v7());
    let data = test_session_data(&tenant_id, &user_id);

    let session_id = sut.create(&data).await.unwrap();
    let result = sut.delete(&tenant_id, &session_id).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_削除後のセッションはnoneを返す() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let user_id = UserId::from_uuid(Uuid::now_v7());
    let data = test_session_data(&tenant_id, &user_id);

    let session_id = sut.create(&data).await.unwrap();

    // CSRF トークンも作成
    sut.create_csrf_token(&tenant_id, &session_id)
        .await
        .unwrap();

    sut.delete(&tenant_id, &session_id).await.unwrap();

    // セッションが削除されている
    let result = sut.get(&tenant_id, &session_id).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    // CSRF トークンも自動的に削除されている
    assert!(
        sut.get_csrf_token(&tenant_id, &session_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_テナント単位で全セッションを削除できる() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let user_id1 = UserId::from_uuid(Uuid::now_v7());
    let user_id2 = UserId::from_uuid(Uuid::now_v7());

    // 同一テナントに複数セッションを作成
    let data1 = test_session_data(&tenant_id, &user_id1);
    let data2 = test_session_data(&tenant_id, &user_id2);
    let session_id1 = sut.create(&data1).await.unwrap();
    let session_id2 = sut.create(&data2).await.unwrap();

    // 各セッションに CSRF トークンを作成
    sut.create_csrf_token(&tenant_id, &session_id1)
        .await
        .unwrap();
    sut.create_csrf_token(&tenant_id, &session_id2)
        .await
        .unwrap();

    // テナント単位で削除
    let result = sut.delete_all_for_tenant(&tenant_id).await;
    assert!(result.is_ok());

    // 両方のセッションが削除されている
    assert!(sut.get(&tenant_id, &session_id1).await.unwrap().is_none());
    assert!(sut.get(&tenant_id, &session_id2).await.unwrap().is_none());

    // 両方の CSRF トークンも削除されている
    assert!(
        sut.get_csrf_token(&tenant_id, &session_id1)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        sut.get_csrf_token(&tenant_id, &session_id2)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_別テナントのセッションは削除されない() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id1 = TenantId::from_uuid(Uuid::now_v7());
    let tenant_id2 = TenantId::from_uuid(Uuid::now_v7());
    let user_id1 = UserId::from_uuid(Uuid::now_v7());
    let user_id2 = UserId::from_uuid(Uuid::now_v7());

    // 異なるテナントにセッションを作成
    let data1 = test_session_data(&tenant_id1, &user_id1);
    let data2 = test_session_data(&tenant_id2, &user_id2);
    let session_id1 = sut.create(&data1).await.unwrap();
    let session_id2 = sut.create(&data2).await.unwrap();

    // テナント1のセッションのみ削除
    sut.delete_all_for_tenant(&tenant_id1).await.unwrap();

    // テナント1のセッションは削除、テナント2は残っている
    assert!(sut.get(&tenant_id1, &session_id1).await.unwrap().is_none());
    assert!(sut.get(&tenant_id2, &session_id2).await.unwrap().is_some());

    cleanup_session(&sut, &tenant_id2, &session_id2).await;
}

#[tokio::test]
async fn test_セッションに有効期限が設定される() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let user_id = UserId::from_uuid(Uuid::now_v7());
    let data = test_session_data(&tenant_id, &user_id);

    let session_id = sut.create(&data).await.unwrap();

    // TTL が設定されていることを確認（28800秒 = 8時間）
    // 正確な値ではなく、TTL が存在することを確認
    let ttl = sut.get_ttl(&tenant_id, &session_id).await.unwrap();
    assert!(ttl.is_some());
    let ttl = ttl.unwrap();
    // TTL は 28800 秒以下で、0 より大きい
    assert!(ttl > 0);
    assert!(ttl <= 28800);

    cleanup_session(&sut, &tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_created_atとlast_accessed_atが設定される() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let user_id = UserId::from_uuid(Uuid::now_v7());
    let before = Utc::now();
    let data = test_session_data(&tenant_id, &user_id);

    let session_id = sut.create(&data).await.unwrap();
    let retrieved = sut.get(&tenant_id, &session_id).await.unwrap().unwrap();
    let after = Utc::now();

    // created_at と last_accessed_at が適切な範囲内
    assert!(retrieved.created_at() >= before);
    assert!(retrieved.created_at() <= after);
    assert!(retrieved.last_accessed_at() >= before);
    assert!(retrieved.last_accessed_at() <= after);

    cleanup_session(&sut, &tenant_id, &session_id).await;
}

// --- CSRF トークンテスト ---

/// テスト後に CSRF トークンをクリーンアップ
async fn cleanup_csrf(sm: &impl SessionManager, tenant_id: &TenantId, session_id: &str) {
    let _ = sm.delete_csrf_token(tenant_id, session_id).await;
}

#[tokio::test]
async fn test_csrfトークンを作成できる() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let session_id = Uuid::now_v7().to_string();

    let result = sut.create_csrf_token(&tenant_id, &session_id).await;

    assert!(result.is_ok());
    let token = result.unwrap();
    // 64文字の hex 文字列
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));

    cleanup_csrf(&sut, &tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_csrfトークンを取得できる() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let session_id = Uuid::now_v7().to_string();

    let token = sut
        .create_csrf_token(&tenant_id, &session_id)
        .await
        .unwrap();
    let result = sut.get_csrf_token(&tenant_id, &session_id).await;

    assert!(result.is_ok());
    let retrieved = result.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), token);

    cleanup_csrf(&sut, &tenant_id, &session_id).await;
}

#[tokio::test]
async fn test_存在しないcsrfトークンはnoneを返す() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());

    let result = sut
        .get_csrf_token(&tenant_id, "nonexistent-session-id")
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_csrfトークンを削除できる() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let session_id = Uuid::now_v7().to_string();

    sut.create_csrf_token(&tenant_id, &session_id)
        .await
        .unwrap();
    let result = sut.delete_csrf_token(&tenant_id, &session_id).await;

    assert!(result.is_ok());

    // 削除後は None を返す
    let retrieved = sut.get_csrf_token(&tenant_id, &session_id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_テナント単位で全csrfトークンを削除できる() {
    let sut = RedisSessionManager::new(&redis_url()).await.unwrap();
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    let session_id1 = Uuid::now_v7().to_string();
    let session_id2 = Uuid::now_v7().to_string();

    // 同一テナントに複数の CSRF トークンを作成
    sut.create_csrf_token(&tenant_id, &session_id1)
        .await
        .unwrap();
    sut.create_csrf_token(&tenant_id, &session_id2)
        .await
        .unwrap();

    // テナント単位で削除
    let result = sut.delete_all_csrf_for_tenant(&tenant_id).await;
    assert!(result.is_ok());

    // 両方のトークンが削除されている
    assert!(
        sut.get_csrf_token(&tenant_id, &session_id1)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        sut.get_csrf_token(&tenant_id, &session_id2)
            .await
            .unwrap()
            .is_none()
    );
}
