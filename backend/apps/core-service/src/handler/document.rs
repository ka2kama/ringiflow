//! # ドキュメントハンドラ
//!
//! Core API のドキュメント管理内部 API を提供する。
//!
//! ## エンドポイント
//!
//! - `POST /internal/documents/upload-url` - Upload URL 発行
//! - `POST /internal/documents/{document_id}/confirm` - アップロード完了確認

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::CoreError,
    usecase::document::{DocumentUseCaseImpl, RequestUploadUrlInput},
};

/// ドキュメント API の共有状態
pub struct DocumentState {
    pub usecase: DocumentUseCaseImpl,
}

// --- リクエスト/レスポンス型 ---

/// Upload URL 発行リクエスト
#[derive(Debug, Deserialize)]
pub struct RequestUploadUrlRequest {
    pub tenant_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub content_length: i64,
    pub folder_id: Option<Uuid>,
    pub workflow_instance_id: Option<Uuid>,
    pub uploaded_by: Uuid,
}

/// テナント ID クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct ConfirmUploadQuery {
    pub tenant_id: Uuid,
}

/// Upload URL レスポンス DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadUrlDto {
    pub document_id: Uuid,
    pub upload_url:  String,
    pub expires_in:  u64,
}

/// ドキュメント詳細レスポンス DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentDto {
    pub id:           Uuid,
    pub filename:     String,
    pub content_type: String,
    pub size:         i64,
    pub status:       String,
    pub created_at:   String,
}

// --- ハンドラ ---

/// POST /internal/documents/upload-url
///
/// Presigned PUT URL を発行する。
///
/// ## レスポンス
///
/// - `200 OK`: upload URL と document_id
/// - `400 Bad Request`: バリデーションエラー（Content-Type、サイズ、コンテキスト）
#[tracing::instrument(skip_all)]
pub async fn request_upload_url(
    State(state): State<Arc<DocumentState>>,
    Json(req): Json<RequestUploadUrlRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let input = RequestUploadUrlInput {
        tenant_id: ringiflow_domain::tenant::TenantId::from_uuid(req.tenant_id),
        filename: req.filename,
        content_type: req.content_type,
        content_length: req.content_length,
        folder_id: req.folder_id,
        workflow_instance_id: req.workflow_instance_id,
        uploaded_by: req.uploaded_by,
    };

    let output = state.usecase.request_upload_url(input).await?;

    let dto = UploadUrlDto {
        document_id: *output.document_id.as_uuid(),
        upload_url:  output.upload_url,
        expires_in:  output.expires_in,
    };

    let response = ApiResponse::new(dto);
    Ok((StatusCode::OK, Json(response)))
}

