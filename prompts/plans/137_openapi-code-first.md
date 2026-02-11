# #137 Phase 2: utoipa 導入による OpenAPI Code First 移行

## Context

Issue #137 は API 仕様と実装の整合性担保を目的とする。Phase 1（Playwright E2E テスト）は完了済み。Phase 2 では utoipa を BFF に導入し、Rust の型を Single Source of Truth として OpenAPI 仕様を自動生成する。

現状の課題: openapi/openapi.yaml（1703行）を手動管理しており、BFF の型変更時に同期漏れが発生するリスクがある。

### ブランチ戦略

`feature/403-phase-2` をベースにブランチを作成する。#403 で追加された API エンドポイント（ユーザー CRUD、ロール管理、監査ログ）を含む最新の状態から開始する。PR マージは #403 が main にマージされた後、rebase してから行う。

## スコープ

対象:
- Issue タスク 1: utoipa を BFF に導入し、Rust の型から OpenAPI を自動生成
- Issue タスク 2: CI で生成された OpenAPI と既存の openapi.yaml を比較し、差分があればエラー

対象外:
- Issue タスク 3-4: Elm デコーダー生成（別 Issue として切り出す）
- Swagger UI（utoipa-swagger-ui）の追加
- `/health/ready` エンドポイントの実装（OpenAPI に定義はあるが BFF に未実装）

## 設計判断

### 1. OpenApiRouter vs 手動パス登録

**手動パス登録を採用する。**

BFF の Router は 4 つの異なる State 型（`AuthState`, `WorkflowState`, `UserState`, `RoleState`, `AuditLogState`）を `.with_state()` で分割して使用しており、OpenApiRouter への移行は影響範囲が大きい。`#[derive(OpenApi)]` の `paths()` で全ハンドラを列挙する non-invasive なアプローチを選択する。

### 2. YAML エクスポート方法

**別バイナリ `generate-openapi` を BFF クレートに追加する。** `just sqlx-prepare` / `just sqlx-check` と同じパターン。

### 3. shared クレートの utoipa 依存

**`openapi` feature flag で条件付きコンパイルとする。** shared は全サービスから依存されるが、OpenAPI 生成は BFF のみが必要。

### 4. スキーマ命名

**Rust の型名をそのまま使う（Code First の精神）。** ただし `ErrorResponse` のみ RFC 9457 に合わせて `schema(as = ProblemDetails)` とする。既存 OpenAPI とスキーマ名が変わるが、生成結果で openapi.yaml を置き換えることで解消する。

### 5. テスト戦略

`insta`（workspace に導入済み）でスナップショットテスト。`ApiDoc::openapi()` の JSON 出力全体を検証する。

## エンドポイント一覧（27 個）

