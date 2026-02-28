//! # ドキュメントハンドラ
//!
//! Core API のドキュメント管理内部 API を提供する。
//!
//! ## エンドポイント
//!
//! - `POST /internal/documents/upload-url` - Upload URL 発行
//! - `POST /internal/documents/{document_id}/confirm` - アップロード完了確認
//! - `POST /internal/documents/{document_id}/download-url` - ダウンロード URL 発行
//! - `DELETE /internal/documents/{document_id}` - ソフトデリート
//! - `GET /internal/documents` - ドキュメント一覧取得
//! - `GET /internal/workflows/{workflow_instance_id}/attachments` - ワークフロー添付ファイル一覧

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
    usecase::document::{DocumentUseCaseImpl, RequestUploadUrlInput, SoftDeleteInput},
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

/// ダウンロード URL レスポンス DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadUrlDto {
    pub download_url: String,
    pub expires_in:   u64,
}

/// ドキュメント削除クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct DeleteDocumentQuery {
    pub tenant_id:       Uuid,
    pub user_id:         Uuid,
    pub is_tenant_admin: bool,
}

/// テナント ID クエリパラメータ（汎用）
#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
}

/// ドキュメント一覧クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    pub tenant_id: Uuid,
    pub folder_id: Uuid,
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
/// - `400 Bad Request`: ステータスが uploading ではない / S3 にファイルが存在しない
/// - `404 Not Found`: ドキュメントが見つからない
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

/// POST /internal/documents/{document_id}/download-url
///
/// ダウンロード用の Presigned GET URL を発行する。
///
/// ## レスポンス
///
/// - `200 OK`: download_url と expires_in
/// - `400 Bad Request`: ステータスが active ではない
/// - `404 Not Found`: ドキュメントが見つからない
#[tracing::instrument(skip_all, fields(%document_id))]
pub async fn generate_download_url(
    State(state): State<Arc<DocumentState>>,
    Path(document_id): Path<Uuid>,
    Query(query): Query<TenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let document_id = ringiflow_domain::document::DocumentId::from_uuid(document_id);
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(query.tenant_id);

    let output = state
        .usecase
        .generate_download_url(&document_id, &tenant_id)
        .await?;

    let dto = DownloadUrlDto {
        download_url: output.download_url,
        expires_in:   output.expires_in,
    };

    let response = ApiResponse::new(dto);
    Ok((StatusCode::OK, Json(response)))
}

