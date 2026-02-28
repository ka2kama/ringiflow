# #882 ファイルダウンロード・削除 API 実装計画

## Context

Epic #406（ドキュメント管理）の一部。#881（ファイルアップロード API）で構築された基盤の上に、ダウンロード URL 発行・ソフトデリート・一覧取得の 4 エンドポイントを追加し、ドキュメント管理のバックエンド API を完成させる。

## スコープ

対象:
- `POST /api/v1/documents/{id}/download-url`（ダウンロード URL 発行）
- `DELETE /api/v1/documents/{id}`（ソフトデリート）
- `GET /api/v1/documents`（ファイル一覧取得）
- `GET /api/v1/workflows/{id}/attachments`（ワークフロー添付ファイル一覧）
- 削除権限チェック（ワークフローステータス確認含む）

対象外:
- フロントエンド実装
- API テスト（Hurl）
- E2E テスト（Playwright）

## 設計判断

### 1. 削除権限チェックでの WorkflowInstanceRepository 依存追加

`DocumentUseCaseImpl` に `WorkflowInstanceRepository` を追加注入する。削除はドキュメント管理の一部であり、別ユースケースに分離するほどの複雑さではない。削除時にのみ使用する点を doc コメントに明記する。

### 2. `soft_delete` メソッド追加（`update_status` 拡張ではなく）

既存 `update_status` は `status` + `updated_at` のみ更新する。ソフトデリートは `status` + `updated_at` + `deleted_at` の 3 カラム更新が必要。異なるドメインの意味を 1 つのメソッドに詰め込むより、意図が明確な別メソッドの方がシンプル。

### 3. 管理者判定

BFF で `session_data.roles().iter().any(|r| r == "tenant_admin")` で判定し、Core Service に `is_tenant_admin: bool` パラメータとして渡す。

### 4. 行→エンティティ変換の共通化

`find_by_id` と `list_by_*` で行→`Document` 変換ロジックが重複する。ヘルパー関数 `row_to_document` を抽出する。

## Phase 分割

| Phase | 内容 | レイヤー |
|-------|------|---------|
| 1 | Domain: `Document::soft_delete` メソッド | domain |
| 2 | Repository: `soft_delete`, `list_by_folder`, `list_by_workflow` | infra |
| 3 | UseCase: 4 メソッド + 権限チェック | core-service/usecase |
| 4 | Core Handler: 4 内部エンドポイント + テスト | core-service/handler |
| 5 | BFF Client + Handler: 4 公開エンドポイント | bff |

---

## Phase 1: Domain - `Document::soft_delete` メソッド

### 確認事項

- 型: `Document` エンティティ → `backend/crates/domain/src/document.rs`
- パターン: `Document::confirm` メソッド（状態遷移パターン、L297-310）

### 実装

```rust
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
```

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | active ドキュメントを削除する | 正常系 | ユニット |
| 2 | uploading ドキュメントを削除しようとする | 準正常系 | ユニット |
| 3 | deleted ドキュメントを再度削除しようとする | 準正常系 | ユニット |

### テストリスト

ユニットテスト:
- [ ] `soft_delete` で active から deleted に遷移する（status, updated_at, deleted_at を検証）
- [ ] `soft_delete` で uploading ステータスからの遷移を拒否する
- [ ] `soft_delete` で deleted ステータスからの遷移を拒否する

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 2: Repository - 新規クエリメソッド追加

### 確認事項

- 型: `DocumentRepository` トレイト → `backend/crates/infra/src/repository/document_repository.rs`
- パターン: `find_by_id` の行→エンティティ変換（L111-145）、`update_status` の SQL パターン（L186-208）

### 実装

トレイトに 3 メソッド追加:

```rust
/// ドキュメントをソフトデリートする（status, updated_at, deleted_at を更新）
async fn soft_delete(
    &self,
    id: &DocumentId,
    tenant_id: &TenantId,
    now: DateTime<Utc>,
) -> Result<(), InfraError>;

/// フォルダ内の active ドキュメント一覧を取得する
async fn list_by_folder(
    &self,
    folder_id: &FolderId,
    tenant_id: &TenantId,
) -> Result<Vec<Document>, InfraError>;

/// ワークフローインスタンスの active ドキュメント一覧を取得する
async fn list_by_workflow(
    &self,
    workflow_instance_id: &WorkflowInstanceId,
    tenant_id: &TenantId,
) -> Result<Vec<Document>, InfraError>;
```

