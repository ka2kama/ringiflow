# 実装計画: #881 ファイルアップロード API

## Context

Epic #406（ドキュメント管理）の Story。#880（S3 基盤）が完了し、S3Client trait + MinIO Docker が利用可能になった。Presigned URL 方式の Upload URL 発行 + Confirm の 2 エンドポイントを全レイヤー（Domain → Migration → Repository → UseCase → Core Handler → BFF Handler）で実装する。

詳細設計: `docs/40_詳細設計書/17_ドキュメント管理設計.md`

## スコープ

対象:
- `documents` テーブルマイグレーション（RLS 付き）
- Document ドメインモデル（エンティティ、FileValidation、S3KeyGenerator）
- `DocumentRepository` trait + PostgreSQL 実装
- `DocumentUseCaseImpl`（S3Client 依存）
- `POST /internal/documents/upload-url` + `POST /internal/documents/{id}/confirm`（Core Service）
- `POST /api/v1/documents/upload-url` + `POST /api/v1/documents/{id}/confirm`（BFF）
- S3Client の AppState 注入（`TODO(#881)` 解消）

対象外:
- ダウンロード URL（#882）
- ファイル削除（#882）
- ドキュメント一覧（#882）
- フロントエンド（#885）
- ワークフロー定義のファイルフィールド拡張（#884）

## 設計判断

### 1. UploadContext enum による XOR 制約

`folder_id` と `workflow_instance_id` の排他制約を型レベルで強制する。

```rust
pub enum UploadContext {
    Folder(FolderId),
    Workflow(WorkflowInstanceId),
}
```

DB は 2 つの nullable カラム + CHECK 制約。Repository 層で enum ↔ カラム変換を行う。

理由: 「不正な状態を表現不可能にする」原則（ADR-054）。Option 2 つのフラット構造だと domain 層で XOR 違反が表現可能になる。

### 2. S3 キー生成をドメインロジックとして配置

`S3KeyGenerator` を domain クレートに配置する。テナント分離・コンテキスト別パスの知識はビジネスルール。

### 3. DocumentStatus の遷移メソッド

`Document::confirm(self, now)` で uploading → active の遷移を行い、非 uploading からの遷移はエラーにする。設計書の uploading → active → deleted の状態遷移を型安全に管理。

## 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ユーザーがファイルメタデータを送信し、upload URL と document_id を受け取る | 正常系 | ハンドラ, ユニット |
| 2 | ユーザーが S3 にアップロード後、confirm を呼び document が active になる | 正常系 | ハンドラ |
| 3 | 非対応 content_type で upload URL を要求 | 準正常系 | ハンドラ, ユニット |
| 4 | 20MB 超のファイルサイズで upload URL を要求 | 準正常系 | ハンドラ, ユニット |
| 5 | folder_id も workflow_instance_id も未指定で upload URL を要求 | 準正常系 | ハンドラ, ユニット |
| 6 | folder_id と workflow_instance_id の両方を指定して upload URL を要求 | 準正常系 | ハンドラ, ユニット |
| 7 | ファイル数上限（10）超過で upload URL を要求 | 準正常系 | ハンドラ, ユニット |
| 8 | 合計サイズ上限（100MB）超過で upload URL を要求 | 準正常系 | ハンドラ, ユニット |
| 9 | 存在しない document_id で confirm を呼ぶ | 準正常系 | ハンドラ |
| 10 | uploading 以外のステータスで confirm を呼ぶ | 準正常系 | ハンドラ, ユニット |
| 11 | S3 にファイルが存在しない状態で confirm を呼ぶ | 準正常系 | ハンドラ |

---

## Phase 1: ドメインモデル + ユニットテスト

### 作成ファイル
- `backend/crates/domain/src/document.rs`

### 変更ファイル
- `backend/crates/domain/src/lib.rs` — `pub mod document;` 追加

