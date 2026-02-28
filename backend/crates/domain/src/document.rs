//! # ドキュメント
//!
//! Presigned URL 方式のファイルアップロードで管理されるドキュメントのドメインモデル。
//!
//! ## アップロードフロー
//!
//! 1. クライアントがファイルメタデータを送信
//! 2. サーバーが Presigned PUT URL を発行（ステータス: `uploading`）
//! 3. クライアントが S3 に直接アップロード
//! 4. クライアントがアップロード完了を通知（ステータス: `active`）
//!
//! → 詳細設計: [ドキュメント管理設計](../../../../docs/40_詳細設計書/17_ドキュメント管理設計.md)
//!
//! ## 設計判断
//!
//! - `UploadContext` enum で `folder_id` XOR `workflow_instance_id` を型レベルで強制
//! - `FileValidation` でファイルの Content-Type・サイズ・数量を検証
//! - `S3KeyGenerator` でテナント分離されたオブジェクトキーを生成

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::{
    DomainError,
    folder::FolderId,
    tenant::TenantId,
    user::UserId,
    workflow::WorkflowInstanceId,
};

// ============================================================================
// DocumentId
// ============================================================================

define_uuid_id! {
    /// ドキュメントの一意識別子
    pub struct DocumentId;
}

// ============================================================================
// DocumentStatus
// ============================================================================

/// ドキュメントのステータス
///
/// 状態遷移: `uploading` → `active` → `deleted`
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr, strum::Display,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "snake_case")]
pub enum DocumentStatus {
    /// Presigned URL 発行済み、S3 アップロード待ち
    Uploading,
    /// アップロード完了、利用可能
    Active,
    /// ソフトデリート済み
    Deleted,
}

impl std::str::FromStr for DocumentStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "uploading" => Ok(Self::Uploading),
            "active" => Ok(Self::Active),
            "deleted" => Ok(Self::Deleted),
            _ => Err(DomainError::Validation(format!(
                "不正なドキュメントステータス: {}",
                s
            ))),
        }
    }
}

// ============================================================================
// UploadContext
// ============================================================================

/// アップロード先のコンテキスト
///
/// `folder_id` と `workflow_instance_id` の排他制約を型レベルで強制する。
/// DB は 2 つの nullable カラム + CHECK 制約で格納する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadContext {
    /// フォルダ内のドキュメント（ドキュメント管理画面からのアップロード）
    Folder(FolderId),
    /// ワークフロー申請の添付ファイル
    Workflow(WorkflowInstanceId),
}

impl UploadContext {
    /// フォルダ ID を取得する（Folder コンテキストの場合のみ）
    pub fn folder_id(&self) -> Option<&FolderId> {
        match self {
            UploadContext::Folder(id) => Some(id),
            UploadContext::Workflow(_) => None,
        }
    }

    /// ワークフローインスタンス ID を取得する（Workflow コンテキストの場合のみ）
    pub fn workflow_instance_id(&self) -> Option<&WorkflowInstanceId> {
        match self {
            UploadContext::Folder(_) => None,
            UploadContext::Workflow(id) => Some(id),
        }
    }
}

// ============================================================================
// FileValidation
// ============================================================================

/// ファイルアップロードのバリデーション
///
/// Content-Type、ファイルサイズ、ファイル数の制限を検証する。
/// 制限値は設計書に準拠。
pub struct FileValidation;

impl FileValidation {
    /// 対応 Content-Type の一覧
    const ALLOWED_CONTENT_TYPES: &[&str] = &[
        "application/pdf",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.ms-excel",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "text/plain",
        "text/csv",
        "image/png",
        "image/jpeg",
        "image/gif",
    ];
    /// 最大ファイル数
    pub const MAX_FILE_COUNT: usize = 10;
    /// 最大ファイルサイズ（20 MB）
    pub const MAX_FILE_SIZE: i64 = 20 * 1024 * 1024;
    /// 最大合計サイズ（100 MB）
    pub const MAX_TOTAL_SIZE: i64 = 100 * 1024 * 1024;

    /// 単一ファイルのバリデーション
    ///
    /// Content-Type とファイルサイズを検証する。
    pub fn validate_file(content_type: &str, content_length: i64) -> Result<(), DomainError> {
        if !Self::ALLOWED_CONTENT_TYPES.contains(&content_type) {
            return Err(DomainError::Validation(format!(
                "非対応のファイル形式です: {}",
                content_type
            )));
        }

        if content_length <= 0 {
            return Err(DomainError::Validation(
                "ファイルサイズは 1 バイト以上である必要があります".to_string(),
            ));
        }

        if content_length > Self::MAX_FILE_SIZE {
            return Err(DomainError::Validation(format!(
                "ファイルサイズが上限（{} MB）を超えています",
                Self::MAX_FILE_SIZE / (1024 * 1024)
            )));
        }

        Ok(())
    }

