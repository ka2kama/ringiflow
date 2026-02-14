# 計画: Redocly 警告をゼロにする (#511)

## Context

`just check` 実行時に Redocly lint の警告が 23 件出力されている（`operation-4xx-response`: 8件、`no-unused-components`: 15件）。警告をゼロにし、ルールを `error` に昇格させて再発を防止する。

## スコープ

対象:
- `no-unused-components` 15件の解消
- `operation-4xx-response` 8件の解消（うち `/health` は設計判断あり）
- `redocly.yaml` のルールレベル変更（`warn` → `error`）

対象外:
- jscpd コードクローン（別 Story）
- ファイルサイズ超過（別 Story）
- SQLx キャッシュ（既に解消済み）

## 設計判断

### no-unused-components の解消方針

`openapi.rs` の `components(schemas())` に、standalone 型と `ApiResponse<T>` ラッパーの両方が登録されている。`ApiResponse<T>` 経由で inline 展開されるため standalone 型は不要。

削除対象（15件）:
- `auth::LoginResponseData`, `auth::MeResponseData`, `auth::CsrfResponseData`
- `workflow::WorkflowDefinitionData`, `workflow::WorkflowData`, `workflow::WorkflowCommentData`
- `task::TaskItemData`, `task::TaskDetailData`
- `user::UserItemData`, `user::CreateUserResponseData`, `user::UserDetailData`, `user::UserResponseData`
- `audit_log::AuditLogItemData`
- `role::RoleItemData`, `role::RoleDetailData`
- `dashboard::DashboardStatsData`

注意: `ApiResponse<T>` や `PaginatedResponse<T>` のラッパー、リクエスト型、HealthResponse、ErrorResponse は削除しない（直接参照されるため）。

### operation-4xx-response の解消方針

認証保護された 7 エンドポイント（`security(("session_auth" = []))` あり）:
- 401 Unauthorized を追加（セッション無効時のレスポンス）
- `body = ErrorResponse` で統一（既存パターンに準拠）

`/health` エンドポイント:
- 認証なし、パスパラメータなし、リクエストボディなし → 4xx レスポンスが意味的に存在しない
- OpenAPI 仕様から `/health` を除外する。ヘルスチェックはインフラ・モニタリング用であり、ビジネス API ではない
- `openapi.rs` の `paths()` から `health::health_check` を削除し、`tags()` から `health` も削除

代替案:
1. 意味のない 4xx を追加 → セマンティクスに反する。不採用
2. redocly の lint-ignore で抑制 → 例外を増やすより根本対処。不採用
3. `/health` を OpenAPI から除外（採用）→ ヘルスチェックは API 仕様書の対象外が一般的

### redocly.yaml のルール変更

コメントも実態に合わせて更新:
- `operation-4xx-response: warn` → `error`
- `no-unused-components: warn` → `error`

## Phase 1: no-unused-components の解消

### 確認事項
- なし（既知のパターンのみ）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just openapi-generate` 後に `pnpm exec redocly lint` で `no-unused-components` が 0 件であること

### 変更内容

`backend/apps/bff/src/openapi.rs`:
- `components(schemas())` から未使用 standalone スキーマ 15 件を削除
- `paths()` から `health::health_check` を削除
- `tags()` から health タグを削除
- `use crate::handler::{...}` から `health` を削除

削除する行:
```rust
// 削除: standalone 型（ApiResponse<T> 経由で参照済み）
auth::LoginResponseData,        // L83
auth::MeResponseData,           // L85
auth::CsrfResponseData,         // L86
workflow::WorkflowData,          // L94
workflow::WorkflowDefinitionData, // L95
workflow::WorkflowCommentData,   // L97
task::TaskItemData,              // L102
task::TaskDetailData,            // L103
user::UserItemData,              // L108
user::CreateUserResponseData,    // L109
user::UserDetailData,            // L110
user::UserResponseData,          // L111
audit_log::AuditLogItemData,     // L113
role::RoleItemData,              // L118
role::RoleDetailData,            // L119
dashboard::DashboardStatsData,   // L123
```

残す型（直接参照される）:
- `HealthResponse` → `/health` を除外するので削除対象に変更
- `ErrorResponse` → responses で直接参照
- リクエスト型（`LoginRequest`, `CreateWorkflowRequest` 等）→ request_body で直接参照
- `LoginUserResponse`, `UserRefData`, `WorkflowStepData`, `TaskWorkflowSummaryData` 等 → responses 内で参照
- `ApiResponse<T>` / `PaginatedResponse<T>` → responses の body で直接参照

## Phase 2: operation-4xx-response の解消

### 確認事項
- [x] パターン: 既存の 401 レスポンス定義 → `handler/auth.rs` L277,328,389: `(status = 401, description = "未認証", body = ErrorResponse)`

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just openapi-generate` 後に `pnpm exec redocly lint` で `operation-4xx-response` が 0 件であること

### 変更内容

7 つの GET ハンドラに 401 レスポンスを追加:

| ファイル | 関数 | エンドポイント |
|---------|------|---------------|
| `handler/workflow/query.rs` | `list_workflow_definitions` | GET /api/v1/workflow-definitions |
| `handler/workflow/query.rs` | `list_my_workflows` | GET /api/v1/workflows |
| `handler/task.rs` | `list_my_tasks` | GET /api/v1/tasks/my |
| `handler/user.rs` | `list_users` | GET /api/v1/users |
| `handler/role.rs` | `list_roles` | GET /api/v1/roles |
| `handler/audit_log.rs` | `list_audit_logs` | GET /api/v1/audit-logs |
| `handler/dashboard.rs` | `get_dashboard_stats` | GET /api/v1/dashboard/stats |

追加パターン:
```rust
responses(
   (status = 200, description = "...", body = ...),
   (status = 401, description = "認証エラー", body = ErrorResponse)
)
```

## Phase 3: redocly.yaml の更新

### 確認事項
- なし（既知のパターンのみ）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `pnpm exec redocly lint` が警告・エラーゼロで通過

### 変更内容

`openapi/redocly.yaml`:
```yaml
rules:
  no-server-example.com: off
  operation-4xx-response: error
  no-unused-components: error
```

コメントを更新（`warn` 時の理由コメントを削除し、必要に応じて更新）。

## 検証

1. `just openapi-generate` で OpenAPI 仕様を再生成
2. `pnpm exec redocly lint --config openapi/redocly.yaml` で警告・エラーゼロ
3. `just check` が通過

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `/health` を OpenAPI から除外する場合、`HealthResponse` も削除対象 | 不完全なパス | Phase 1 に `HealthResponse` 削除と `health` import/tag 削除を追加 |
| 2回目 | Issue の未使用コンポーネントが 14 件→実際は 15 件（WorkflowCommentData 追加） | 既存手段の見落とし | 削除対象リストを実際の 15 件に更新 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Redocly 出力の 23 警告すべてに対応策あり（15 unused + 7 GET 4xx + 1 health 除外） |
| 2 | 曖昧さ排除 | OK | 各ファイル・関数を具体的に列挙、「必要に応じて」等の曖昧表現なし |
| 3 | 設計判断の完結性 | OK | `/health` 除外の判断理由と代替案を記載 |
| 4 | スコープ境界 | OK | 対象（Redocly 警告）と対象外（jscpd, ファイルサイズ, SQLx）を明記 |
| 5 | 技術的前提 | OK | utoipa の inline 展開挙動を既存コードで確認済み |
| 6 | 既存ドキュメント整合 | OK | OpenAPI 仕様が Single Source of Truth（Issue 駆動開発 4.2）に準拠 |
