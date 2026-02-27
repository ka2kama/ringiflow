//! ドキュメント関連の Core Service クライアント

use async_trait::async_trait;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
    client_impl::CoreServiceClientImpl,
    error::CoreServiceError,
    response::handle_response,
    types::{
        DocumentDetailCoreDto,
        DownloadUrlCoreDto,
        RequestUploadUrlCoreRequest,
        UploadUrlCoreDto,
    },
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

    /// ダウンロード URL を発行する
    ///
    /// Core Service の `POST /internal/documents/{document_id}/download-url` を呼び出す。
    async fn generate_download_url(
        &self,
        document_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiResponse<DownloadUrlCoreDto>, CoreServiceError>;

    /// ドキュメントを削除する（ソフトデリート）
    ///
    /// Core Service の `DELETE /internal/documents/{document_id}` を呼び出す。
    async fn delete_document(
        &self,
        document_id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        is_tenant_admin: bool,
    ) -> Result<(), CoreServiceError>;

    /// フォルダ内のドキュメント一覧を取得する
    ///
    /// Core Service の `GET /internal/documents?tenant_id=...&folder_id=...` を呼び出す。
    async fn list_documents(
        &self,
        tenant_id: Uuid,
        folder_id: Uuid,
    ) -> Result<ApiResponse<Vec<DocumentDetailCoreDto>>, CoreServiceError>;

    /// ワークフロー添付ファイル一覧を取得する
    ///
    /// Core Service の `GET /internal/workflows/{id}/attachments?tenant_id=...` を呼び出す。
    async fn list_workflow_attachments(
        &self,
        workflow_instance_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiResponse<Vec<DocumentDetailCoreDto>>, CoreServiceError>;
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

    #[tracing::instrument(skip_all, level = "debug", fields(%document_id, %tenant_id))]
    async fn generate_download_url(
        &self,
        document_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiResponse<DownloadUrlCoreDto>, CoreServiceError> {
        let url = format!(
            "{}/internal/documents/{}/download-url?tenant_id={}",
            self.base_url, document_id, tenant_id
        );

        let response = inject_request_id(self.client.post(&url)).send().await?;
        handle_response(response, Some(CoreServiceError::DocumentNotFound)).await
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%document_id, %tenant_id, %user_id))]
    async fn delete_document(
        &self,
        document_id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        is_tenant_admin: bool,
    ) -> Result<(), CoreServiceError> {
        let url = format!(
            "{}/internal/documents/{}?tenant_id={}&user_id={}&is_tenant_admin={}",
            self.base_url, document_id, tenant_id, user_id, is_tenant_admin
        );

        let response = inject_request_id(self.client.delete(&url)).send().await?;
        let status = response.status();

        if status.is_success() {
            return Ok(());
        }

        let body = response.text().await.unwrap_or_default();

        let error = match status {
            reqwest::StatusCode::NOT_FOUND => CoreServiceError::DocumentNotFound,
            reqwest::StatusCode::BAD_REQUEST => CoreServiceError::ValidationError(body),
            reqwest::StatusCode::FORBIDDEN => CoreServiceError::Forbidden(body),
            _ => CoreServiceError::Unexpected(format!("予期しないステータス {}: {}", status, body)),
        };

        Err(error)
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id, %folder_id))]
    async fn list_documents(
        &self,
        tenant_id: Uuid,
        folder_id: Uuid,
    ) -> Result<ApiResponse<Vec<DocumentDetailCoreDto>>, CoreServiceError> {
        let url = format!(
            "{}/internal/documents?tenant_id={}&folder_id={}",
            self.base_url, tenant_id, folder_id
        );

        let response = inject_request_id(self.client.get(&url)).send().await?;
        handle_response(response, None).await
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%workflow_instance_id, %tenant_id))]
    async fn list_workflow_attachments(
        &self,
        workflow_instance_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<ApiResponse<Vec<DocumentDetailCoreDto>>, CoreServiceError> {
        let url = format!(
            "{}/internal/workflows/{}/attachments?tenant_id={}",
            self.base_url, workflow_instance_id, tenant_id
        );

        let response = inject_request_id(self.client.get(&url)).send().await?;
        handle_response(response, None).await
    }
}
