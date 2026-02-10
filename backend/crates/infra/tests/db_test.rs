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

use ringiflow_infra::db;

/// テスト用の DATABASE_URL
fn database_url() -> String {
   dotenvy::dotenv().ok();
   std::env::var("DATABASE_URL").expect("DATABASE_URL must be set (check backend/.env)")
}

#[tokio::test]
async fn test_after_releaseでtenant_idがリセットされる() {
   // Arrange: max_connections=1 で同一物理接続の再取得を保証
   let sut = db::pool_options()
      .max_connections(1)
      .connect(&database_url())
      .await
      .unwrap();

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
