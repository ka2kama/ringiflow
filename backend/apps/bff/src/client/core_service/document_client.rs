//! ドキュメント関連の Core Service クライアント

use async_trait::async_trait;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
    client_impl::CoreServiceClientImpl,
    error::CoreServiceError,
    response::handle_response,
    types::{DocumentDetailCoreDto, RequestUploadUrlCoreRequest, UploadUrlCoreDto},
};
use crate::middleware::request_id::inject_request_id;

/// ドキュメント関連の Core Service クライアントトレイト
#[async_trait]
pub trait CoreServiceDocumentClient: Send + Sync {
    /// Upload URL を発行する
    ///
    /// Core Service の `POST /internal/documents/upload-url` を呼び出す。
    async fn request_upload_url(
        &self,
        req: &RequestUploadUrlCoreRequest,
    ) -> Result<ApiResponse<UploadUrlCoreDto>, CoreServiceError>;

    /// アップロード完了を確認する
    ///
    /// Core Service の `POST /internal/documents/{document_id}/confirm` を呼び出す。
    async fn confirm_upload(
        &self,
        document_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiResponse<DocumentDetailCoreDto>, CoreServiceError>;
}

#[async_trait]
impl CoreServiceDocumentClient for CoreServiceClientImpl {
    #[tracing::instrument(skip_all, level = "debug")]
    async fn request_upload_url(
        &self,
        req: &RequestUploadUrlCoreRequest,
    ) -> Result<ApiResponse<UploadUrlCoreDto>, CoreServiceError> {
        let url = format!("{}/internal/documents/upload-url", self.base_url);

        let response = inject_request_id(self.client.post(&url))
            .json(req)
            .send()
            .await?;
        handle_response(response, None).await
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%document_id, %tenant_id))]
    async fn confirm_upload(
        &self,
        document_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiResponse<DocumentDetailCoreDto>, CoreServiceError> {
        let url = format!(
            "{}/internal/documents/{}/confirm?tenant_id={}",
            self.base_url, document_id, tenant_id
        );

        let response = inject_request_id(self.client.post(&url)).send().await?;
        handle_response(response, Some(CoreServiceError::DocumentNotFound)).await
    }
}