### 確認事項
- 型: `WorkflowInstanceStatus` の derive マクロと `FromStr` パターン → `backend/crates/domain/src/workflow/instance.rs:28-68`
- 型: `define_uuid_id!` マクロ → `backend/crates/domain/src/macros.rs`
- 型: `FolderId`, `WorkflowInstanceId` → 各定義ファイル
- 型: `DomainError` バリアント → `backend/crates/domain/src/error.rs`
- パターン: `FolderName` 手動 Newtype → `backend/crates/domain/src/folder.rs`
- ライブラリ: `strum::IntoStaticStr`, `strum::Display` — Grep で既存使用確認

### 型定義

```rust
define_uuid_id! { pub struct DocumentId; }

// WorkflowInstanceStatus パターンを踏襲
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "snake_case")]
pub enum DocumentStatus { Uploading, Active, Deleted }

impl FromStr for DocumentStatus { ... } // 手動実装

pub enum UploadContext {
    Folder(FolderId),
    Workflow(WorkflowInstanceId),
}

pub struct FileValidation;
impl FileValidation {
    const ALLOWED_CONTENT_TYPES: &[&str] = &[...]; // 設計書の 10 種
    const MAX_FILE_SIZE: i64 = 20 * 1024 * 1024;
    const MAX_TOTAL_SIZE: i64 = 100 * 1024 * 1024;
    const MAX_FILE_COUNT: usize = 10;
    pub fn validate_file(content_type: &str, content_length: i64) -> Result<(), DomainError> { ... }
    pub fn validate_total(existing_count: usize, existing_total_size: i64, new_size: i64) -> Result<(), DomainError> { ... }
}

pub struct S3KeyGenerator;
impl S3KeyGenerator {
    // "{tenant_id}/workflows/{instance_id}/{document_id}_{filename}"
    // "{tenant_id}/folders/{folder_id}/{document_id}_{filename}"
    pub fn generate(tenant_id: &TenantId, context: &UploadContext, document_id: &DocumentId, filename: &str) -> String { ... }
}

pub struct Document { id, tenant_id, filename, content_type, size, s3_key, upload_context, status, uploaded_by, created_at, updated_at, deleted_at }
impl Document {
    pub fn new_uploading(...) -> Result<Self, DomainError> { ... }
    pub fn confirm(self, now: DateTime<Utc>) -> Result<Self, DomainError> { ... }
    pub fn from_db(...) -> Self { ... }
    // getters
}
```

### テストリスト

ユニットテスト:
- [ ] `DocumentStatus` を lowercase 文字列にシリアライズできる
- [ ] `DocumentStatus::from_str` で有効な文字列をパースできる
- [ ] `DocumentStatus::from_str` で不正な文字列にエラーを返す
- [ ] `FileValidation::validate_file` で PDF を受け入れる
- [ ] `FileValidation::validate_file` で全許可 Content-Type を受け入れる
- [ ] `FileValidation::validate_file` で非対応 Content-Type を拒否する
- [ ] `FileValidation::validate_file` でゼロサイズファイルを拒否する
- [ ] `FileValidation::validate_file` で最大サイズ超過を拒否する
- [ ] `FileValidation::validate_file` で最大サイズちょうどを受け入れる
- [ ] `FileValidation::validate_total` で制限内を受け入れる
- [ ] `FileValidation::validate_total` でファイル数上限超過を拒否する
- [ ] `FileValidation::validate_total` で合計サイズ上限超過を拒否する
- [ ] `S3KeyGenerator::generate` でワークフローコンテキストのキーを生成する
- [ ] `S3KeyGenerator::generate` でフォルダコンテキストのキーを生成する
- [ ] `Document::new_uploading` で uploading ステータスのドキュメントを作成する
- [ ] `Document::confirm` で uploading から active に遷移する
- [ ] `Document::confirm` で非 uploading ステータスからの遷移を拒否する

ハンドラテスト: 該当なし
API テスト: 該当なし
E2E テスト: 該当なし

---

## Phase 2: データベースマイグレーション

### 作成ファイル
- `backend/migrations/20260226000001_create_documents.sql`