| # | グループ | メソッド | パス | ハンドラ | ファイル |
|---|---------|---------|------|---------|---------|
| 1 | health | GET | /health | `health_check` | health.rs |
| 2 | auth | POST | /api/v1/auth/login | `login` | auth.rs |
| 3 | auth | POST | /api/v1/auth/logout | `logout` | auth.rs |
| 4 | auth | GET | /api/v1/auth/me | `me` | auth.rs |
| 5 | auth | GET | /api/v1/auth/csrf | `csrf` | auth.rs |
| 6 | workflows | GET | /api/v1/workflow-definitions | `list_workflow_definitions` | workflow/query.rs |
| 7 | workflows | GET | /api/v1/workflow-definitions/{id} | `get_workflow_definition` | workflow/query.rs |
| 8 | workflows | GET | /api/v1/workflows | `list_my_workflows` | workflow/query.rs |
| 9 | workflows | POST | /api/v1/workflows | `create_workflow` | workflow/command.rs |
| 10 | workflows | GET | /api/v1/workflows/{display_number} | `get_workflow` | workflow/query.rs |
| 11 | workflows | POST | /api/v1/workflows/{display_number}/submit | `submit_workflow` | workflow/command.rs |
| 12 | workflows | POST | /api/v1/workflows/{dn}/steps/{sdn}/approve | `approve_step` | workflow/command.rs |
| 13 | workflows | POST | /api/v1/workflows/{dn}/steps/{sdn}/reject | `reject_step` | workflow/command.rs |
| 14 | tasks | GET | /api/v1/tasks/my | `list_my_tasks` | task.rs |
| 15 | tasks | GET | /api/v1/workflows/{dn}/tasks/{sdn} | `get_task_by_display_numbers` | workflow/query.rs |
| 16 | users | GET | /api/v1/users | `list_users` | user.rs |
| 17 | users | POST | /api/v1/users | `create_user` | user.rs |
| 18 | users | GET | /api/v1/users/{display_number} | `get_user_detail` | user.rs |
| 19 | users | PATCH | /api/v1/users/{display_number} | `update_user` | user.rs |
| 20 | users | PATCH | /api/v1/users/{display_number}/status | `update_user_status` | user.rs |
| 21 | roles | GET | /api/v1/roles | `list_roles` | role.rs |
| 22 | roles | POST | /api/v1/roles | `create_role` | role.rs |
| 23 | roles | GET | /api/v1/roles/{role_id} | `get_role` | role.rs |
| 24 | roles | PATCH | /api/v1/roles/{role_id} | `update_role` | role.rs |
| 25 | roles | DELETE | /api/v1/roles/{role_id} | `delete_role` | role.rs |
| 26 | dashboard | GET | /api/v1/dashboard/stats | `get_dashboard_stats` | dashboard.rs |
| 27 | audit-logs | GET | /api/v1/audit-logs | `list_audit_logs` | audit_log.rs |

## 実装計画

### Phase 1: utoipa 依存追加 + shared クレートの ToSchema 導入

#### 確認事項（事後検証）

注: 実装時に未実施。PR レビュー後にコードと照合して事後検証した。

- [x] ライブラリ: utoipa の ToSchema + Generic 型 → `backend/Cargo.toml:83` に utoipa v5 追加。`ApiResponse<T>` でジェネリクス対応を確認
- [x] ライブラリ: utoipa features → `backend/Cargo.toml:83` で `["chrono", "uuid", "yaml", "preserve_order"]` を確認（計画の `time` は `chrono` に変更）
- [x] 型: `ApiResponse<T>` → `backend/crates/shared/src/api_response.rs:24` に `#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]` を確認
- [x] 型: `ErrorResponse` → `backend/crates/shared/src/error_response.rs:22-23` に ToSchema + `schema(as = ProblemDetails)` を確認
- [ ] 型: `PaginatedResponse<T>` → `backend/crates/shared/src/paginated_response.rs:22` — **ToSchema 未実装**。現時点で BFF のレスポンスに使用されていないため影響なし

#### テストリスト

- [x] `ApiResponse<String>` に ToSchema が実装されていること
- [ ] `PaginatedResponse<String>` に ToSchema が実装されていること — **未実装**
- [x] `ErrorResponse` のスキーマ名が `ProblemDetails` になること
- [x] `openapi` feature 無効時に ToSchema が derive されないこと

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/Cargo.toml` | workspace deps に `utoipa` 追加 |
| `backend/crates/shared/Cargo.toml` | `utoipa` optional 依存、`openapi` feature 追加 |
| `backend/crates/shared/src/api_response.rs` | `#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]` |
| `backend/crates/shared/src/error_response.rs` | 同上 + `schema(as = ProblemDetails)` |
| `backend/crates/shared/src/paginated_response.rs` | `#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]` |

---

### Phase 2: BFF ハンドラ型に ToSchema / IntoParams 追加

#### 確認事項（事後検証）

注: 実装時に未実施。PR レビュー後にコードと照合して事後検証した。

