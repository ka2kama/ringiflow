//! # S3DocumentDeleter
//!
//! テナントのドキュメントを S3 から削除する。
//!
//! ## 削除方式
//!
//! S3 のオブジェクトは `{tenant_id}/` プレフィックスで格納される。
//! ListObjectsV2 で該当キーを列挙し、DeleteObjects で 1000 件ずつ削除する。

use async_trait::async_trait;
use aws_sdk_s3::{
    Client,
    types::{Delete, ObjectIdentifier},
};
use ringiflow_domain::tenant::TenantId;

use super::{DeletionResult, TenantDeleter};
use crate::error::InfraError;

/// S3 ドキュメント Deleter
pub struct S3DocumentDeleter {
    client:      Client,
    bucket_name: String,
}

impl S3DocumentDeleter {
    pub fn new(client: Client, bucket_name: String) -> Self {
        Self {
            client,
            bucket_name,
        }
    }
}

#[async_trait]
impl TenantDeleter for S3DocumentDeleter {
    fn name(&self) -> &'static str {
        "s3:documents"
    }

    async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
        let prefix = format!("{}/", tenant_id.as_uuid());
        let mut deleted_count: u64 = 0;
        let mut continuation_token = None;

        loop {
            // ListObjectsV2 でテナントのオブジェクトキーを取得
            let mut list = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket_name)
                .prefix(&prefix);

            if let Some(token) = continuation_token {
                list = list.continuation_token(token);
            }

            let output = list
                .send()
                .await
                .map_err(|e| InfraError::S3(format!("オブジェクト一覧の取得に失敗: {e}")))?;

            let contents = output.contents();
            if contents.is_empty() {
                break;
            }

            // DeleteObjects で 1000 件ずつ削除（S3 API の上限）
            for chunk in contents.chunks(1000) {
                let objects: Vec<ObjectIdentifier> = chunk
                    .iter()
                    .filter_map(|obj| {
                        obj.key().map(|key| {
                            ObjectIdentifier::builder()
                                .key(key)
                                .build()
                                .expect("key は必須フィールド")
                        })
                    })
                    .collect();

                let count = objects.len() as u64;

                let delete = Delete::builder()
                    .set_objects(Some(objects))
                    .quiet(true)
                    .build()
                    .map_err(|e| InfraError::S3(format!("Delete リクエストの構築に失敗: {e}")))?;

                let result = self
                    .client
                    .delete_objects()
                    .bucket(&self.bucket_name)
                    .delete(delete)
                    .send()
                    .await
                    .map_err(|e| InfraError::S3(format!("オブジェクトの削除に失敗: {e}")))?;

                // エラーが含まれる場合はログに記録
                let errors = result.errors();
                if !errors.is_empty() {
                    tracing::error!(
                        error_count = errors.len(),
                        "S3 DeleteObjects: 一部のオブジェクト削除に失敗"
                    );
                    return Err(InfraError::S3(format!(
                        "{}件のオブジェクト削除に失敗",
                        errors.len()
                    )));
                }

                deleted_count += count;
            }

            // ページネーション
            if output.is_truncated() != Some(true) {
                break;
            }
            continuation_token = output.next_continuation_token().map(String::from);
        }

        Ok(DeletionResult { deleted_count })
    }

    async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError> {
        let prefix = format!("{}/", tenant_id.as_uuid());
        let mut total: u64 = 0;
        let mut continuation_token = None;

        loop {
            let mut list = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket_name)
                .prefix(&prefix);

            if let Some(token) = continuation_token {
                list = list.continuation_token(token);
            }

            let output = list
                .send()
                .await
                .map_err(|e| InfraError::S3(format!("オブジェクト件数の取得に失敗: {e}")))?;

            total += output.key_count().unwrap_or(0) as u64;

            if output.is_truncated() != Some(true) {
                break;
            }
            continuation_token = output.next_continuation_token().map(String::from);
        }

        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nameがs3_documentsを返す() {
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .build();
        let client = Client::from_conf(config);
        let sut = S3DocumentDeleter::new(client, "test-bucket".to_string());

        assert_eq!(sut.name(), "s3:documents");
    }

    #[test]
    fn test_send_syncを満たす() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<S3DocumentDeleter>();
    }
}