### 確認事項
- パターン: `backend/migrations/20260225000001_create_folders.sql` — RLS ポリシー構文、インデックス、CHECK 制約

### マイグレーション内容

```sql
CREATE TABLE documents (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    filename              VARCHAR(255) NOT NULL,
    content_type          VARCHAR(100) NOT NULL,
    size                  BIGINT NOT NULL,
    s3_key                VARCHAR(1000) NOT NULL,
    folder_id             UUID REFERENCES folders(id) ON DELETE SET NULL,
    workflow_instance_id  UUID REFERENCES workflow_instances(id) ON DELETE CASCADE,
    status                VARCHAR(20) NOT NULL DEFAULT 'uploading',
    uploaded_by           UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at            TIMESTAMPTZ,
    CONSTRAINT documents_context_check CHECK (
        (folder_id IS NOT NULL AND workflow_instance_id IS NULL)
        OR (folder_id IS NULL AND workflow_instance_id IS NOT NULL)
    )
);

ALTER TABLE documents ENABLE ROW LEVEL SECURITY;

-- folders.sql と同じ NULLIF パターン
CREATE POLICY tenant_isolation ON documents
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

CREATE INDEX idx_documents_tenant_id ON documents (tenant_id);
CREATE INDEX idx_documents_folder_id ON documents (folder_id) WHERE folder_id IS NOT NULL;
CREATE INDEX idx_documents_workflow_instance_id ON documents (workflow_instance_id) WHERE workflow_instance_id IS NOT NULL;
CREATE INDEX idx_documents_status ON documents (status) WHERE status != 'deleted';
```

### テストリスト

ユニットテスト: 該当なし
ハンドラテスト: 該当なし
API テスト: 該当なし（マイグレーションは Phase 3 リポジトリで間接検証）
E2E テスト: 該当なし

---

## Phase 3: リポジトリ trait + PostgreSQL 実装

### 作成ファイル
- `backend/crates/infra/src/repository/document_repository.rs`

### 変更ファイル
- `backend/crates/infra/src/repository.rs` — `pub mod document_repository;` + re-export

### 確認事項
- 型: `Document`, `DocumentId`, `DocumentStatus`, `UploadContext` → Phase 1 の成果物
- パターン: `PostgresFolderRepository` → `backend/crates/infra/src/repository/folder_repository.rs`（トレイト定義、`sqlx::query!`、`from_db` 復元、`#[tracing::instrument]`）
- パターン: `InfraError` → `backend/crates/infra/src/error.rs`

### 型定義

```rust
#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn find_by_id(&self, id: &DocumentId, tenant_id: &TenantId) -> Result<Option<Document>, InfraError>;
    async fn insert(&self, document: &Document) -> Result<(), InfraError>;
    async fn update_status(&self, id: &DocumentId, status: DocumentStatus, tenant_id: &TenantId, now: DateTime<Utc>) -> Result<(), InfraError>;
    async fn count_and_total_size_by_folder(&self, folder_id: &FolderId, tenant_id: &TenantId) -> Result<(usize, i64), InfraError>;
    async fn count_and_total_size_by_workflow(&self, workflow_instance_id: &WorkflowInstanceId, tenant_id: &TenantId) -> Result<(usize, i64), InfraError>;
}

pub struct PostgresDocumentRepository { pool: PgPool }
```

`from_db` 復元で `UploadContext` の変換:
```rust
let upload_context = match (row.folder_id, row.workflow_instance_id) {
    (Some(fid), None) => UploadContext::Folder(FolderId::from_uuid(fid)),
    (None, Some(wid)) => UploadContext::Workflow(WorkflowInstanceId::from_uuid(wid)),
    _ => unreachable!("CHECK constraint guarantees XOR"),
};
```

### テストリスト

ユニットテスト: 該当なし（リポジトリはハンドラテスト経由で間接テスト）
ハンドラテスト: 該当なし
API テスト: 該当なし
E2E テスト: 該当なし

---

## Phase 4: ユースケース

### 作成ファイル
- `backend/apps/core-service/src/usecase/document.rs`