- [ ] 型: 全ハンドラのリクエスト/レスポンス型 → **role.rs と audit_log.rs の型に ToSchema / IntoParams が未実装**。6 ファイル 30 箇所に ToSchema 実装済みだが role.rs, audit_log.rs は 0 件
- [x] ライブラリ: `serde_json::Value` の utoipa での扱い → `handler/workflow.rs` の `form_data: serde_json::Value` で `object` 型にマッピングされることを確認
- [x] ライブラリ: `IntoParams` の使い方 → `handler/user.rs:58-59` の `ListUsersQuery` で `#[into_params(parameter_in = Query)]` を確認

#### テストリスト

- [ ] 全 ToSchema 型がコンパイルできること — **role.rs, audit_log.rs の型が未対応**
- [x] `serde_json::Value` フィールドが `object` 型にマッピングされること
- [x] IntoParams 型がコンパイルできること

#### 変更ファイル

| ファイル | 追加する型 |
|---------|-----------|
| `backend/apps/bff/Cargo.toml` | `utoipa` 依存追加、`ringiflow-shared = { features = ["openapi"] }` |
| handler/health.rs | `HealthResponse`: ToSchema |
| handler/auth.rs | `LoginRequest`, `LoginResponseData`, `LoginUserResponse`, `MeResponseData`, `CsrfResponseData`: ToSchema |
| handler/workflow.rs | リクエスト型: ToSchema / `StepPathParams`: IntoParams |
| handler/workflow/query.rs | `WorkflowData`, `WorkflowStepData`, `WorkflowDefinitionData`, `UserRefData`: ToSchema |
| handler/task.rs | `TaskItemData`, `TaskWorkflowSummaryData`, `TaskDetailData`: ToSchema |
| handler/user.rs | `CreateUserRequest`, `CreateUserResponseData`, `UserDetailData`, `UserItemData`, `UpdateUserRequest`, `UpdateUserStatusRequest`, `UserResponseData`, `ListUsersQuery`: ToSchema/IntoParams |
| handler/role.rs | `CreateRoleRequest`, `RoleDetailData`, `RoleItemData`, `UpdateRoleRequest`: ToSchema |
| handler/dashboard.rs | `DashboardStatsData`: ToSchema |
| handler/audit_log.rs | `AuditLogItemData`, `ListAuditLogsQuery`: ToSchema/IntoParams |

---

### Phase 3: `#[utoipa::path]` アノテーション追加

#### 確認事項（事後検証）

注: 実装時に未実施。PR レビュー後にコードと照合して事後検証した。

- [x] ライブラリ: `#[utoipa::path]` の security / tag / responses 属性 → `handler/auth.rs` 等で `get/post`, `path`, `tag`, `security`, `responses` の使用を確認
- [x] パターン: `impl IntoResponse` 戻り値での responses 定義方法 → `responses()` マクロで明示指定するパターンを確認

#### テストリスト

- [ ] 全 27 ハンドラに `#[utoipa::path]` を付与してコンパイルできること — **20 ハンドラのみ実装済み**（7 ファイル 21 箇所）。role.rs（5）と audit_log.rs（1）が未対応、health の内部エンドポイント（1）はスコープ外として除外

#### 変更ファイル

全 27 ハンドラに `#[utoipa::path]` を追加（エンドポイント一覧参照）。

---

### Phase 4: OpenApi ルート定義 + YAML 生成バイナリ + テスト

#### 確認事項（事後検証）

注: 実装時に未実施。PR レビュー後にコードと照合して事後検証した。

- [x] ライブラリ: `#[derive(OpenApi)]` の paths / components / modifiers / tags → `openapi.rs:14-108` で paths(20), components(schemas 32), tags(6), modifiers(SecurityAddon) を確認
- [x] ライブラリ: `Modify` トレイトで securitySchemes 追加 → `openapi.rs:115-123` で `SecurityAddon` が `session_auth`（Cookie）を追加
- [x] ライブラリ: `OpenApi::to_yaml()` の出力形式 → `bin/generate_openapi.rs:15-17` で YAML 出力を確認

#### テストリスト