    /// 合計ファイル数・サイズのバリデーション
    ///
    /// 既存ドキュメントの数と合計サイズに対し、新しいファイルの追加が可能かを検証する。
    pub fn validate_total(
        existing_count: usize,
        existing_total_size: i64,
        new_size: i64,
    ) -> Result<(), DomainError> {
        if existing_count >= Self::MAX_FILE_COUNT {
            return Err(DomainError::Validation(format!(
                "ファイル数が上限（{}）に達しています",
                Self::MAX_FILE_COUNT
            )));
        }

        if existing_total_size + new_size > Self::MAX_TOTAL_SIZE {
            return Err(DomainError::Validation(format!(
                "合計ファイルサイズが上限（{} MB）を超えます",
                Self::MAX_TOTAL_SIZE / (1024 * 1024)
            )));
        }

        Ok(())
    }
}

// ============================================================================
// S3KeyGenerator
// ============================================================================

/// S3 オブジェクトキーの生成
///
/// テナント分離されたキーを生成する。
/// - ワークフロー: `{tenant_id}/workflows/{instance_id}/{document_id}_{filename}`
/// - フォルダ: `{tenant_id}/folders/{folder_id}/{document_id}_{filename}`
pub struct S3KeyGenerator;

impl S3KeyGenerator {
    /// S3 オブジェクトキーを生成する
    pub fn generate(
        tenant_id: &TenantId,
        context: &UploadContext,
        document_id: &DocumentId,
        filename: &str,
    ) -> String {
        match context {
            UploadContext::Workflow(instance_id) => {
                format!(
                    "{}/workflows/{}/{}_{}",
                    tenant_id.as_uuid(),
                    instance_id.as_uuid(),
                    document_id.as_uuid(),
                    filename
                )
            }
            UploadContext::Folder(folder_id) => {
                format!(
                    "{}/folders/{}/{}_{}",
                    tenant_id.as_uuid(),
                    folder_id.as_uuid(),
                    document_id.as_uuid(),
                    filename
                )
            }
        }
    }
}

// ============================================================================
// Document
// ============================================================================