### 変更ファイル
- `backend/apps/core-service/src/usecase.rs` — `pub mod document;` + re-export

### 確認事項
- 型: `S3Client` trait → `backend/crates/infra/src/s3.rs`（`generate_presigned_put_url` シグネチャ、`head_object` シグネチャ）
- 型: `CoreError` → `backend/apps/core-service/src/error.rs`（BadRequest, NotFound, Internal）
- パターン: `FolderUseCaseImpl` → `backend/apps/core-service/src/usecase/folder.rs`（DI パターン、エラーマッピング）
- 型: `Clock` trait → `backend/crates/domain/src/clock.rs`
- ライブラリ: `std::time::Duration` — presigned URL 有効期限

### 型定義

```rust
const UPLOAD_URL_EXPIRES_IN: Duration = Duration::from_secs(300); // 5 分

pub struct RequestUploadUrlInput {
    pub tenant_id: TenantId,
    pub filename: String,
    pub content_type: String,
    pub content_length: i64,
    pub folder_id: Option<Uuid>,
    pub workflow_instance_id: Option<Uuid>,
    pub uploaded_by: Uuid,
}

pub struct UploadUrlOutput {
    pub document_id: DocumentId,
    pub upload_url: String,
    pub expires_in: u64, // 秒
}

pub struct DocumentUseCaseImpl {
    document_repository: Arc<dyn DocumentRepository>,
    s3_client: Arc<dyn S3Client>,
    clock: Arc<dyn Clock>,
}
```

`request_upload_url` フロー:
1. `UploadContext` 構築（folder_id XOR workflow_instance_id）→ BadRequest
2. `FileValidation::validate_file` → BadRequest
3. `count_and_total_size_by_*` で既存ドキュメント取得
4. `FileValidation::validate_total` → BadRequest
5. `Document::new_uploading` でエンティティ作成
6. `DocumentRepository::insert`
7. `S3Client::generate_presigned_put_url`
8. `UploadUrlOutput` 返却

`confirm_upload` フロー:
1. `DocumentRepository::find_by_id` → NotFound
2. ステータスが `uploading` か確認 → BadRequest
3. `S3Client::head_object` でファイル存在確認 → Internal
4. `Document::confirm` で active に遷移
5. `DocumentRepository::update_status`
6. `Document` 返却

### テストリスト

ユニットテスト: 該当なし（ハンドラテストで間接テスト — プロジェクトパターンに従う）
ハンドラテスト: 該当なし（Phase 5 で実装）
API テスト: 該当なし
E2E テスト: 該当なし

---

## Phase 5: Core Service ハンドラ + ハンドラテスト + S3Client AppState 注入

### 作成ファイル
- `backend/apps/core-service/src/handler/document.rs`

### 変更ファイル
- `backend/apps/core-service/src/handler.rs` — `pub mod document;` + re-export
- `backend/apps/core-service/src/main.rs` — `TODO(#881)` 解消: `_s3_client` → `s3_client`、DocumentState 構築、ルート登録

### 確認事項
- パターン: `FolderState` + ハンドラ + テスト → `backend/apps/core-service/src/handler/folder.rs`（State 構造体、リクエスト/レスポンス DTO、`create_test_app`、スタブ）
- パターン: ルート登録 → `backend/apps/core-service/src/main.rs`（`.with_state()` パターン）
- 型: `ApiResponse<T>` → `backend/crates/shared/src/api_response.rs`

### 型定義

```rust
pub struct DocumentState { pub usecase: DocumentUseCaseImpl }

// リクエスト
#[derive(Debug, Deserialize)]
pub struct RequestUploadUrlRequest {
    pub tenant_id: Uuid, pub filename: String, pub content_type: String,
    pub content_length: i64, pub folder_id: Option<Uuid>,
    pub workflow_instance_id: Option<Uuid>, pub uploaded_by: Uuid,
}
#[derive(Debug, Deserialize)]
pub struct ConfirmUploadQuery { pub tenant_id: Uuid }

// レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadUrlDto { pub document_id: Uuid, pub upload_url: String, pub expires_in: u64 }
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentDto { pub id: Uuid, pub filename: String, pub content_type: String, pub size: i64, pub status: String, pub created_at: String }

// ハンドラ
pub async fn request_upload_url(State, Json) -> Result<impl IntoResponse, CoreError> { ... } // 200 OK
pub async fn confirm_upload(State, Path(document_id), Query) -> Result<impl IntoResponse, CoreError> { ... } // 200 OK
```

