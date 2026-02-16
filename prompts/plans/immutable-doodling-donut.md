# #526 BFF ハンドラ層のクローン削減

## Context

BFF ハンドラ層（7ファイル、2519行、約34クローン）で以下のパターンが繰り返されている:

1. セッション取得（`extract_tenant_id` + `get_session`）: 10行 × 27ハンドラ
2. `CoreServiceError` のエラーマッピング: 4-20行 × 20+箇所
3. import ブロックの重複

3つの施策で削減する:
- `authenticate` ヘルパーで 10行 → 1行
- `IntoResponse for CoreServiceError` でエラー match を `?` に
- `log_and_convert_core_error` でコンテキスト付きログ

## スコープ

対象: `error.rs`, `workflow/command.rs`, `workflow/query.rs`, `user.rs`, `role.rs`, `task.rs`, `dashboard.rs`, `audit_log.rs`

対象外: `auth.rs`（認証フロー固有のセッション操作で `authenticate` パターン不適合。4ハンドラのみで費用対効果が低い）

制約: API 外部仕様（レスポンス構造、ステータスコード）、OpenAPI 仕様は変更しない。

## 設計判断

### 判断1: ハンドラ戻り値を `Result<Response, Response>` に変更

axum の `Result<T, E>` は `T: IntoResponse, E: IntoResponse` で `IntoResponse` を実装。`Result<Response, Response>` を使い `?` を有効化する。カスタムエラー型は KISS 違反のため不採用。

### 判断2: `IntoResponse for CoreServiceError` を `error.rs` に配置

既存のレスポンスヘルパー群と同じ場所に配置し凝集度を高める。`client/core_service/error.rs` は HTTP レスポンスの知識を持つべきでない。core-service の `IntoResponse for CoreError`（`core-service/src/error.rs:41`）と同じパターン。

### 判断3: ログコンテキストは `log_and_convert_core_error` で提供

`IntoResponse::into_response()` にはコンテキスト引数がないため、`Network`/`Unexpected` のログ出力にはヘルパー関数を使う。

### 判断4: 監査ログ付きハンドラ（カテゴリ B）の扱い

セッション取得は `?` 化。Core Service 呼び出しは `Ok` パスで監査ログ記録が必要なため `match` を維持するが、`Err` パスは `log_and_convert_core_error` で簡略化。

## Phase 1: 共通インフラ（`error.rs` 拡張）

### 確認事項
- [ ] 型: `CoreServiceError` 全バリアント → `client/core_service/error.rs`
- [ ] 型: `TenantIdError` の `IntoResponse` 実装パターン → `error.rs` L32-44
- [ ] パターン: core-service の `IntoResponse for CoreError` → `core-service/src/error.rs` L41-71
- [ ] パターン: 既存レスポンスヘルパーの引数 → `error.rs` L84-158

### 実装

`error.rs` に以下を追加:

1. `authenticate(session_manager, headers, jar) -> Result<SessionData, Response>`
   - `extract_tenant_id` + `get_session` を統合

2. `impl IntoResponse for CoreServiceError`
   - `UserNotFound` → `not_found_response("user-not-found", ...)`
   - `WorkflowDefinitionNotFound` → `not_found_response("workflow-definition-not-found", ...)`
   - `WorkflowInstanceNotFound` → `not_found_response("workflow-instance-not-found", ...)`
   - `StepNotFound` → `not_found_response("step-not-found", ...)`
   - `RoleNotFound` → `not_found_response("role-not-found", ...)`
   - `ValidationError(detail)` → `validation_error_response(&detail)`
   - `Forbidden(detail)` → `forbidden_response(&detail)`
   - `EmailAlreadyExists` → `conflict_response("このメールアドレスは既に使用されています")`
   - `Conflict(detail)` → `conflict_response(&detail)`
   - `Network(_) | Unexpected(_)` → `internal_error_response()`

3. `log_and_convert_core_error(context, err) -> Response`
   - `Network`/`Unexpected` → `tracing::error!` + `into_response()`
   - その他 → `into_response()` のみ

### テストリスト

ユニットテスト:
- [ ] `authenticate` 正常系で `SessionData` を返す
- [ ] `authenticate` テナント ID ヘッダーなしで 400
- [ ] `authenticate` テナント ID 不正形式で 400
- [ ] `authenticate` セッション Cookie なしで 401
- [ ] `authenticate` セッション存在しない場合に 401
- [ ] `CoreServiceError::UserNotFound` → 404, error_type `user-not-found`
- [ ] `CoreServiceError::WorkflowDefinitionNotFound` → 404
- [ ] `CoreServiceError::WorkflowInstanceNotFound` → 404
- [ ] `CoreServiceError::StepNotFound` → 404
- [ ] `CoreServiceError::RoleNotFound` → 404
- [ ] `CoreServiceError::ValidationError` → 400
- [ ] `CoreServiceError::Forbidden` → 403
- [ ] `CoreServiceError::EmailAlreadyExists` → 409
- [ ] `CoreServiceError::Conflict` → 409
- [ ] `CoreServiceError::Network` → 500
- [ ] `CoreServiceError::Unexpected` → 500
- [ ] `log_and_convert_core_error` Network → 500
- [ ] `log_and_convert_core_error` UserNotFound → 404（ログなし）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: workflow ハンドラ（command.rs + query.rs）

### 確認事項
- [ ] パターン: Phase 1 完了後の `authenticate`, `log_and_convert_core_error` シグネチャ → `error.rs`