`find_by_id` の行→エンティティ変換ロジックを `row_to_document` ヘルパーに抽出し、`list_by_*` でも再利用する。

### テストリスト

ユニットテスト:
- [ ] `PostgresDocumentRepository` が Send + Sync を実装している（既存テストでカバー済み）

ハンドラテスト（該当なし - Phase 4 でスタブ経由でテスト）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 3: UseCase - 4 つの新メソッド

### 確認事項

- 型: `DocumentUseCaseImpl` → `backend/apps/core-service/src/usecase/document.rs`
- 型: `WorkflowInstanceRepository` トレイト → `backend/crates/infra/src/repository/workflow_instance_repository.rs`（`find_by_id(id, tenant_id)` シグネチャ）
- 型: `WorkflowInstanceStatus` → `backend/crates/domain/src/workflow/instance.rs`（Draft バリアント）
- パターン: `confirm_upload` メソッド（find → validate → update パターン、L155-204）

### 実装

コンストラクタに `WorkflowInstanceRepository` を追加:

```rust
pub struct DocumentUseCaseImpl {
    document_repository: Arc<dyn DocumentRepository>,
    workflow_instance_repository: Arc<dyn WorkflowInstanceRepository>,
    s3_client: Arc<dyn S3Client>,
    clock: Arc<dyn Clock>,
}
```

入出力型:

```rust
const DOWNLOAD_URL_EXPIRES_IN: Duration = Duration::from_secs(900);

pub struct DownloadUrlOutput {
    pub download_url: String,
    pub expires_in: u64,
}

pub struct SoftDeleteInput {
    pub document_id: DocumentId,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub is_tenant_admin: bool,
}
```

4 メソッド:

- `generate_download_url(document_id, tenant_id)`: find → active チェック → presigned GET URL 生成
- `soft_delete_document(SoftDeleteInput)`: find → 権限チェック → ワークフロー状態チェック → soft_delete → 永続化
- `list_documents(folder_id, tenant_id)`: list_by_folder
- `list_workflow_attachments(workflow_instance_id, tenant_id)`: list_by_workflow

削除権限ロジック:
1. テナント管理者 → OK
2. アップロード者本人 → OK
3. それ以外 → Forbidden
4. ワークフロー添付の場合: ワークフローが Draft でなければ BadRequest

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | active ドキュメントのダウンロード URL を発行する | 正常系 | ハンドラ |
| 2 | 存在しないドキュメントのダウンロード URL を要求する | 準正常系 | ハンドラ |
| 3 | 非 active ドキュメントのダウンロード URL を要求する | 準正常系 | ハンドラ |
| 4 | アップロード者がフォルダ内ドキュメントを削除する | 正常系 | ハンドラ |
| 5 | テナント管理者がドキュメントを削除する | 正常系 | ハンドラ |
| 6 | 権限のないユーザーがドキュメントを削除しようとする | 準正常系 | ハンドラ |
| 7 | 申請済みワークフローの添付ファイルを削除しようとする | 準正常系 | ハンドラ |
| 8 | 下書きワークフローの添付ファイルを削除する | 正常系 | ハンドラ |
| 9 | フォルダ内のドキュメント一覧を取得する | 正常系 | ハンドラ |
| 10 | ワークフロー添付ファイル一覧を取得する | 正常系 | ハンドラ |

### テストリスト

ユニットテスト（該当なし - UseCase のロジックは Phase 4 のハンドラテストでカバー。既存パターンと同じ方針）
ハンドラテスト（Phase 4 で実施）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 4: Core Handler - 4 つの内部 API エンドポイント + テスト

### 確認事項