- [x] `ApiDoc::openapi()` がパニックせず生成されること
- [ ] 27 パスが含まれること — **18 パス（20 ハンドラ）のみ**。role（5）と audit_log（1）が未登録
- [x] `session_auth` セキュリティスキームが含まれること
- [ ] 全 8 タグが含まれること — **6 タグのみ**（health, auth, workflows, tasks, users, dashboard）。roles と audit-logs が未登録
- [x] `ProblemDetails` スキーマが登録されていること
- [x] スナップショットテスト（insta）: OpenApi JSON 全体

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/bff/src/openapi.rs` | **新規**: `ApiDoc` + `SecurityAddon` |
| `backend/apps/bff/src/lib.rs` | `pub mod openapi;` 追加 |
| `backend/apps/bff/src/bin/generate_openapi.rs` | **新規**: YAML 生成バイナリ |
| `backend/apps/bff/Cargo.toml` | `[[bin]]` セクション追加 |
| `backend/apps/bff/tests/openapi_spec.rs` | **新規**: スナップショットテスト |

---

### Phase 5: justfile コマンド + CI 統合 + openapi.yaml 置換

#### 確認事項（事後検証）

注: 実装時に未実施。PR レビュー後にコードと照合して事後検証した。

- [x] パターン: `just check` の構成 → `justfile:379` で `openapi-check` が `check` レシピに含まれていることを確認
- [x] パターン: CI `rust` ジョブの構成 → `.github/workflows/ci.yaml:125-135` で OpenAPI sync check ステップを確認

#### テストリスト

- [x] `just openapi-generate` が openapi/openapi.yaml を生成すること
- [x] `just openapi-check` が同期状態で正常終了すること
- [x] `just check` に `openapi-check` が含まれていること

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `justfile` | `openapi-generate`, `openapi-check` 追加。`check` に `openapi-check` 追加 |
| `.github/workflows/ci.yaml` | `rust` ジョブに OpenAPI sync check ステップ追加 |
| `openapi/openapi.yaml` | utoipa 生成結果で置換 |

---

## 手動 OpenAPI との既知の差分

| 差分 | 理由 | 対応 |
|------|------|------|
| `/health/ready` がない | BFF に未実装 | 将来の実装 Issue で対応 |
| スキーマ名が Rust 型名 | Code First の方針 | Elm 側は別 Issue で対応 |
| `example` 値がない | 初期スコープ外 | 後続で段階的に追加 |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `/health/ready` が OpenAPI にあるが BFF に未実装 | 不完全なパス | スコープ対象外に明記 |
| 1回目 | スキーマ命名差異 | 曖昧 | 設計判断 4 で方針決定 |
| 2回目 | shared の utoipa 依存が全サービスに波及 | アーキテクチャ不整合 | `openapi` feature flag 導入 |
| 3回目 | #403 の追加エンドポイント（+11個）が計画に含まれていない | 網羅性 | feature/403-phase-2 をベースに計画を更新。27 エンドポイント + 新規型を反映 |
| 3回目 | `PaginatedResponse<T>` が shared に追加されている | 網羅性 | Phase 1 に PaginatedResponse の ToSchema 追加を含めた |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 27 ハンドラ、全型（#403 追加分含む）、shared 3 型、CI、justfile をカバー |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | スキーマ命名、YAML 出力、テスト戦略、ブランチ戦略を明記 |
| 3 | 設計判断の完結性 | 全差異に判断が記載 | OK | 5 つの設計判断に理由を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | Elm 生成、Swagger UI、/health/ready を対象外に明記 |
| 5 | 技術的前提 | 前提が確認済み | OK | utoipa generic 型、feature flag、yaml feature、Modify trait を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 既存 OpenAPI との差分を列挙。ブランチ戦略で #403 との関係を明記 |

## 検証方法

1. `cargo test -p ringiflow-bff` — スナップショットテスト + コンパイルテスト
2. `just openapi-generate` — openapi.yaml 生成
3. `just openapi-check` — 同期確認
4. `just check` — 全体チェック通過
