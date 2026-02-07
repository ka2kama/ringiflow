# #125 ApiResponse<T> 統一

## 概要

全公開 API の `{ "data": T }` エンベロープを個別定義（24+ structs）から `ApiResponse<T>` ジェネリック型に統一する。

## Phase 1: `ApiResponse<T>` を `ringiflow-shared` に作成

**ファイル:**
- `backend/crates/shared/Cargo.toml` — serde 依存追加
- `backend/crates/shared/src/api_response.rs` — 新規
- `backend/crates/shared/src/lib.rs` — モジュール宣言 + re-export

**実装:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self { Self { data } }
}
```

**テスト:** serialize/deserialize roundtrip, Vec payload, JSON shape `{ "data": ... }` 確認

## Phase 2: Core Service ハンドラを移行

**ファイル:**
- `backend/apps/core-service/Cargo.toml` — ringiflow-shared 依存追加
- `backend/apps/core-service/src/handler/dashboard.rs` — 1 struct 削除
- `backend/apps/core-service/src/handler/task.rs` — 2 structs 削除
- `backend/apps/core-service/src/handler/workflow.rs` — 4 structs 削除

**変換パターン:**
```rust
// Before
let response = WorkflowResponse { data: dto };
// After
let response = ApiResponse::new(dto);
```

## Phase 3+4: BFF client + ハンドラを一括移行

Phase 3 (client) と Phase 4 (handlers) はコンパイルエラー回避のため一括で実施。

**ファイル:**
- `backend/apps/bff/Cargo.toml` — ringiflow-shared 依存追加
- `backend/apps/bff/src/client/core_service.rs` — 7 structs 削除, trait 署名更新
- `backend/apps/bff/src/handler/dashboard.rs` — 1 struct 削除
- `backend/apps/bff/src/handler/task.rs` — 2 structs 削除
- `backend/apps/bff/src/handler/workflow.rs` — 4 structs 削除
- `backend/apps/bff/src/handler/auth.rs` — 3 structs 削除 + テストスタブ更新

**trait 署名変更例:**
```rust
// Before
async fn create_workflow(...) -> Result<WorkflowResponse, CoreServiceError>;
// After
async fn create_workflow(...) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;
```

## 変更しないもの

- `HealthResponse` — `data` wrapper なし
- `GetUserByEmailResponse` — `user` フィールド（`data` ではない）
- `UserWithPermissionsResponse` — 複数フィールド
- `VerifyResponse` / `CreateCredentialsResponse` — 内部 API
- `ErrorResponse` — エラー用エンベロープ
- Elm フロントエンド — JSON 形状は変わらない

## 設計判断

- **型エイリアスは使わない**: `ApiResponse<WorkflowInstanceDto>` を直接使用（KISS）
- **`ApiResponse<T>` は shared クレートに配置**: BFF と Core Service の両方で使うため
- **Serialize + Deserialize 両方 derive**: Core Service は serialize、BFF client は deserialize

## 検証

```bash
just check-all  # lint + 全テスト
```

既存の BFF auth テストが `json["data"]["user"]["email"]` 等で JSON shape を検証しているため、互換性が担保される。