main.rs のルート登録:
```rust
// TODO(#881) を解消: _s3_client → s3_client
let s3_client: Arc<dyn S3Client> = Arc::new(AwsS3Client::new(...));
let document_repo: Arc<dyn DocumentRepository> = Arc::new(PostgresDocumentRepository::new(pool.clone()));
let document_usecase = DocumentUseCaseImpl::new(document_repo, s3_client, clock.clone());
let document_state = Arc::new(DocumentState { usecase: document_usecase });

.route("/internal/documents/upload-url", post(request_upload_url))
.route("/internal/documents/{document_id}/confirm", post(confirm_upload))
.with_state(document_state)
```

### テストリスト

ユニットテスト: 該当なし（Phase 1 で実装済み）

ハンドラテスト:
- [ ] `POST /internal/documents/upload-url` 正常系: 200 で upload URL と document_id が返る
- [ ] `POST /internal/documents/upload-url` 非対応 content_type で 400 が返る
- [ ] `POST /internal/documents/upload-url` サイズ超過で 400 が返る
- [ ] `POST /internal/documents/upload-url` folder_id も workflow_instance_id も未指定で 400 が返る
- [ ] `POST /internal/documents/upload-url` 両方指定で 400 が返る
- [ ] `POST /internal/documents/{id}/confirm` 正常系: 200 で active ドキュメントが返る
- [ ] `POST /internal/documents/{id}/confirm` 存在しない ID で 404 が返る
- [ ] `POST /internal/documents/{id}/confirm` 非 uploading ステータスで 400 が返る
- [ ] `POST /internal/documents/{id}/confirm` S3 にファイルなしで 500 が返る

API テスト: 該当なし（MinIO 統合テストは後続で対応）
E2E テスト: 該当なし

テストスタブ:
```rust
struct StubDocumentRepository { documents: Vec<Document> }
struct StubS3Client { presigned_url: String, existing_keys: HashSet<String> }
```

---

## Phase 6: BFF ハンドラ + クライアント

### 作成ファイル
- `backend/apps/bff/src/handler/document.rs`
- `backend/apps/bff/src/client/core_service/document_client.rs`

### 変更ファイル
- `backend/apps/bff/src/handler.rs` — `pub mod document;` + re-export
- `backend/apps/bff/src/client/core_service.rs` — `mod document_client;` + re-export
- `backend/apps/bff/src/client/core_service/client_impl.rs` — `CoreServiceClient` に `+ CoreServiceDocumentClient` 追加 + blanket impl 更新
- `backend/apps/bff/src/client/core_service/types.rs` — Core Service DTO 追加
- `backend/apps/bff/src/client/core_service/error.rs` — `DocumentNotFound` バリアント追加
- `backend/apps/bff/src/client.rs` — re-export 追加
- `backend/apps/bff/src/error.rs` — `DocumentNotFound` の `IntoResponse` 追加
- `backend/apps/bff/src/openapi.rs` — ハンドラ + "documents" タグ登録
- `backend/apps/bff/src/main.rs` — DocumentState + ルート登録
- `backend/apps/bff/tests/openapi_spec.rs` — パス数更新 + 新パスアサーション

### 確認事項
- パターン: `CoreServiceFolderClient` → `backend/apps/bff/src/client/core_service/folder_client.rs`
- パターン: BFF folder ハンドラ → `backend/apps/bff/src/handler/folder.rs`（セッション認証、DTO 変換）
- パターン: `handle_response` → `backend/apps/bff/src/client/core_service/response.rs`
- パターン: OpenAPI 登録 → `backend/apps/bff/src/openapi.rs`
- 型: `CoreServiceError` バリアント → `backend/apps/bff/src/client/core_service/error.rs`
- パターン: `IntoResponse for CoreServiceError` → `backend/apps/bff/src/error.rs`