/// POST /internal/documents/{document_id}/confirm
///
/// アップロード完了を確認し、ドキュメントを active にする。
///
/// ## レスポンス
///
/// - `200 OK`: active になったドキュメント
/// - `400 Bad Request`: ステータスが uploading ではない
/// - `404 Not Found`: ドキュメントが見つからない
/// - `500 Internal Server Error`: S3 にファイルが存在しない
#[tracing::instrument(skip_all, fields(%document_id))]
pub async fn confirm_upload(
    State(state): State<Arc<DocumentState>>,
    Path(document_id): Path<Uuid>,
    Query(query): Query<ConfirmUploadQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let document_id = ringiflow_domain::document::DocumentId::from_uuid(document_id);
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(query.tenant_id);

    let document = state
        .usecase
        .confirm_upload(&document_id, &tenant_id)
        .await?;

    let dto = DocumentDto {
        id:           *document.id().as_uuid(),
        filename:     document.filename().to_string(),
        content_type: document.content_type().to_string(),
        size:         document.size(),
        status:       document.status().to_string(),
        created_at:   document.created_at().to_rfc3339(),
    };

    let response = ApiResponse::new(dto);
    Ok((StatusCode::OK, Json(response)))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc, time::Duration};

    use async_trait::async_trait;
    use axum::{Router, body::Body, http::Request, routing::post};
    use chrono::{DateTime, Utc};
    use ringiflow_domain::{
        clock::Clock,
        document::{Document, DocumentId, DocumentStatus, UploadContext},
        folder::FolderId,
        tenant::TenantId,
        user::UserId,
        workflow::WorkflowInstanceId,
    };
    use ringiflow_infra::{InfraError, repository::DocumentRepository, s3::S3Client};
    use ringiflow_shared::ApiResponse;
    use tower::ServiceExt;

    use super::*;

    // --- スタブ ---

    struct StubDocumentRepository {
        documents: Vec<Document>,
    }

    impl StubDocumentRepository {
        fn empty() -> Self {
            Self {
                documents: Vec::new(),
            }
        }

        fn with_documents(documents: Vec<Document>) -> Self {
            Self { documents }
        }
    }

    #[async_trait]
    impl DocumentRepository for StubDocumentRepository {
        async fn find_by_id(
            &self,
            id: &DocumentId,
            _tenant_id: &TenantId,
        ) -> Result<Option<Document>, InfraError> {
            Ok(self.documents.iter().find(|d| d.id() == id).cloned())
        }

        async fn insert(&self, _document: &Document) -> Result<(), InfraError> {
            Ok(())
        }

        async fn update_status(
            &self,
            _id: &DocumentId,
            _status: DocumentStatus,
            _tenant_id: &TenantId,
            _now: DateTime<Utc>,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn count_and_total_size_by_folder(
            &self,
            _folder_id: &FolderId,
            _tenant_id: &TenantId,
        ) -> Result<(usize, i64), InfraError> {
            let count = self.documents.len();
            let total_size: i64 = self.documents.iter().map(|d| d.size()).sum();
            Ok((count, total_size))
        }

        async fn count_and_total_size_by_workflow(
            &self,
            _workflow_instance_id: &WorkflowInstanceId,
            _tenant_id: &TenantId,
        ) -> Result<(usize, i64), InfraError> {
            let count = self.documents.len();
            let total_size: i64 = self.documents.iter().map(|d| d.size()).sum();
            Ok((count, total_size))
        }
    }

    struct StubS3Client {
        presigned_url: String,
        existing_keys: HashSet<String>,
    }

    impl StubS3Client {
        fn new(presigned_url: &str) -> Self {
            Self {
                presigned_url: presigned_url.to_string(),
                existing_keys: HashSet::new(),
            }
        }

        fn with_existing_keys(mut self, keys: Vec<String>) -> Self {
            self.existing_keys = keys.into_iter().collect();
            self
        }
    }

    #[async_trait]
    impl S3Client for StubS3Client {
        async fn generate_presigned_put_url(
            &self,
            _s3_key: &str,
            _content_type: &str,
            _content_length: i64,
            _expires_in: Duration,
        ) -> Result<String, InfraError> {
            Ok(self.presigned_url.clone())
        }

        async fn generate_presigned_get_url(
            &self,
            _s3_key: &str,
            _expires_in: Duration,
        ) -> Result<String, InfraError> {
            Ok(self.presigned_url.clone())
        }

        async fn head_object(&self, s3_key: &str) -> Result<bool, InfraError> {
            Ok(self.existing_keys.contains(s3_key))
        }
    }

    struct StubClock;

    impl Clock for StubClock {
        fn now(&self) -> DateTime<Utc> {
            DateTime::from_timestamp(1_700_000_000, 0).unwrap()
        }
    }

    // --- ヘルパー ---

    fn create_test_app(repo: StubDocumentRepository, s3_client: StubS3Client) -> Router {
        let repo_arc = Arc::new(repo) as Arc<dyn DocumentRepository>;
        let s3_arc = Arc::new(s3_client) as Arc<dyn S3Client>;
        let usecase =
            DocumentUseCaseImpl::new(repo_arc, s3_arc, Arc::new(StubClock) as Arc<dyn Clock>);
        let state = Arc::new(DocumentState { usecase });

        Router::new()
            .route("/internal/documents/upload-url", post(request_upload_url))
            .route(
                "/internal/documents/{document_id}/confirm",
                post(confirm_upload),
            )
            .with_state(state)
    }

    fn fixed_now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    async fn response_body<T: serde::de::DeserializeOwned>(
        response: axum::http::Response<Body>,
    ) -> T {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn make_uploading_document(tenant_id: &TenantId) -> Document {
        Document::new_uploading(
            DocumentId::new(),
            tenant_id.clone(),
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "some-key".to_string(),
            UploadContext::Folder(FolderId::new()),
            Some(UserId::new()),
            fixed_now(),
        )
    }

    // --- テストケース ---

    // upload-url 正常系

    #[tokio::test]
    async fn test_post_upload_url正常系_200でupload_urlとdocument_idが返る() {
        // Given
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubS3Client::new("https://s3.example.com/presigned"),
        );
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/documents/upload-url")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "filename": "test.pdf",
                    "content_type": "application/pdf",
                    "content_length": 1024,
                    "folder_id": FolderId::new().as_uuid(),
                    "uploaded_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        let body: ApiResponse<UploadUrlDto> = response_body(response).await;
        assert_eq!(body.data.upload_url, "https://s3.example.com/presigned");
        assert_eq!(body.data.expires_in, 300);
    }

    // upload-url 準正常系

    #[tokio::test]
    async fn test_post_upload_url非対応content_typeで400が返る() {
        // Given
        let sut = create_test_app(StubDocumentRepository::empty(), StubS3Client::new("url"));
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/documents/upload-url")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "filename": "test.zip",
                    "content_type": "application/zip",
                    "content_length": 1024,
                    "folder_id": FolderId::new().as_uuid(),
                    "uploaded_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_post_upload_urlサイズ超過で400が返る() {
        // Given
        let sut = create_test_app(StubDocumentRepository::empty(), StubS3Client::new("url"));
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/documents/upload-url")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "filename": "big.pdf",
                    "content_type": "application/pdf",
                    "content_length": 21 * 1024 * 1024,
                    "folder_id": FolderId::new().as_uuid(),
                    "uploaded_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_post_upload_url_folder_idもworkflow_instance_idも未指定で400が返る() {
        // Given
        let sut = create_test_app(StubDocumentRepository::empty(), StubS3Client::new("url"));
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/documents/upload-url")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "filename": "test.pdf",
                    "content_type": "application/pdf",
                    "content_length": 1024,
                    "uploaded_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_post_upload_url両方指定で400が返る() {
        // Given
        let sut = create_test_app(StubDocumentRepository::empty(), StubS3Client::new("url"));
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/documents/upload-url")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "filename": "test.pdf",
                    "content_type": "application/pdf",
                    "content_length": 1024,
                    "folder_id": FolderId::new().as_uuid(),
                    "workflow_instance_id": WorkflowInstanceId::new().as_uuid(),
                    "uploaded_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // confirm 正常系

    #[tokio::test]
    async fn test_post_confirm正常系_200でactiveドキュメントが返る() {
        // Given
        let tenant_id = TenantId::new();
        let doc = make_uploading_document(&tenant_id);
        let doc_id = *doc.id().as_uuid();
        let s3_key = doc.s3_key().to_string();

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubS3Client::new("url").with_existing_keys(vec![s3_key]),
        );

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!(
                "/internal/documents/{}/confirm?tenant_id={}",
                doc_id,
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        let body: ApiResponse<DocumentDto> = response_body(response).await;
        assert_eq!(body.data.status, "active");
        assert_eq!(body.data.filename, "test.pdf");
    }

    // confirm 準正常系

    #[tokio::test]
    async fn test_post_confirm存在しないidで404が返る() {
        // Given
        let sut = create_test_app(StubDocumentRepository::empty(), StubS3Client::new("url"));
        let tenant_id = TenantId::new();
        let nonexistent_id = Uuid::new_v4();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!(
                "/internal/documents/{}/confirm?tenant_id={}",
                nonexistent_id,
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_post_confirm非uploadingステータスで400が返る() {
        // Given
        let tenant_id = TenantId::new();
        let doc = Document::from_db(
            DocumentId::new(),
            tenant_id.clone(),
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "some-key".to_string(),
            UploadContext::Folder(FolderId::new()),
            DocumentStatus::Active,
            None,
            fixed_now(),
            fixed_now(),
            None,
        );
        let doc_id = *doc.id().as_uuid();

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!(
                "/internal/documents/{}/confirm?tenant_id={}",
                doc_id,
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_post_confirm_s3にファイルなしで500が返る() {
        // Given
        let tenant_id = TenantId::new();
        let doc = make_uploading_document(&tenant_id);
        let doc_id = *doc.id().as_uuid();
        // S3 に key を登録しない → head_object が false を返す

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!(
                "/internal/documents/{}/confirm?tenant_id={}",
                doc_id,
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