/// DELETE /internal/documents/{document_id}
///
/// ドキュメントをソフトデリートする。
///
/// ## レスポンス
///
/// - `204 No Content`: 削除成功
/// - `400 Bad Request`: ステータスが active ではない / ワークフロー添付で下書き以外
/// - `403 Forbidden`: 権限不足
/// - `404 Not Found`: ドキュメントが見つからない
#[tracing::instrument(skip_all, fields(%document_id))]
pub async fn delete_document(
    State(state): State<Arc<DocumentState>>,
    Path(document_id): Path<Uuid>,
    Query(query): Query<DeleteDocumentQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let input = SoftDeleteInput {
        document_id:     ringiflow_domain::document::DocumentId::from_uuid(document_id),
        tenant_id:       ringiflow_domain::tenant::TenantId::from_uuid(query.tenant_id),
        user_id:         ringiflow_domain::user::UserId::from_uuid(query.user_id),
        is_tenant_admin: query.is_tenant_admin,
    };

    state.usecase.soft_delete_document(input).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /internal/documents
///
/// フォルダ内のドキュメント一覧を取得する。
///
/// ## レスポンス
///
/// - `200 OK`: ドキュメント配列
#[tracing::instrument(skip_all)]
pub async fn list_documents(
    State(state): State<Arc<DocumentState>>,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let folder_id = ringiflow_domain::folder::FolderId::from_uuid(query.folder_id);
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(query.tenant_id);

    let documents = state.usecase.list_documents(&folder_id, &tenant_id).await?;

    let dtos: Vec<DocumentDto> = documents
        .iter()
        .map(|doc| DocumentDto {
            id:           *doc.id().as_uuid(),
            filename:     doc.filename().to_string(),
            content_type: doc.content_type().to_string(),
            size:         doc.size(),
            status:       doc.status().to_string(),
            created_at:   doc.created_at().to_rfc3339(),
        })
        .collect();

    let response = ApiResponse::new(dtos);
    Ok((StatusCode::OK, Json(response)))
}

/// GET /internal/workflows/{workflow_instance_id}/attachments
///
/// ワークフロー添付ファイル一覧を取得する。
///
/// ## レスポンス
///
/// - `200 OK`: ドキュメント配列
#[tracing::instrument(skip_all, fields(%workflow_instance_id))]
pub async fn list_workflow_attachments(
    State(state): State<Arc<DocumentState>>,
    Path(workflow_instance_id): Path<Uuid>,
    Query(query): Query<TenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let workflow_instance_id =
        ringiflow_domain::workflow::WorkflowInstanceId::from_uuid(workflow_instance_id);
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(query.tenant_id);

    let documents = state
        .usecase
        .list_workflow_attachments(&workflow_instance_id, &tenant_id)
        .await?;

    let dtos: Vec<DocumentDto> = documents
        .iter()
        .map(|doc| DocumentDto {
            id:           *doc.id().as_uuid(),
            filename:     doc.filename().to_string(),
            content_type: doc.content_type().to_string(),
            size:         doc.size(),
            status:       doc.status().to_string(),
            created_at:   doc.created_at().to_rfc3339(),
        })
        .collect();

    let response = ApiResponse::new(dtos);
    Ok((StatusCode::OK, Json(response)))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc, time::Duration};

    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::Request,
        routing::{delete, get, post},
    };
    use chrono::{DateTime, Utc};
    use ringiflow_domain::{
        clock::Clock,
        document::{Document, DocumentId, DocumentStatus, UploadContext},
        folder::FolderId,
        tenant::TenantId,
        user::UserId,
        value_objects::{DisplayNumber, Version},
        workflow::{
            WorkflowDefinitionId,
            WorkflowInstance,
            WorkflowInstanceId,
            WorkflowInstanceRecord,
            WorkflowInstanceStatus,
        },
    };
    use ringiflow_infra::{
        InfraError,
        TxContext,
        repository::{DocumentRepository, WorkflowInstanceRepository},
        s3::S3Client,
    };
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

        async fn soft_delete(
            &self,
            _id: &DocumentId,
            _tenant_id: &TenantId,
            _now: DateTime<Utc>,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn list_by_folder(
            &self,
            _folder_id: &FolderId,
            _tenant_id: &TenantId,
        ) -> Result<Vec<Document>, InfraError> {
            Ok(self.documents.clone())
        }

        async fn list_by_workflow(
            &self,
            _workflow_instance_id: &WorkflowInstanceId,
            _tenant_id: &TenantId,
        ) -> Result<Vec<Document>, InfraError> {
            Ok(self.documents.clone())
        }
    }

    struct StubWorkflowInstanceRepository {
        instance: Option<WorkflowInstance>,
    }

    impl StubWorkflowInstanceRepository {
        fn empty() -> Self {
            Self { instance: None }
        }

        fn with_instance(instance: WorkflowInstance) -> Self {
            Self {
                instance: Some(instance),
            }
        }
    }

    #[async_trait]
    impl WorkflowInstanceRepository for StubWorkflowInstanceRepository {
        async fn insert(
            &self,
            _tx: &mut TxContext,
            _instance: &WorkflowInstance,
        ) -> Result<(), InfraError> {
            unimplemented!()
        }

        async fn update_with_version_check(
            &self,
            _tx: &mut TxContext,
            _instance: &WorkflowInstance,
            _expected_version: Version,
            _tenant_id: &TenantId,
        ) -> Result<(), InfraError> {
            unimplemented!()
        }

        async fn find_by_id(
            &self,
            _id: &WorkflowInstanceId,
            _tenant_id: &TenantId,
        ) -> Result<Option<WorkflowInstance>, InfraError> {
            Ok(self.instance.clone())
        }

        async fn find_by_tenant(
            &self,
            _tenant_id: &TenantId,
        ) -> Result<Vec<WorkflowInstance>, InfraError> {
            unimplemented!()
        }

        async fn find_by_initiated_by(
            &self,
            _tenant_id: &TenantId,
            _user_id: &UserId,
        ) -> Result<Vec<WorkflowInstance>, InfraError> {
            unimplemented!()
        }

        async fn find_by_ids(
            &self,
            _ids: &[WorkflowInstanceId],
            _tenant_id: &TenantId,
        ) -> Result<Vec<WorkflowInstance>, InfraError> {
            unimplemented!()
        }

        async fn find_by_display_number(
            &self,
            _display_number: DisplayNumber,
            _tenant_id: &TenantId,
        ) -> Result<Option<WorkflowInstance>, InfraError> {
            unimplemented!()
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

    fn create_test_app(
        repo: StubDocumentRepository,
        workflow_repo: StubWorkflowInstanceRepository,
        s3_client: StubS3Client,
    ) -> Router {
        let repo_arc = Arc::new(repo) as Arc<dyn DocumentRepository>;
        let workflow_repo_arc = Arc::new(workflow_repo) as Arc<dyn WorkflowInstanceRepository>;
        let s3_arc = Arc::new(s3_client) as Arc<dyn S3Client>;
        let usecase = DocumentUseCaseImpl::new(
            repo_arc,
            workflow_repo_arc,
            s3_arc,
            Arc::new(StubClock) as Arc<dyn Clock>,
        );
        let state = Arc::new(DocumentState { usecase });

        Router::new()
            .route("/internal/documents", get(list_documents))
            .route("/internal/documents/upload-url", post(request_upload_url))
            .route(
                "/internal/documents/{document_id}/confirm",
                post(confirm_upload),
            )
            .route(
                "/internal/documents/{document_id}/download-url",
                post(generate_download_url),
            )
            .route("/internal/documents/{document_id}", delete(delete_document))
            .route(
                "/internal/workflows/{workflow_instance_id}/attachments",
                get(list_workflow_attachments),
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

    fn make_active_document(tenant_id: &TenantId, uploaded_by: &UserId) -> Document {
        Document::from_db(
            DocumentId::new(),
            tenant_id.clone(),
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "some-key".to_string(),
            UploadContext::Folder(FolderId::new()),
            DocumentStatus::Active,
            Some(uploaded_by.clone()),
            fixed_now(),
            fixed_now(),
            None,
        )
    }

    fn make_active_workflow_document(
        tenant_id: &TenantId,
        uploaded_by: &UserId,
        workflow_instance_id: &WorkflowInstanceId,
    ) -> Document {
        Document::from_db(
            DocumentId::new(),
            tenant_id.clone(),
            "attachment.pdf".to_string(),
            "application/pdf".to_string(),
            2048,
            "wf-key".to_string(),
            UploadContext::Workflow(workflow_instance_id.clone()),
            DocumentStatus::Active,
            Some(uploaded_by.clone()),
            fixed_now(),
            fixed_now(),
            None,
        )
    }

    fn make_draft_workflow(
        tenant_id: &TenantId,
        workflow_instance_id: &WorkflowInstanceId,
    ) -> WorkflowInstance {
        WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: workflow_instance_id.clone(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(1).unwrap(),
            title: "テストワークフロー".to_string(),
            form_data: serde_json::json!({}),
            status: WorkflowInstanceStatus::Draft,
            version: Version::initial(),
            current_step_id: None,
            initiated_by: UserId::new(),
            submitted_at: None,
            completed_at: None,
            created_at: fixed_now(),
            updated_at: fixed_now(),
        })
        .unwrap()
    }

    fn make_pending_workflow(
        tenant_id: &TenantId,
        workflow_instance_id: &WorkflowInstanceId,
    ) -> WorkflowInstance {
        WorkflowInstance::from_db(WorkflowInstanceRecord {
            id: workflow_instance_id.clone(),
            tenant_id: tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(2).unwrap(),
            title: "申請済みワークフロー".to_string(),
            form_data: serde_json::json!({}),
            status: WorkflowInstanceStatus::Pending,
            version: Version::initial(),
            current_step_id: None,
            initiated_by: UserId::new(),
            submitted_at: Some(fixed_now()),
            completed_at: None,
            created_at: fixed_now(),
            updated_at: fixed_now(),
        })
        .unwrap()
    }

    // --- テストケース ---

    // upload-url 正常系

    #[tokio::test]
    async fn test_post_upload_url正常系_200でupload_urlとdocument_idが返る() {
        // Given
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubWorkflowInstanceRepository::empty(),
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
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );
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
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );
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
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
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
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
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
            StubWorkflowInstanceRepository::empty(),
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
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );
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
            StubWorkflowInstanceRepository::empty(),
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
    async fn test_post_confirm_s3にファイルなしで400が返る() {
        // Given
        let tenant_id = TenantId::new();
        let doc = make_uploading_document(&tenant_id);
        let doc_id = *doc.id().as_uuid();
        // S3 に key を登録しない → head_object が false を返す

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::empty(),
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

    // --- download-url テスト ---

    #[tokio::test]
    async fn test_post_download_url正常系_200でdownload_urlとexpires_inが返る() {
        // Given
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let doc = make_active_document(&tenant_id, &user_id);
        let doc_id = *doc.id().as_uuid();

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("https://s3.example.com/download"),
        );

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!(
                "/internal/documents/{}/download-url?tenant_id={}",
                doc_id,
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        let body: ApiResponse<DownloadUrlDto> = response_body(response).await;
        assert_eq!(body.data.download_url, "https://s3.example.com/download");
        assert_eq!(body.data.expires_in, 900);
    }

    #[tokio::test]
    async fn test_post_download_url存在しないidで404が返る() {
        // Given
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!(
                "/internal/documents/{}/download-url?tenant_id={}",
                Uuid::new_v4(),
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
    async fn test_post_download_url非activeドキュメントで400が返る() {
        // Given
        let tenant_id = TenantId::new();
        let doc = make_uploading_document(&tenant_id);
        let doc_id = *doc.id().as_uuid();

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri(format!(
                "/internal/documents/{}/download-url?tenant_id={}",
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

    // --- delete テスト ---

    #[tokio::test]
    async fn test_delete正常系_アップロード者が削除して204が返る() {
        // Given
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let doc = make_active_document(&tenant_id, &user_id);
        let doc_id = *doc.id().as_uuid();

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/documents/{}?tenant_id={}&user_id={}&is_tenant_admin=false",
                doc_id,
                tenant_id.as_uuid(),
                user_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete正常系_テナント管理者が削除して204が返る() {
        // Given
        let tenant_id = TenantId::new();
        let uploader_id = UserId::new();
        let admin_id = UserId::new();
        let doc = make_active_document(&tenant_id, &uploader_id);
        let doc_id = *doc.id().as_uuid();

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/documents/{}?tenant_id={}&user_id={}&is_tenant_admin=true",
                doc_id,
                tenant_id.as_uuid(),
                admin_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete権限なしで403が返る() {
        // Given
        let tenant_id = TenantId::new();
        let uploader_id = UserId::new();
        let other_user_id = UserId::new();
        let doc = make_active_document(&tenant_id, &uploader_id);
        let doc_id = *doc.id().as_uuid();

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/documents/{}?tenant_id={}&user_id={}&is_tenant_admin=false",
                doc_id,
                tenant_id.as_uuid(),
                other_user_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_delete存在しないidで404が返る() {
        // Given
        let sut = create_test_app(
            StubDocumentRepository::empty(),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/documents/{}?tenant_id={}&user_id={}&is_tenant_admin=false",
                Uuid::new_v4(),
                tenant_id.as_uuid(),
                Uuid::new_v4()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_deleteワークフロー添付_下書きで204が返る() {
        // Given
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let workflow_instance_id = WorkflowInstanceId::new();
        let doc = make_active_workflow_document(&tenant_id, &user_id, &workflow_instance_id);
        let doc_id = *doc.id().as_uuid();
        let workflow = make_draft_workflow(&tenant_id, &workflow_instance_id);

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::with_instance(workflow),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/documents/{}?tenant_id={}&user_id={}&is_tenant_admin=false",
                doc_id,
                tenant_id.as_uuid(),
                user_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_deleteワークフロー添付_申請済みで400が返る() {
        // Given
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let workflow_instance_id = WorkflowInstanceId::new();
        let doc = make_active_workflow_document(&tenant_id, &user_id, &workflow_instance_id);
        let doc_id = *doc.id().as_uuid();
        let workflow = make_pending_workflow(&tenant_id, &workflow_instance_id);

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::with_instance(workflow),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/documents/{}?tenant_id={}&user_id={}&is_tenant_admin=false",
                doc_id,
                tenant_id.as_uuid(),
                user_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // --- list_documents テスト ---

    #[tokio::test]
    async fn test_get_documents正常系_200でドキュメント配列が返る() {
        // Given
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let doc = make_active_document(&tenant_id, &user_id);
        let folder_id = FolderId::new();

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::GET)
            .uri(format!(
                "/internal/documents?tenant_id={}&folder_id={}",
                tenant_id.as_uuid(),
                folder_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        let body: ApiResponse<Vec<DocumentDto>> = response_body(response).await;
        assert_eq!(body.data.len(), 1);
        assert_eq!(body.data[0].filename, "test.pdf");
    }

    // --- list_workflow_attachments テスト ---

    #[tokio::test]
    async fn test_get_workflow_attachments正常系_200でドキュメント配列が返る() {
        // Given
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let workflow_instance_id = WorkflowInstanceId::new();
        let doc = make_active_workflow_document(&tenant_id, &user_id, &workflow_instance_id);

        let sut = create_test_app(
            StubDocumentRepository::with_documents(vec![doc]),
            StubWorkflowInstanceRepository::empty(),
            StubS3Client::new("url"),
        );

        let request = Request::builder()
            .method(axum::http::Method::GET)
            .uri(format!(
                "/internal/workflows/{}/attachments?tenant_id={}",
                workflow_instance_id.as_uuid(),
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        let body: ApiResponse<Vec<DocumentDto>> = response_body(response).await;
        assert_eq!(body.data.len(), 1);
        assert_eq!(body.data[0].filename, "attachment.pdf");
    }
}
