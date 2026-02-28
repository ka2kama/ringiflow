# #882 ファイルダウンロード・削除 API 実装（Phase 5）

## 概要

Issue #882 の Phase 5（BFF Client + Handler）を実装した。前セッションで Phase 1-4（Domain, Repository, UseCase, Core Service Handler）が完了しており、本セッションで BFF レイヤーの実装を完了させた。

## 実施内容

### Phase 5: BFF Client + Handler

1. `DownloadUrlCoreDto` を `types.rs` に追加（Core Service レスポンスのデシリアライズ用）
2. `CoreServiceDocumentClient` トレイトに 4 メソッドを追加・実装:
   - `generate_download_url`: POST `/internal/documents/{id}/download-url`
   - `delete_document`: DELETE `/internal/documents/{id}`（手動ステータスチェックパターン）
   - `list_documents`: GET `/internal/documents`
   - `list_workflow_attachments`: GET `/internal/workflows/{id}/attachments`
3. BFF ハンドラ 4 関数を追加（utoipa アノテーション付き）:
   - `generate_download_url`, `delete_document`, `list_documents`, `list_workflow_attachments`
4. ルーティング登録（`main.rs`）、OpenAPI パス登録（`openapi.rs`）
5. OpenAPI スナップショットテスト更新（パス数 31 → 35）
6. Core Service の `handler.rs` re-export 更新

### 修正対応

- OpenAPI スナップショット差分: `just openapi-generate` + スナップショットファイル手動更新
- OpenAPI パス数テスト: アサーション値 31 → 35、新パス 4 件のアサーション追加
- Core Service コンパイルエラー: `handler.rs` の re-export に新ハンドラ 4 件を追加
- 計画ファイル命名規則違反: `synthetic-inventing-valley.md` → `882_file-download-delete-api.md` にリネーム

## 判断ログ

- DELETE の BFF Client 実装で `handle_response` ではなく手動ステータスチェックパターンを採用した。理由: 204 No Content はレスポンスボディがなく、`handle_response<T>` が `ApiResponse<T>` のデシリアライズを前提とするため互換性がない。`folder_client.rs` の `delete_folder` と同じパターンに準拠
- `delete_document` の手動ステータスチェックに 403 Forbidden ハンドリングを追加した。理由: Core Service の delete エンドポイントが権限不足時に 403 を返すため、`folder_client` のパターンに Forbidden 分岐を追加
- `/api/v1/workflows/{workflow_instance_id}/attachments` ルートを `document_state` スコープに配置した。理由: レスポンスがドキュメントデータであり、`CoreServiceDocumentClient` を使用するため

## 成果物

### コミット

- `a54835b6` #882 Implement file download URL, soft delete, and document listing APIs

### 変更ファイル（Phase 5 のみ）

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/bff/src/client/core_service/types.rs` | `DownloadUrlCoreDto` 追加 |
| `backend/apps/bff/src/client.rs` | re-export 追加 |
| `backend/apps/bff/src/client/core_service/document_client.rs` | 4 メソッド追加 |
| `backend/apps/bff/src/handler/document.rs` | 4 ハンドラ追加 |
| `backend/apps/bff/src/handler.rs` | re-export 更新 |
| `backend/apps/bff/src/main.rs` | 4 ルート追加 |
| `backend/apps/bff/src/openapi.rs` | 4 パス登録 |
| `backend/apps/bff/tests/openapi_spec.rs` | パス数・アサーション更新 |
| `backend/apps/bff/tests/snapshots/openapi_spec__openapi_spec.snap` | スナップショット更新 |
| `backend/apps/core-service/src/handler.rs` | re-export 更新 |
| `openapi/openapi.yaml` | 再生成 |
