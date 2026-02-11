//! DynamoDB 接続・テーブル自動作成の統合テスト
//!
//! DynamoDB Local を使用したテスト。
//!
//! 実行方法:
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-infra --test dynamodb_test
//! ```

use ringiflow_infra::dynamodb;

/// テスト用の DynamoDB エンドポイント
///
/// 優先順位:
/// 1. `DYNAMODB_ENDPOINT`（CI で明示的に設定）
/// 2. `DYNAMODB_PORT` から構築（justfile が root `.env` から渡す）
/// 3. フォールバック: `http://localhost:18000`
fn dynamodb_endpoint() -> String {
   std::env::var("DYNAMODB_ENDPOINT").unwrap_or_else(|_| {
      let port = std::env::var("DYNAMODB_PORT").unwrap_or_else(|_| "18000".to_string());
      format!("http://localhost:{port}")
   })
}

#[tokio::test]
async fn test_create_clientがエンドポイントに接続できる() {
   let client = dynamodb::create_client(&dynamodb_endpoint()).await;

   // ListTables が呼べれば接続成功
   let result = client.list_tables().send().await;
   assert!(
      result.is_ok(),
      "DynamoDB への接続に失敗: {:?}",
      result.err()
   );
}

#[tokio::test]
async fn test_ensure_audit_log_tableが初回呼び出しでテーブルを作成する() {
   let client = dynamodb::create_client(&dynamodb_endpoint()).await;

   // ランダムなテーブル名で分離（他テストとの競合を防止）
   let table_name = format!("test_audit_logs_{}", uuid::Uuid::now_v7());

   let result = dynamodb::ensure_audit_log_table(&client, &table_name).await;
   assert!(result.is_ok(), "テーブル作成に失敗: {:?}", result.err());

   // テーブルが存在することを確認
   let describe = client.describe_table().table_name(&table_name).send().await;
   assert!(describe.is_ok(), "テーブルが存在しません");

   let table = describe.unwrap().table.unwrap();
   let key_schema = table.key_schema();

   // PK: tenant_id (HASH)
   assert!(
      key_schema
         .iter()
         .any(|ks| ks.attribute_name() == "tenant_id"
            && ks.key_type == aws_sdk_dynamodb::types::KeyType::Hash),
      "tenant_id HASH キーが見つかりません"
   );

   // SK: sk (RANGE)
   assert!(
      key_schema
         .iter()
         .any(|ks| ks.attribute_name() == "sk"
            && ks.key_type == aws_sdk_dynamodb::types::KeyType::Range),
      "sk RANGE キーが見つかりません"
   );

   // クリーンアップ
   let _ = client.delete_table().table_name(&table_name).send().await;
}

#[tokio::test]
async fn test_ensure_audit_log_tableが既存テーブルに対して冪等に動作する() {
   let client = dynamodb::create_client(&dynamodb_endpoint()).await;
   let table_name = format!("test_audit_logs_{}", uuid::Uuid::now_v7());

   // 1回目: テーブル作成
   let result1 = dynamodb::ensure_audit_log_table(&client, &table_name).await;
   assert!(
      result1.is_ok(),
      "1回目のテーブル作成に失敗: {:?}",
      result1.err()
   );

   // 2回目: 冪等に動作（エラーにならない）
   let result2 = dynamodb::ensure_audit_log_table(&client, &table_name).await;
   assert!(
      result2.is_ok(),
      "2回目の呼び出しでエラー: {:?}",
      result2.err()
   );

   // クリーンアップ
   let _ = client.delete_table().table_name(&table_name).send().await;
}
