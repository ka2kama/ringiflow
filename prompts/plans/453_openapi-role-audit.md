# 計画: #453 OpenAPI 仕様に role・audit_log エンドポイントを追加

## Context

PR #444（utoipa 導入）の事後検証で、`role`（5エンドポイント）と `audit_log`（1エンドポイント）の utoipa アノテーションが未実装と判明。計画では27エンドポイントを対象としていたが、実装は20ハンドラ（18パス）のみカバー。既存パターンに従い、不足分を追加する。

## スコープ

対象:
- `handler/role.rs`: 5ハンドラ（list, get, create, update, delete）+ 4型
- `handler/audit_log.rs`: 1ハンドラ（list_audit_logs）+ 2型
- `PaginatedResponse<T>`: `ToSchema` 追加（audit_log のレスポンスで必要）
- `openapi.rs`: paths, schemas, tags 登録
- スナップショットテスト更新
- `openapi/openapi.yaml` 再生成

対象外:
- ハンドラの実装ロジック変更
- 新しいエンドポイントの追加

## Phase 1: role.rs に utoipa アノテーション追加

### 確認事項
- [ ] パターン: user.rs の ToSchema / IntoParams / #[utoipa::path] パターン → `handler/user.rs`
- [ ] パターン: Path パラメータ（Uuid）の utoipa での表現 → workspace utoipa に `uuid` feature あり
- [ ] パターン: openapi.rs のコンポーネント登録パターン → `openapi.rs`

### 変更内容

#### 1.1 型にアノテーション追加（`backend/apps/bff/src/handler/role.rs`）

```rust
// リクエスト型に ToSchema 追加
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest { ... }

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest { ... }

// レスポンス型に ToSchema 追加
#[derive(Debug, Serialize, ToSchema)]
pub struct RoleItemData { ... }

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleDetailData { ... }
```

#### 1.2 ハンドラに `#[utoipa::path]` 追加

| ハンドラ | メソッド | パス | レスポンス | 備考 |
|---------|---------|------|-----------|------|
| `list_roles` | GET | `/api/v1/roles` | 200: `ApiResponse<Vec<RoleItemData>>` | |
| `get_role` | GET | `/api/v1/roles/{role_id}` | 200: `ApiResponse<RoleDetailData>`, 404 | Path: `role_id = Uuid` |
| `create_role` | POST | `/api/v1/roles` | 201: `ApiResponse<RoleDetailData>`, 400, 409 | body: `CreateRoleRequest` |
| `update_role` | PATCH | `/api/v1/roles/{role_id}` | 200: `ApiResponse<RoleDetailData>`, 400, 404 | Path + body |
| `delete_role` | DELETE | `/api/v1/roles/{role_id}` | 204, 404 | Path のみ |

#### 1.3 openapi.rs に登録

- `paths()`: `role::list_roles`, `role::get_role`, `role::create_role`, `role::update_role`, `role::delete_role`
- `components(schemas())`: `role::CreateRoleRequest`, `role::UpdateRoleRequest`, `role::RoleItemData`, `role::RoleDetailData`, `ApiResponse<Vec<role::RoleItemData>>`, `ApiResponse<role::RoleDetailData>`
- `tags()`: `(name = "roles", description = "ロール管理")`
- `use` 文に `role` を追加

#### 1.4 テスト更新（`backend/apps/bff/tests/openapi_spec.rs`）

- パス数: 18 → 20（`/api/v1/roles` + `/api/v1/roles/{role_id}`）
- パスリスト: 2パス追加
- タグリスト: `roles` 追加

### テストリスト
- [ ] `test_全パスが含まれている`: パス数 20、role パス 2 件を含む
- [ ] `test_全タグが含まれている`: `roles` タグを含む
- [ ] コンパイル通過

## Phase 2: audit_log.rs に utoipa アノテーション追加

### 確認事項
- [ ] パターン: PaginatedResponse の ToSchema 追加方法 → `ApiResponse` と同じ `cfg_attr` パターン（`api_response.rs`）
- [ ] パターン: IntoParams の Query パラメータパターン → `handler/user.rs` の `ListUsersQuery`

### 変更内容

#### 2.1 PaginatedResponse に ToSchema 追加（`backend/crates/shared/src/paginated_response.rs`）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PaginatedResponse<T> {
   pub data:        Vec<T>,
   pub next_cursor: Option<String>,
}
```

`ApiResponse<T>` と同じ `cfg_attr(feature = "openapi")` パターンを使用。

#### 2.2 型にアノテーション追加（`backend/apps/bff/src/handler/audit_log.rs`）

```rust
// クエリパラメータ
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListAuditLogsQuery { ... }

