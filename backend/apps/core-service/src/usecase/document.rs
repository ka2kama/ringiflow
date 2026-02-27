//! ドキュメント管理ユースケース

use std::{sync::Arc, time::Duration};

use ringiflow_domain::{
    clock::Clock,
    document::{
        Document,
        DocumentId,
        DocumentStatus,
        FileValidation,
        S3KeyGenerator,
        UploadContext,
    },
    folder::FolderId,
    tenant::TenantId,
    user::UserId,
    workflow::{WorkflowInstanceId, WorkflowInstanceStatus},
};
use ringiflow_infra::{
    repository::{DocumentRepository, WorkflowInstanceRepository},
    s3::S3Client,
};
use uuid::Uuid;

use crate::error::CoreError;

/// Presigned URL の有効期限（5 分）
const UPLOAD_URL_EXPIRES_IN: Duration = Duration::from_secs(300);

/// ダウンロード URL の有効期限（15 分）
const DOWNLOAD_URL_EXPIRES_IN: Duration = Duration::from_secs(900);

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

/// ダウンロード URL 発行レスポンス
pub struct DownloadUrlOutput {
    pub download_url: String,
    pub expires_in:   u64,
}

/// ソフトデリートリクエスト
pub struct SoftDeleteInput {
    pub document_id:     DocumentId,
    pub tenant_id:       TenantId,
    pub user_id:         UserId,
    pub is_tenant_admin: bool,
}

/// ドキュメント管理ユースケース
pub struct DocumentUseCaseImpl {
    document_repository: Arc<dyn DocumentRepository>,
    /// 削除時のワークフロー状態チェックに使用
    workflow_instance_repository: Arc<dyn WorkflowInstanceRepository>,
    s3_client: Arc<dyn S3Client>,
    clock: Arc<dyn Clock>,
}

impl DocumentUseCaseImpl {
    pub fn new(
        document_repository: Arc<dyn DocumentRepository>,
        workflow_instance_repository: Arc<dyn WorkflowInstanceRepository>,
        s3_client: Arc<dyn S3Client>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            document_repository,
            workflow_instance_repository,
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
        if document.status() != DocumentStatus::Uploading {
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
            return Err(CoreError::BadRequest(
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

    /// ダウンロード URL を発行する
    ///
    /// active なドキュメントに対して Presigned GET URL を発行する。
    pub async fn generate_download_url(
        &self,
        document_id: &DocumentId,
        tenant_id: &TenantId,
    ) -> Result<DownloadUrlOutput, CoreError> {
        let document = self
            .document_repository
            .find_by_id(document_id, tenant_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("ドキュメントが見つかりません".to_string()))?;

        if document.status() != DocumentStatus::Active {
            return Err(CoreError::BadRequest(format!(
                "ドキュメントのステータスが active ではありません: {}",
                document.status()
            )));
        }

        let download_url = self
            .s3_client
            .generate_presigned_get_url(document.s3_key(), DOWNLOAD_URL_EXPIRES_IN)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;

        Ok(DownloadUrlOutput {
            download_url,
            expires_in: DOWNLOAD_URL_EXPIRES_IN.as_secs(),
        })
    }

    /// ドキュメントをソフトデリートする
    ///
    /// 権限チェック:
    /// 1. テナント管理者 → 許可
    /// 2. アップロード者本人 → 許可
    /// 3. それ以外 → 拒否
    ///
    /// ワークフロー添付の場合: ワークフローが Draft でなければ拒否
    pub async fn soft_delete_document(&self, input: SoftDeleteInput) -> Result<(), CoreError> {
        let document = self
            .document_repository
            .find_by_id(&input.document_id, &input.tenant_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("ドキュメントが見つかりません".to_string()))?;

        // 権限チェック
        let is_uploader = document
            .uploaded_by()
            .is_some_and(|uid| uid == &input.user_id);
        if !input.is_tenant_admin && !is_uploader {
            return Err(CoreError::Forbidden(
                "このドキュメントを削除する権限がありません".to_string(),
            ));
        }

        // ワークフロー添付の場合、ワークフロー状態チェック
        if let UploadContext::Workflow(workflow_instance_id) = document.upload_context() {
            let workflow = self
                .workflow_instance_repository
                .find_by_id(workflow_instance_id, &input.tenant_id)
                .await?
                .ok_or_else(|| {
                    CoreError::Internal("ワークフローインスタンスが見つかりません".to_string())
                })?;

            if workflow.status() != WorkflowInstanceStatus::Draft {
                return Err(CoreError::BadRequest(
                    "下書き以外のワークフローの添付ファイルは削除できません".to_string(),
                ));
            }
        }

        // ソフトデリート
        let now = self.clock.now();
        let _deleted = document
            .soft_delete(now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        self.document_repository
            .soft_delete(&input.document_id, &input.tenant_id, now)
            .await?;

        Ok(())
    }

    /// フォルダ内のドキュメント一覧を取得する
    pub async fn list_documents(
        &self,
        folder_id: &FolderId,
        tenant_id: &TenantId,
    ) -> Result<Vec<Document>, CoreError> {
        let documents = self
            .document_repository
            .list_by_folder(folder_id, tenant_id)
            .await?;
        Ok(documents)
    }

    /// ワークフロー添付ファイル一覧を取得する
    pub async fn list_workflow_attachments(
        &self,
        workflow_instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Vec<Document>, CoreError> {
        let documents = self
            .document_repository
            .list_by_workflow(workflow_instance_id, tenant_id)
            .await?;
        Ok(documents)
    }
}