- パターン: `request_upload_url`, `confirm_upload` ハンドラ（L82-142）
- パターン: テストのスタブパターン（L167-296）
- 型: `DocumentDto` （L61-69）
- ルーティング: `backend/apps/core-service/src/main.rs`（L386-394）

### 実装

新エンドポイント:

| メソッド | パス | ハンドラ関数名 |
|---------|------|--------------|
| POST | `/internal/documents/{document_id}/download-url` | `generate_download_url` |
| DELETE | `/internal/documents/{document_id}` | `delete_document` |
| GET | `/internal/documents` | `list_documents` |
| GET | `/internal/workflows/{workflow_instance_id}/attachments` | `list_workflow_attachments` |

新しい型:

```rust
#[derive(Debug, Deserialize)]
pub struct DeleteDocumentQuery {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub is_tenant_admin: bool,
}

#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    pub tenant_id: Uuid,
    pub folder_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadUrlDto {
    pub download_url: String,
    pub expires_in: u64,
}
```

スタブ拡張:
- `StubDocumentRepository` に `soft_delete`, `list_by_folder`, `list_by_workflow` を追加
- 新規 `StubWorkflowInstanceRepository` を作成（`find_by_id` のみ）
- `DocumentState` に `workflow_instance_repository` フィールド追加不要（UseCase に注入済み）
- `create_test_app` の引数に `StubWorkflowInstanceRepository` を追加

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト:
- [ ] `POST .../download-url` 正常系: 200 で download_url と expires_in が返る
- [ ] `POST .../download-url` 存在しない ID: 404
- [ ] `POST .../download-url` 非 active ドキュメント: 400
- [ ] `DELETE .../documents/{id}` 正常系（アップロード者 = リクエスト者）: 204
- [ ] `DELETE .../documents/{id}` 正常系（テナント管理者）: 204
- [ ] `DELETE .../documents/{id}` 権限なし: 403
- [ ] `DELETE .../documents/{id}` 存在しない ID: 404
- [ ] `DELETE .../documents/{id}` ワークフロー添付 + 下書き: 204
- [ ] `DELETE .../documents/{id}` ワークフロー添付 + 申請済み: 400
- [ ] `GET .../documents?tenant_id=...&folder_id=...` 正常系: 200 でドキュメント配列
- [ ] `GET .../workflows/{id}/attachments?tenant_id=...` 正常系: 200 でドキュメント配列

API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 5: BFF Client + Handler - 4 つの公開 API エンドポイント

### 確認事項

- パターン: `CoreServiceDocumentClient` トレイト → `backend/apps/bff/src/client/core_service/document_client.rs`
- パターン: BFF `document.rs` ハンドラ → `backend/apps/bff/src/handler/document.rs`
- パターン: DELETE クライアントパターン → `backend/apps/bff/src/client/core_service/folder_client.rs`（L107-121: 手動 status チェック）
- パターン: `handle_response` → `backend/apps/bff/src/client/core_service/response.rs`（403 Forbidden 対応済み）
- パターン: `openapi.rs` → `backend/apps/bff/src/openapi.rs`（L83-85）
- ライブラリ: utoipa `#[utoipa::path]`、`IntoParams`（既存使用を参照）
- 型: `SessionData::roles()` → `backend/crates/infra/src/session.rs`（L109: `&[String]`）

### 実装

BFF Client トレイトに 4 メソッド追加:

```rust
async fn generate_download_url(document_id, tenant_id) -> Result<ApiResponse<DownloadUrlCoreDto>, CoreServiceError>;
async fn delete_document(document_id, tenant_id, user_id, is_tenant_admin) -> Result<(), CoreServiceError>;
async fn list_documents(tenant_id, folder_id: Option<Uuid>) -> Result<ApiResponse<Vec<DocumentDetailCoreDto>>, CoreServiceError>;
async fn list_workflow_attachments(workflow_instance_id, tenant_id) -> Result<ApiResponse<Vec<DocumentDetailCoreDto>>, CoreServiceError>;
```

