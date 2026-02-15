//! DB コネクション管理の統合テスト
//!
//! PostgreSQL セッション変数（`set_config` / `current_setting`）のみ使用し、
//! テーブルへのアクセスは不要。
//!
//! 実行方法:
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-infra --test db_test
//! ```

use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::db::{self, TenantConnection};
use uuid::Uuid;

/// テスト用の DATABASE_URL
fn database_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set (check backend/.env)")
}

/// テスト用のプールを作成する（max_connections=1 で同一物理接続の再取得を保証）
async fn create_test_pool() -> sqlx::PgPool {
    db::pool_options()
        .max_connections(1)
        .connect(&database_url())
        .await
        .unwrap()
}

// =============================================================================
// after_release フック
// =============================================================================

#[tokio::test]
async fn test_after_releaseでtenant_idがリセットされる() {
    let sut = create_test_pool().await;

    // コネクションを取得し、tenant_id を設定
    {
        let mut conn = sut.acquire().await.unwrap();
        sqlx::query("SELECT set_config('app.tenant_id', 'test-tenant-id', false)")
            .execute(&mut *conn)
            .await
            .unwrap();

        // 設定されていることを確認
        let row: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id')")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert_eq!(row.0, "test-tenant-id");
    }
    // ここで conn がドロップ → after_release が実行される

    // Act: 同じ物理接続を再取得
    let mut conn = sut.acquire().await.unwrap();

    // Assert: tenant_id がリセットされている
    let row: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id')")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(row.0, "");
}

// =============================================================================
// TenantConnection
// =============================================================================

#[tokio::test]
async fn test_acquireでテナントidがセッション変数に設定される() {
    let pool = create_test_pool().await;
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());

    // Act
    let mut sut = TenantConnection::acquire(&pool, &tenant_id).await.unwrap();

    // Assert: セッション変数にテナント ID が設定されている
    let row: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id')")
        .fetch_one(&mut *sut)
        .await
        .unwrap();
    assert_eq!(row.0, tenant_id.to_string());

    // tenant_id() アクセサも正しい値を返す
    assert_eq!(sut.tenant_id(), &tenant_id);
}

#[tokio::test]
async fn test_drop後に接続がプールに返却されtenant_idがリセットされる() {
    let pool = create_test_pool().await;
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());

    // TenantConnection を取得してドロップ
    {
        let sut = TenantConnection::acquire(&pool, &tenant_id).await.unwrap();
        assert_eq!(sut.tenant_id(), &tenant_id);
    }
    // ここでドロップ → after_release が実行される

    // 同じ物理接続を再取得
    let mut conn = pool.acquire().await.unwrap();

    // Assert: tenant_id がリセットされている
    let row: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id')")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(row.0, "");
}

#[tokio::test]
async fn test_異なるテナントで連続してacquireできる() {
    let pool = create_test_pool().await;
    let tenant_a = TenantId::from_uuid(Uuid::now_v7());
    let tenant_b = TenantId::from_uuid(Uuid::now_v7());

    // テナント A で取得
    {
        let mut conn = TenantConnection::acquire(&pool, &tenant_a).await.unwrap();
        let row: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id')")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert_eq!(row.0, tenant_a.to_string());
    }

    // テナント B で取得（同じ物理接続が再利用される）
    {
        let mut conn = TenantConnection::acquire(&pool, &tenant_b).await.unwrap();
        let row: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id')")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert_eq!(row.0, tenant_b.to_string());
    }
}

// =============================================================================
// コンパイル時検証
// =============================================================================

/// TenantConnection が Send を実装していることをコンパイル時に検証する。
/// tokio::spawn で非同期タスクに渡すために必要。
#[allow(dead_code)]
fn assert_send<T: Send>() {}

#[test]
fn test_tenant_connectionはsendを実装している() {
    assert_send::<TenantConnection>();
}