/// ドキュメントエンティティ
///
/// Presigned URL 方式でアップロードされるファイルのメタデータを管理する。
#[derive(Debug, Clone)]
pub struct Document {
    id: DocumentId,
    tenant_id: TenantId,
    filename: String,
    content_type: String,
    size: i64,
    s3_key: String,
    upload_context: UploadContext,
    status: DocumentStatus,
    uploaded_by: Option<UserId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl Document {
    /// アップロード中のドキュメントを作成する
    ///
    /// Presigned URL 発行時にステータス `uploading` で作成される。
    // FIXME: 引数が多い。作成パラメータを値オブジェクトにまとめて引数を削減する
    #[allow(clippy::too_many_arguments)]
    pub fn new_uploading(
        id: DocumentId,
        tenant_id: TenantId,
        filename: String,
        content_type: String,
        size: i64,
        s3_key: String,
        upload_context: UploadContext,
        uploaded_by: Option<UserId>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            filename,
            content_type,
            size,
            s3_key,
            upload_context,
            status: DocumentStatus::Uploading,
            uploaded_by,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// アップロード完了を確認し、ステータスを `active` に遷移する
    ///
    /// `uploading` 以外のステータスからの遷移はエラーになる。
    pub fn confirm(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
        if self.status != DocumentStatus::Uploading {
            return Err(DomainError::Validation(format!(
                "ドキュメントのステータスが uploading ではありません: {}",
                self.status
            )));
        }

        Ok(Self {
            status: DocumentStatus::Active,
            updated_at: now,
            ..self
        })
    }

    /// ソフトデリートを実行し、ステータスを `deleted` に遷移する
    ///
    /// `active` 以外のステータスからの遷移はエラーになる。
    pub fn soft_delete(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
        if self.status != DocumentStatus::Active {
            return Err(DomainError::Validation(format!(
                "ドキュメントのステータスが active ではありません: {}",
                self.status
            )));
        }
        Ok(Self {
            status: DocumentStatus::Deleted,
            updated_at: now,
            deleted_at: Some(now),
            ..self
        })
    }

    /// DB からエンティティを復元する（バリデーションをスキップ）
    // FIXME: 引数が多い。DB 行データの中間構造体を経由して引数を削減する
    #[allow(clippy::too_many_arguments)]
    pub fn from_db(
        id: DocumentId,
        tenant_id: TenantId,
        filename: String,
        content_type: String,
        size: i64,
        s3_key: String,
        upload_context: UploadContext,
        status: DocumentStatus,
        uploaded_by: Option<UserId>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        deleted_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            filename,
            content_type,
            size,
            s3_key,
            upload_context,
            status,
            uploaded_by,
            created_at,
            updated_at,
            deleted_at,
        }
    }

    // --- Getters ---

    pub fn id(&self) -> &DocumentId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub fn size(&self) -> i64 {
        self.size
    }

    pub fn s3_key(&self) -> &str {
        &self.s3_key
    }

    pub fn upload_context(&self) -> &UploadContext {
        &self.upload_context
    }

    pub fn status(&self) -> DocumentStatus {
        self.status
    }

    pub fn uploaded_by(&self) -> Option<&UserId> {
        self.uploaded_by.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- DocumentStatus ---

    #[test]
    fn test_document_statusをlowercase文字列にシリアライズできる() {
        let json = serde_json::to_string(&DocumentStatus::Uploading).unwrap();
        assert_eq!(json, "\"uploading\"");

        let json = serde_json::to_string(&DocumentStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");

        let json = serde_json::to_string(&DocumentStatus::Deleted).unwrap();
        assert_eq!(json, "\"deleted\"");
    }

    #[test]
    fn test_document_status_from_strで有効な文字列をパースできる() {
        assert_eq!(
            "uploading".parse::<DocumentStatus>().unwrap(),
            DocumentStatus::Uploading
        );
        assert_eq!(
            "active".parse::<DocumentStatus>().unwrap(),
            DocumentStatus::Active
        );
        assert_eq!(
            "deleted".parse::<DocumentStatus>().unwrap(),
            DocumentStatus::Deleted
        );
    }

    #[test]
    fn test_document_status_from_strで不正な文字列にエラーを返す() {
        let result = "invalid".parse::<DocumentStatus>();
        assert!(result.is_err());
    }

    // --- FileValidation::validate_file ---

    #[test]
    fn test_validate_fileでpdfを受け入れる() {
        let result = FileValidation::validate_file("application/pdf", 1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fileで全許可content_typeを受け入れる() {
        for ct in FileValidation::ALLOWED_CONTENT_TYPES {
            let result = FileValidation::validate_file(ct, 1024);
            assert!(result.is_ok(), "Content-Type {} が拒否された", ct);
        }
    }

    #[test]
    fn test_validate_fileで非対応content_typeを拒否する() {
        let result = FileValidation::validate_file("application/zip", 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_fileでゼロサイズファイルを拒否する() {
        let result = FileValidation::validate_file("application/pdf", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_fileで最大サイズ超過を拒否する() {
        let result =
            FileValidation::validate_file("application/pdf", FileValidation::MAX_FILE_SIZE + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_fileで最大サイズちょうどを受け入れる() {
        let result =
            FileValidation::validate_file("application/pdf", FileValidation::MAX_FILE_SIZE);
        assert!(result.is_ok());
    }

    // --- FileValidation::validate_total ---

    #[test]
    fn test_validate_totalで制限内を受け入れる() {
        let result = FileValidation::validate_total(5, 50 * 1024 * 1024, 10 * 1024 * 1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_totalでファイル数上限超過を拒否する() {
        let result = FileValidation::validate_total(FileValidation::MAX_FILE_COUNT, 0, 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_totalで合計サイズ上限超過を拒否する() {
        let result = FileValidation::validate_total(0, FileValidation::MAX_TOTAL_SIZE, 1);
        assert!(result.is_err());
    }

    // --- S3KeyGenerator ---

    #[test]
    fn test_s3_key_generatorでワークフローコンテキストのキーを生成する() {
        let tenant_id = TenantId::new();
        let instance_id = WorkflowInstanceId::new();
        let document_id = DocumentId::new();
        let context = UploadContext::Workflow(instance_id.clone());

        let key = S3KeyGenerator::generate(&tenant_id, &context, &document_id, "領収書.pdf");

        let expected = format!(
            "{}/workflows/{}/{}_領収書.pdf",
            tenant_id.as_uuid(),
            instance_id.as_uuid(),
            document_id.as_uuid()
        );
        assert_eq!(key, expected);
    }

    #[test]
    fn test_s3_key_generatorでフォルダコンテキストのキーを生成する() {
        let tenant_id = TenantId::new();
        let folder_id = FolderId::new();
        let document_id = DocumentId::new();
        let context = UploadContext::Folder(folder_id.clone());

        let key = S3KeyGenerator::generate(&tenant_id, &context, &document_id, "見積書.xlsx");

        let expected = format!(
            "{}/folders/{}/{}_見積書.xlsx",
            tenant_id.as_uuid(),
            folder_id.as_uuid(),
            document_id.as_uuid()
        );
        assert_eq!(key, expected);
    }

    // --- Document ---

    fn fixed_now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn test_document_new_uploadingでuploadingステータスのドキュメントを作成する() {
        let tenant_id = TenantId::new();
        let folder_id = FolderId::new();
        let now = fixed_now();

        let doc = Document::new_uploading(
            DocumentId::new(),
            tenant_id,
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "key".to_string(),
            UploadContext::Folder(folder_id),
            Some(UserId::new()),
            now,
        );

        assert_eq!(doc.status(), DocumentStatus::Uploading);
        assert_eq!(doc.filename(), "test.pdf");
        assert_eq!(doc.content_type(), "application/pdf");
        assert_eq!(doc.size(), 1024);
        assert_eq!(doc.created_at(), now);
    }

    #[test]
    fn test_document_confirmでuploadingからactiveに遷移する() {
        let now = fixed_now();
        let later = DateTime::from_timestamp(1_700_001_000, 0).unwrap();

        let doc = Document::new_uploading(
            DocumentId::new(),
            TenantId::new(),
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "key".to_string(),
            UploadContext::Folder(FolderId::new()),
            None,
            now,
        );

        let confirmed = doc.confirm(later).unwrap();
        assert_eq!(confirmed.status(), DocumentStatus::Active);
        assert_eq!(confirmed.updated_at(), later);
    }

    // --- Document::soft_delete ---

    #[test]
    fn test_document_soft_deleteでactiveからdeletedに遷移する() {
        let now = fixed_now();
        let later = DateTime::from_timestamp(1_700_001_000, 0).unwrap();

        let doc = Document::from_db(
            DocumentId::new(),
            TenantId::new(),
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "key".to_string(),
            UploadContext::Folder(FolderId::new()),
            DocumentStatus::Active,
            Some(UserId::new()),
            now,
            now,
            None,
        );

        let deleted = doc.soft_delete(later).unwrap();
        assert_eq!(deleted.status(), DocumentStatus::Deleted);
        assert_eq!(deleted.updated_at(), later);
        assert_eq!(deleted.deleted_at(), Some(later));
    }

    #[test]
    fn test_document_soft_deleteでuploadingステータスからの遷移を拒否する() {
        let now = fixed_now();
        let later = DateTime::from_timestamp(1_700_001_000, 0).unwrap();

        let doc = Document::new_uploading(
            DocumentId::new(),
            TenantId::new(),
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "key".to_string(),
            UploadContext::Folder(FolderId::new()),
            None,
            now,
        );

        let result = doc.soft_delete(later);
        assert!(result.is_err());
    }

    #[test]
    fn test_document_soft_deleteでdeletedステータスからの遷移を拒否する() {
        let now = fixed_now();
        let later = DateTime::from_timestamp(1_700_001_000, 0).unwrap();

        let doc = Document::from_db(
            DocumentId::new(),
            TenantId::new(),
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "key".to_string(),
            UploadContext::Folder(FolderId::new()),
            DocumentStatus::Deleted,
            None,
            now,
            now,
            Some(now),
        );

        let result = doc.soft_delete(later);
        assert!(result.is_err());
    }

    #[test]
    fn test_document_confirmで非uploadingステータスからの遷移を拒否する() {
        let now = fixed_now();
        let later = DateTime::from_timestamp(1_700_001_000, 0).unwrap();

        let doc = Document::from_db(
            DocumentId::new(),
            TenantId::new(),
            "test.pdf".to_string(),
            "application/pdf".to_string(),
            1024,
            "key".to_string(),
            UploadContext::Folder(FolderId::new()),
            DocumentStatus::Active,
            None,
            now,
            now,
            None,
        );

        let result = doc.confirm(later);
        assert!(result.is_err());
    }
}