### 実装

13 ハンドラ全てカテゴリ A（監査ログなし）。戻り値を `Result<Response, Response>` に変更し全面的に `?` 化。

Before → After 例（`list_workflow_definitions`）:

```rust
// Before (21行)
pub async fn list_workflow_definitions(...) -> impl IntoResponse {
    let tenant_id = match extract_tenant_id(&headers) { Ok(id) => id, Err(e) => return e.into_response() };
    let session_data = match get_session(...).await { Ok(data) => data, Err(response) => return response };
    match state.core_service_client.list_workflow_definitions(...).await {
        Ok(r) => { ... (StatusCode::OK, Json(response)).into_response() }
        Err(e) => { tracing::error!(...); internal_error_response() }
    }
}

// After (8行)
pub async fn list_workflow_definitions(...) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;
    let core_response = state.core_service_client
        .list_workflow_definitions(*session_data.tenant_id().as_uuid()).await
        .map_err(|e| log_and_convert_core_error("ワークフロー定義一覧取得", e))?;
    let response = ApiResponse::new(core_response.data.into_iter().map(WorkflowDefinitionData::from).collect::<Vec<_>>());
    Ok((StatusCode::OK, Json(response)).into_response())
}
```

display_number バリデーションのあるハンドラは `Err(validation_error_response(...))` で early return。

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just check-all` の通過 + OpenAPI スナップショットテスト

## Phase 3: user.rs / role.rs

### 確認事項
- [ ] パターン: Phase 2 完了後の workflow ハンドラ最終形 → `command.rs`, `query.rs`
- [ ] 型: `AuditLog::new_success` シグネチャ → Grep で確認
- [ ] 型: `AuditLogRepository::record` シグネチャ → Grep で確認

### 実装

カテゴリ A（4ハンドラ: `list_users`, `get_user_detail`, `list_roles`, `get_role`）: Phase 2 と同パターン。

カテゴリ B（6ハンドラ: `create_user`, `update_user`, `update_user_status`, `create_role`, `update_role`, `delete_role`）: セッション取得を `?` 化、Core Service Err パスを `log_and_convert_core_error` で簡略化。

`get_role` の `tenant_id` 直接使用: リファクタ後は `*session_data.tenant_id().as_uuid()` に統一。

`update_user` / `update_user_status` の display_number → UUID 解決:
```rust
// Before (10行 match)
let user_data = match state.core_service_client.get_user_by_display_number(...) { ... };

// After (3行)
let user_data = state.core_service_client
    .get_user_by_display_number(...).await
    .map_err(|e| log_and_convert_core_error("ユーザー取得", e))?.data;
```

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just check-all` の通過

## Phase 4: task.rs / dashboard.rs / audit_log.rs

### 確認事項
確認事項: なし（Phase 2-3 のパターンを踏襲）

### 実装

`list_my_tasks` (task.rs), `get_dashboard_stats` (dashboard.rs): カテゴリ A パターン。
`list_audit_logs` (audit_log.rs): セッション取得のみ `?` 化。Core Service ではなく DB 直接アクセスのため `log_and_convert_core_error` は不使用。DB エラーの `match` は維持。

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just check-all` の通過

## 削減効果

| 施策 | 削減行数 |
|------|---------|
| `authenticate` ヘルパー | 約 200行（10行→1行 × 27箇所） |
| `IntoResponse for CoreServiceError` + `log_and_convert_core_error` | 約 230行 |
| import 整理 | 約 40行 |
| **合計削減** | **約 470行** |

新規追加: 約 195行（ヘルパー実装 + テスト）

純削減: 約 275行（2519行 → 約 2244行）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `auth.rs` のスコープ判断が未記載 | スコープ境界 | 対象外として理由を明示 |
| 2回目 | `list_audit_logs` は `CoreServiceError` ではなく DB エラー | バリエーション漏れ | Phase 4 に特殊ケースを明記 |
| 3回目 | `create_user` の `EmailAlreadyExists` と `Conflict` のメッセージ統一可能 | 既存手段の見落とし | `IntoResponse` impl でカバー |
| 4回目 | `update_user` の display_number 解決呼び出しも `?` 化可能 | シンプルさ | Phase 3 に追記 |
| 5回目 | `get_role` が `tenant_id` を直接使用（`session_data` 経由でない） | 不完全なパス | `*session_data.tenant_id().as_uuid()` に統一 |
| 6回目 | ギャップなし | -- | -- |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 全 7 ファイル 27 ハンドラを分類済み。auth.rs 除外理由を記録 |
| 2 | 曖昧さ排除 | OK | 各ハンドラのカテゴリ（A/B）と適用パターンが確定 |
| 3 | 設計判断の完結性 | OK | 4 つの設計判断に選択肢・理由を記載 |
| 4 | スコープ境界 | OK | 対象（27ハンドラ）・対象外（auth.rs, OpenAPI）を明記 |
| 5 | 技術的前提 | OK | `Result<T,E>: IntoResponse` 確認、orphan rule 制約なし、utoipa 互換 |
| 6 | 既存ドキュメント整合 | OK | リファクタリングのため設計書・ADR への影響なし |

## 検証方法

1. `just check`（各 Phase のコンパイル・テスト確認）
2. `just check-all`（最終的なリント + テスト + API テスト + E2E テスト）
3. OpenAPI スナップショットテスト（`bff/tests/openapi_spec.rs`）が自動で仕様不変を検証
