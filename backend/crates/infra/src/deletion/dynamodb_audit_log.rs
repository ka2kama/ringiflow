//! # DynamoDbAuditLogDeleter
//!
//! テナントの監査ログを DynamoDB から削除する。
//!
//! ## 削除方式
//!
//! DynamoDB には `DELETE WHERE` 相当の機能がないため、
//! Query で PK/SK を取得してから BatchWriteItem で 25 件ずつ削除する。

use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_dynamodb::{
    Client,
    types::{AttributeValue, DeleteRequest, WriteRequest},
};
use ringiflow_domain::tenant::TenantId;

use super::{DeletionResult, TenantDeleter};
use crate::error::InfraError;

/// リトライ設定
const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_MS: u64 = 100;
const MAX_BACKOFF_MS: u64 = 5_000;

/// exponential backoff の待機時間を計算する
fn compute_backoff_ms(retry: u32) -> u64 {
    let backoff = INITIAL_BACKOFF_MS.saturating_mul(2u64.pow(retry));
    backoff.min(MAX_BACKOFF_MS)
}

/// DynamoDB 監査ログ Deleter
pub struct DynamoDbAuditLogDeleter {
    client:     Client,
    table_name: String,
}

impl DynamoDbAuditLogDeleter {
    pub fn new(client: Client, table_name: String) -> Self {
        Self { client, table_name }
    }
}

#[async_trait]
impl TenantDeleter for DynamoDbAuditLogDeleter {
    fn name(&self) -> &'static str {
        "dynamodb:audit_logs"
    }

    async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
        let mut deleted_count: u64 = 0;
        let mut exclusive_start_key = None;

        loop {
            // Query で PK/SK を取得（削除対象のキーのみ）
            let mut query = self
                .client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("tenant_id = :tid")
                .expression_attribute_values(
                    ":tid",
                    AttributeValue::S(tenant_id.as_uuid().to_string()),
                )
                .projection_expression("tenant_id, sk");

            if let Some(key) = exclusive_start_key {
                query = query.set_exclusive_start_key(Some(key));
            }

            let output = query
                .send()
                .await
                .map_err(|e| InfraError::dynamo_db(format!("監査ログの検索に失敗: {e}")))?;

            let items = output.items();
            if items.is_empty() {
                break;
            }

            // BatchWriteItem で 25 件ずつ削除（unprocessed_items リトライ付き）
            for chunk in items.chunks(25) {
                let delete_requests: Vec<WriteRequest> = chunk
                    .iter()
                    .map(|item| {
                        let key = vec![
                            (
                                "tenant_id".to_string(),
                                item.get("tenant_id").cloned().unwrap(),
                            ),
                            ("sk".to_string(), item.get("sk").cloned().unwrap()),
                        ]
                        .into_iter()
                        .collect();

                        WriteRequest::builder()
                            .delete_request(
                                DeleteRequest::builder().set_key(Some(key)).build().unwrap(),
                            )
                            .build()
                    })
                    .collect();

                let requested_count = delete_requests.len() as u64;
                let mut remaining_requests = delete_requests;

                for retry in 0..=MAX_RETRIES {
                    if retry > 0 {
                        let backoff = compute_backoff_ms(retry - 1);
                        tracing::warn!(
                            retry = retry,
                            unprocessed = remaining_requests.len(),
                            backoff_ms = backoff,
                            "DynamoDB BatchWriteItem: 未処理アイテムをリトライ"
                        );
                        tokio::time::sleep(Duration::from_millis(backoff)).await;
                    }

                    let output = self
                        .client
                        .batch_write_item()
                        .request_items(&self.table_name, remaining_requests)
                        .send()
                        .await
                        .map_err(|e| InfraError::dynamo_db(format!("監査ログの削除に失敗: {e}")))?;

                    let unprocessed = output
                        .unprocessed_items()
                        .and_then(|items| items.get(&self.table_name))
                        .cloned()
                        .unwrap_or_default();

                    if unprocessed.is_empty() {
                        deleted_count += requested_count;
                        break;
                    }

                    if retry == MAX_RETRIES {
                        let unprocessed_count = unprocessed.len();
                        tracing::error!(
                            unprocessed = unprocessed_count,
                            "DynamoDB BatchWriteItem: リトライ上限超過、未処理アイテムが残存"
                        );
                        return Err(InfraError::dynamo_db(format!(
                            "監査ログの削除でリトライ上限超過: {}件が未処理",
                            unprocessed_count
                        )));
                    }

                    remaining_requests = unprocessed;
                }
            }

            // ページネーション
            exclusive_start_key = output.last_evaluated_key().cloned();
            if exclusive_start_key.is_none() {
                break;
            }
        }

        Ok(DeletionResult { deleted_count })
    }

    async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError> {
        let mut total: u64 = 0;
        let mut exclusive_start_key = None;

        loop {
            let mut query = self
                .client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("tenant_id = :tid")
                .expression_attribute_values(
                    ":tid",
                    AttributeValue::S(tenant_id.as_uuid().to_string()),
                )
                .select(aws_sdk_dynamodb::types::Select::Count);

            if let Some(key) = exclusive_start_key {
                query = query.set_exclusive_start_key(Some(key));
            }

            let output = query
                .send()
                .await
                .map_err(|e| InfraError::dynamo_db(format!("監査ログの件数取得に失敗: {e}")))?;

            total += output.count() as u64;

            exclusive_start_key = output.last_evaluated_key().cloned();
            if exclusive_start_key.is_none() {
                break;
            }
        }

        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nameがdynamodb_audit_logsを返す() {
        let config = aws_sdk_dynamodb::Config::builder()
            .behavior_version_latest()
            .build();
        let client = Client::from_conf(config);
        let sut = DynamoDbAuditLogDeleter::new(client, "test_table".to_string());

        assert_eq!(sut.name(), "dynamodb:audit_logs");
    }

    #[test]
    fn test_send_syncを満たす() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DynamoDbAuditLogDeleter>();
    }

    #[test]
    fn test_compute_backoff_msがリトライ0回目で100msを返す() {
        assert_eq!(compute_backoff_ms(0), 100);
    }

    #[test]
    fn test_compute_backoff_msがリトライ1回目で200msを返す() {
        assert_eq!(compute_backoff_ms(1), 200);
    }

    #[test]
    fn test_compute_backoff_msがリトライ4回目で1600msを返す() {
        assert_eq!(compute_backoff_ms(4), 1600);
    }

    #[test]
    fn test_compute_backoff_msが上限5000msを超えない() {
        assert_eq!(compute_backoff_ms(10), 5_000);
    }
}