// レスポンス型
#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogItemData { ... }
```

#### 2.3 ハンドラに `#[utoipa::path]` 追加

| ハンドラ | メソッド | パス | レスポンス |
|---------|---------|------|-----------|
| `list_audit_logs` | GET | `/api/v1/audit-logs` | 200: `PaginatedResponse<AuditLogItemData>` |

#### 2.4 openapi.rs に登録

- `paths()`: `audit_log::list_audit_logs`
- `components(schemas())`: `audit_log::AuditLogItemData`, `PaginatedResponse<audit_log::AuditLogItemData>`
- `tags()`: `(name = "audit-logs", description = "監査ログ")`
- `use` 文に `audit_log` を追加

#### 2.5 テスト更新

- パス数: 20 → 21（`/api/v1/audit-logs`）
- パスリスト: 1パス追加
- タグリスト: `audit-logs` 追加

### テストリスト
- [ ] `test_全パスが含まれている`: パス数 21、audit-logs パスを含む
- [ ] `test_全タグが含まれている`: `audit-logs` タグを含む
- [ ] コンパイル通過

## Phase 3: 統合検証

### 変更内容

1. スナップショット更新: `cargo insta review` で新しいスナップショットを承認
2. OpenAPI yaml 再生成: `just openapi-generate`
3. 全体検証: `just check-all`

### テストリスト
- [ ] `test_openapi_json全体のスナップショット`: 更新されたスナップショットが正しい
- [ ] `just openapi-check`: utoipa 生成と openapi.yaml が同期
- [ ] `just check-all`: 全チェック通過

## 設計判断

### `PaginatedResponse<T>` への ToSchema 追加

Issue では「任意」とされているが、`audit_log` のレスポンス型 `PaginatedResponse<AuditLogItemData>` を OpenAPI に正しく反映するために必要。`ApiResponse<T>` と同じ `cfg_attr(feature = "openapi")` パターンを採用し、shared クレートの一貫性を維持する。

### role_id のパスパラメータ型

`role_id` は `Uuid` 型。utoipa の workspace 設定で `uuid` feature が有効なため、`params(("role_id" = Uuid, Path, description = "ロールID"))` で OpenAPI 上 `type: string, format: uuid` として正しく表現される。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | PaginatedResponse の ToSchema が Issue で「任意」とされているが audit_log レスポンスに必要 | 不完全なパス | Phase 2 に含め、理由を設計判断に記録 |
| 2回目 | delete_role の 204 No Content レスポンスは body なし。utoipa でどう表現するか | 曖昧 | `(status = 204, description = "削除成功")` で body 指定なし。`auth.rs` の `logout` で同パターン確認済み |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue のチェックリスト全8項目に対応する変更を計画に記載。role 5ハンドラ + audit_log 1ハンドラ + 関連型すべてを列挙 |
| 2 | 曖昧さ排除 | OK | 各ハンドラのパス、メソッド、レスポンス型、ステータスコードを表形式で明示 |
| 3 | 設計判断の完結性 | OK | PaginatedResponse の ToSchema 追加理由、role_id の型選択を記録 |
| 4 | スコープ境界 | OK | 対象（6エンドポイント + 関連型）と対象外（ロジック変更なし）を明記 |
| 5 | 技術的前提 | OK | utoipa の uuid feature 有効、cfg_attr パターン、openapi-generate コマンド確認済み |
| 6 | 既存ドキュメント整合 | OK | api.md ルール（OpenAPI 仕様と実装の同期）に適合。既存パターン（user.rs, auth.rs）を踏襲 |

## 変更ファイル一覧

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/bff/src/handler/role.rs` | ToSchema, IntoParams, #[utoipa::path] 追加 |
| `backend/apps/bff/src/handler/audit_log.rs` | ToSchema, IntoParams, #[utoipa::path] 追加 |
| `backend/crates/shared/src/paginated_response.rs` | ToSchema 追加（cfg_attr） |
| `backend/apps/bff/src/openapi.rs` | paths, schemas, tags 登録 |
| `backend/apps/bff/tests/openapi_spec.rs` | パス数・パスリスト・タグリスト更新 |
| `backend/apps/bff/tests/snapshots/openapi_spec__openapi_spec.snap` | スナップショット更新 |
| `openapi/openapi.yaml` | 再生成 |

## 検証方法

```bash
# Phase 1-2 完了後
cd backend && cargo test --package ringiflow-bff

# Phase 3: スナップショット承認
cd backend && cargo insta review

# OpenAPI yaml 再生成
just openapi-generate

# 全体検証
just check-all
```
