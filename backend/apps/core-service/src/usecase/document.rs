//! ドキュメント管理ユースケース

use std::{sync::Arc, time::Duration};

use ringiflow_domain::{
    clock::Clock,
    document::{Document, DocumentId, FileValidation, S3KeyGenerator, UploadContext},
    folder::FolderId,
    tenant::TenantId,
    user::UserId,
    workflow::WorkflowInstanceId,
};
use ringiflow_infra::{repository::DocumentRepository, s3::S3Client};
use uuid::Uuid;

use crate::error::CoreError;

/// Presigned URL の有効期限（5 分）
const UPLOAD_URL_EXPIRES_IN: Duration = Duration::from_secs(300);

/// Upload URL 発行リクエスト
pub struct RequestUploadUrlInput {
    pub tenant_id: TenantId,
    pub filename: String,
    pub content_type: String,
    pub content_length: i64,
    pub folder_id: Option<Uuid>,
    pub workflow_instance_id: Option<Uuid>,
    pub uploaded_by: Uuid,
}

/// Upload URL 発行レスポンス
pub struct UploadUrlOutput {
    pub document_id: DocumentId,
    pub upload_url:  String,
    pub expires_in:  u64,
}

/// ドキュメント管理ユースケース
pub struct DocumentUseCaseImpl {
    document_repository: Arc<dyn DocumentRepository>,
    s3_client: Arc<dyn S3Client>,
    clock: Arc<dyn Clock>,
}

impl DocumentUseCaseImpl {
    pub fn new(
        document_repository: Arc<dyn DocumentRepository>,
        s3_client: Arc<dyn S3Client>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            document_repository,
            s3_client,
            clock,
        }
    }

    /// Upload URL を発行する
    ///
    /// 1. UploadContext 構築（folder_id XOR workflow_instance_id）
    /// 2. ファイルバリデーション（Content-Type、サイズ）
    /// 3. 既存ドキュメントの集計バリデーション（数量、合計サイズ）
    /// 4. Document エンティティ作成・挿入
    /// 5. Presigned PUT URL 生成
    pub async fn request_upload_url(
        &self,
        input: RequestUploadUrlInput,
    ) -> Result<UploadUrlOutput, CoreError> {
        // 1. UploadContext 構築
        let upload_context = match (input.folder_id, input.workflow_instance_id) {
            (Some(fid), None) => UploadContext::Folder(FolderId::from_uuid(fid)),
            (None, Some(wid)) => UploadContext::Workflow(WorkflowInstanceId::from_uuid(wid)),
            (Some(_), Some(_)) => {
                return Err(CoreError::BadRequest(
                    "folder_id と workflow_instance_id は同時に指定できません".to_string(),
                ));
            }
            (None, None) => {
                return Err(CoreError::BadRequest(
                    "folder_id または workflow_instance_id のいずれかを指定してください"
                        .to_string(),
                ));
            }
        };

        // 2. ファイルバリデーション
        FileValidation::validate_file(&input.content_type, input.content_length)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 3. 既存ドキュメントの集計バリデーション
        let (existing_count, existing_total_size) = match &upload_context {
            UploadContext::Folder(folder_id) => {
                self.document_repository
                    .count_and_total_size_by_folder(folder_id, &input.tenant_id)
                    .await?
            }
            UploadContext::Workflow(instance_id) => {
                self.document_repository
                    .count_and_total_size_by_workflow(instance_id, &input.tenant_id)
                    .await?
            }
        };
        FileValidation::validate_total(existing_count, existing_total_size, input.content_length)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        // 4. Document エンティティ作成・挿入
        let now = self.clock.now();
        let document_id = DocumentId::new();
        let s3_key = S3KeyGenerator::generate(
            &input.tenant_id,
            &upload_context,
            &document_id,
            &input.filename,
        );

        let document = Document::new_uploading(
            document_id.clone(),
            input.tenant_id,
            input.filename,
            input.content_type.clone(),
            input.content_length,
            s3_key.clone(),
            upload_context,
            Some(UserId::from_uuid(input.uploaded_by)),
            now,
        );

        self.document_repository.insert(&document).await?;

        // 5. Presigned PUT URL 生成
        let upload_url = self
            .s3_client
            .generate_presigned_put_url(
                &s3_key,
                &input.content_type,
                input.content_length,
                UPLOAD_URL_EXPIRES_IN,
            )
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;

        Ok(UploadUrlOutput {
            document_id,
            upload_url,
            expires_in: UPLOAD_URL_EXPIRES_IN.as_secs(),
        })
    }

    /// アップロード完了を確認する
    ///
    /// 1. ドキュメント取得
    /// 2. S3 上のファイル存在確認
    /// 3. ステータスを active に遷移
    pub async fn confirm_upload(
        &self,
        document_id: &DocumentId,
        tenant_id: &TenantId,
    ) -> Result<Document, CoreError> {
        // 1. ドキュメント取得
        let document = self
            .document_repository
            .find_by_id(document_id, tenant_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("ドキュメントが見つかりません".to_string()))?;

        // ステータスチェック
        if document.status() != ringiflow_domain::document::DocumentStatus::Uploading {
            return Err(CoreError::BadRequest(format!(
                "ドキュメントのステータスが uploading ではありません: {}",
                document.status()
            )));
        }

        // 2. S3 上のファイル存在確認
        let exists = self
            .s3_client
            .head_object(document.s3_key())
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        if !exists {
            return Err(CoreError::Internal(
                "S3 上にファイルが見つかりません。アップロードが完了していない可能性があります"
                    .to_string(),
            ));
        }

        // 3. ステータスを active に遷移
        let now = self.clock.now();
        let confirmed = document
            .confirm(now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        self.document_repository
            .update_status(
                confirmed.id(),
                confirmed.status(),
                confirmed.tenant_id(),
                now,
            )
            .await?;

        Ok(confirmed)
    }
}
