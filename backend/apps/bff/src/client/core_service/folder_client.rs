//! フォルダ関連の Core Service クライアント

use async_trait::async_trait;
use ringiflow_shared::ApiResponse;
use uuid::Uuid;

use super::{
    client_impl::CoreServiceClientImpl,
    error::CoreServiceError,
    response::handle_response,
    types::{CreateFolderCoreRequest, FolderItemDto, UpdateFolderCoreRequest},
};
use crate::middleware::request_id::inject_request_id;

/// フォルダ関連の Core Service クライアントトレイト
#[async_trait]
pub trait CoreServiceFolderClient: Send + Sync {
    /// テナント内のフォルダ一覧を取得する
    ///
    /// Core Service の `GET /internal/folders` を呼び出す。
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    async fn list_folders(
        &self,
        tenant_id: Uuid,
    ) -> Result<ApiResponse<Vec<FolderItemDto>>, CoreServiceError>;

    /// フォルダを作成する
    ///
    /// Core Service の `POST /internal/folders` を呼び出す。
    async fn create_folder(
        &self,
        req: &CreateFolderCoreRequest,
    ) -> Result<ApiResponse<FolderItemDto>, CoreServiceError>;

    /// フォルダを更新する（名前変更・移動）
    ///
    /// Core Service の `PUT /internal/folders/{folder_id}` を呼び出す。
    async fn update_folder(
        &self,
        folder_id: Uuid,
        req: &UpdateFolderCoreRequest,
    ) -> Result<ApiResponse<FolderItemDto>, CoreServiceError>;

    /// フォルダを削除する
    ///
    /// Core Service の `DELETE /internal/folders/{folder_id}` を呼び出す。
    async fn delete_folder(&self, folder_id: Uuid, tenant_id: Uuid)
    -> Result<(), CoreServiceError>;
}

#[async_trait]
impl CoreServiceFolderClient for CoreServiceClientImpl {
    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn list_folders(
        &self,
        tenant_id: Uuid,
    ) -> Result<ApiResponse<Vec<FolderItemDto>>, CoreServiceError> {
        let url = format!("{}/internal/folders?tenant_id={}", self.base_url, tenant_id);

        let response = inject_request_id(self.client.get(&url)).send().await?;
        handle_response(response, None).await
    }

    #[tracing::instrument(skip_all, level = "debug")]
    async fn create_folder(
        &self,
        req: &CreateFolderCoreRequest,
    ) -> Result<ApiResponse<FolderItemDto>, CoreServiceError> {
        let url = format!("{}/internal/folders", self.base_url);

        let response = inject_request_id(self.client.post(&url))
            .json(req)
            .send()
            .await?;
        handle_response(response, None).await
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%folder_id))]
    async fn update_folder(
        &self,
        folder_id: Uuid,
        req: &UpdateFolderCoreRequest,
    ) -> Result<ApiResponse<FolderItemDto>, CoreServiceError> {
        let url = format!("{}/internal/folders/{}", self.base_url, folder_id);

        let response = inject_request_id(self.client.put(&url))
            .json(req)
            .send()
            .await?;
        handle_response(response, Some(CoreServiceError::FolderNotFound)).await
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%folder_id, %tenant_id))]
    async fn delete_folder(
        &self,
        folder_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), CoreServiceError> {
        let url = format!(
            "{}/internal/folders/{}?tenant_id={}",
            self.base_url, folder_id, tenant_id
        );

        let response = inject_request_id(self.client.delete(&url)).send().await?;
        let status = response.status();

        if status.is_success() {
            return Ok(());
        }

        let body = response.text().await.unwrap_or_default();

        let error = match status {
            reqwest::StatusCode::NOT_FOUND => CoreServiceError::FolderNotFound,
            reqwest::StatusCode::BAD_REQUEST => CoreServiceError::ValidationError(body),
            reqwest::StatusCode::CONFLICT => CoreServiceError::Conflict(body),
            _ => CoreServiceError::Unexpected(format!("予期しないステータス {}: {}", status, body)),
        };

        Err(error)
    }
}