BFF Client 型追加（`types.rs`）:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadUrlCoreDto {
    pub download_url: String,
    pub expires_in: u64,
}
```

BFF Handler レスポンス型追加:

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct DownloadUrlData {
    pub download_url: String,
    pub expires_in: u64,
}

// 一覧取得のクエリパラメータ
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListDocumentsQuery {
    pub folder_id: Option<Uuid>,
}
```

管理者判定（BFF ハンドラ内）:

```rust
let is_tenant_admin = session_data.roles().iter().any(|r| r == "tenant_admin");
```

ルーティング追加（BFF `main.rs`）: 既存の document routes に 4 つ追加。
`openapi.rs`: `paths()` に 4 ハンドラ追加。

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし - BFF ハンドラテストは既存パターンでは未実施）
API テスト（該当なし）
E2E テスト（該当なし）

---

## 変更ファイル一覧

| ファイル | Phase | 変更内容 |
|---------|-------|---------|
| `backend/crates/domain/src/document.rs` | 1 | `soft_delete` メソッド + テスト |
| `backend/crates/infra/src/repository/document_repository.rs` | 2 | 3 メソッド追加 + `row_to_document` ヘルパー抽出 |
| `backend/apps/core-service/src/usecase/document.rs` | 3 | 4 メソッド追加、`WorkflowInstanceRepository` 依存追加 |
| `backend/apps/core-service/src/handler/document.rs` | 4 | 4 ハンドラ + 型 + 11 テスト |
| `backend/apps/core-service/src/main.rs` | 3,4 | コンストラクタ更新 + ルーティング追加 |
| `backend/apps/bff/src/client/core_service/document_client.rs` | 5 | 4 メソッド追加 |
| `backend/apps/bff/src/client/core_service/types.rs` | 5 | `DownloadUrlCoreDto` 追加 |
| `backend/apps/bff/src/handler/document.rs` | 5 | 4 ハンドラ + 型追加 |
| `backend/apps/bff/src/main.rs` | 5 | ルーティング追加 |
| `backend/apps/bff/src/openapi.rs` | 5 | 4 パス追加 |
| `openapi/openapi.yaml` | 5 | `just openapi-generate` で再生成 |
| `docs/40_詳細設計書/17_ドキュメント管理設計.md` | 5 | 実装状態マーカー更新 |

## 検証

```bash
# Phase 1-4 の各 Phase 完了後
cargo test -p ringiflow-domain        # Phase 1
cargo test -p ringiflow-infra         # Phase 2
cargo test -p core-service            # Phase 3-4

# Phase 5 完了後
just openapi-generate
git diff openapi/openapi.yaml         # 新規エンドポイントが反映されていること

# 全体
just check-all
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `update_status` では `deleted_at` をセットできない | 不完全なパス | `soft_delete` 専用メソッドをリポジトリに追加 |
| 1回目 | ワークフロー添付ファイル削除に `WorkflowInstanceRepository` が必要 | 未定義 | `DocumentUseCaseImpl` に依存追加、`main.rs` のコンストラクタ更新 |
| 2回目 | 行→エンティティ変換が `find_by_id` と `list_by_*` で重複する | 重複の排除 | `row_to_document` ヘルパー関数を抽出 |
| 2回目 | BFF での管理者判定方法が未定義 | 曖昧 | `session_data.roles()` で判定、Core に `is_tenant_admin: bool` で渡す |
| 3回目 | `GET /api/v1/workflows/{id}/attachments` のルーティング配置 | 曖昧 | `DocumentState` に配置（ドキュメントデータを返すため） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 4 エンドポイント全て Phase 1-5 でカバー。削除権限チェック含む |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 管理者判定、ルーティング配置、soft_delete vs update_status を明確化 |
| 3 | 設計判断の完結性 | 全差異に判断記載 | OK | 4 つの設計判断を記載 |
| 4 | スコープ境界 | 対象/対象外が明記 | OK | 対象: バックエンド 4 API。対象外: フロントエンド、E2E、API テスト |
| 5 | 技術的前提 | 前提が考慮されている | OK | `generate_presigned_get_url` 既存、`SessionData::roles()` 確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 詳細設計書 API 設計、機能仕様書 4.3/4.4 の削除権限ルールと整合 |