### 型定義

```rust
// BFF Client Trait (ISP)
#[async_trait]
pub trait CoreServiceDocumentClient: Send + Sync {
    async fn request_upload_url(&self, req: &RequestUploadUrlCoreRequest) -> Result<ApiResponse<UploadUrlCoreDto>, CoreServiceError>;
    async fn confirm_upload(&self, document_id: Uuid, tenant_id: Uuid) -> Result<ApiResponse<DocumentDetailCoreDto>, CoreServiceError>;
}

// BFF Handler
pub struct DocumentState {
    pub core_service_client: Arc<dyn CoreServiceDocumentClient>,
    pub session_manager: Arc<dyn SessionManager>,
}

// BFF リクエスト（tenant_id/uploaded_by はセッションから取得）
#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestUploadUrlRequest {
    pub filename: String, pub content_type: String, pub content_length: i64,
    pub folder_id: Option<Uuid>, pub workflow_instance_id: Option<Uuid>,
}

// BFF レスポンス（String ID）
#[derive(Debug, Serialize, ToSchema)]
pub struct UploadUrlData { pub document_id: String, pub upload_url: String, pub expires_in: u64 }
#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentData { pub id: String, pub filename: String, pub content_type: String, pub size: i64, pub status: String, pub created_at: String }
```

### テストリスト

ユニットテスト: 該当なし
ハンドラテスト: 該当なし（BFF はプロキシ層のため、Core Service ハンドラテストでロジックをカバー）
API テスト: 該当なし
E2E テスト: 該当なし

---

## 品質ゲート前の追加作業

Phase 6 完了後:
1. `just openapi-generate` → `openapi/openapi.yaml` 再生成
2. `just sqlx-prepare` → `.sqlx/query-*.json` 再生成
3. `just check-all` 実行
4. ベースライン更新が必要か確認（`FILE_SIZE_MAX_COUNT` 等）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | 設計書の RLS ポリシー構文が実際のマイグレーションと異なる | アーキテクチャ不整合 | 実際の `NULLIF(current_setting('app.tenant_id', true), '')::UUID` パターンを採用 |
| 2回目 | UseCase テストをどこに配置するか | 既存手段の見落とし | プロジェクトパターンではハンドラテストで間接テスト（folder.rs と同じ） |
| 3回目 | `count_and_total_size` が folder/workflow で別クエリ必要 | 不完全なパス | Repository に `count_and_total_size_by_folder` と `count_and_total_size_by_workflow` の 2 メソッド |
| 4回目 | BFF の `CoreServiceClient` スーパートレイトに Document 追加必要 | アーキテクチャ不整合 | Phase 6 で `client_impl.rs` の更新を明示 |
| 5回目 | OpenAPI spec テストのパス数アサーション更新が必要 | 未定義 | Phase 6 で `openapi_spec.rs` 更新を明示 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue #881 の全スコープが計画に含まれている | OK | documents テーブル、DocumentRepository、upload-url、confirm、バリデーション、S3 キー生成、RLS を全 Phase でカバー |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全ての型シグネチャ、ファイルパス、パターン参照が具体的 |
| 3 | 設計判断の完結性 | 全差異に判断が記載されている | OK | UploadContext enum、S3 キー配置、confirm 遷移の 3 判断を記録 |
| 4 | スコープ境界 | 対象と対象外が明記されている | OK | ダウンロード、削除、一覧、フロントエンドを対象外に明示 |
| 5 | 技術的前提 | 前提が考慮されている | OK | RLS 構文、strum derive、axum State パターン、S3Client シグネチャを実コードで検証済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 詳細設計書の API 仕様・データモデルと一致（RLS 構文は実装に合わせて修正） |
