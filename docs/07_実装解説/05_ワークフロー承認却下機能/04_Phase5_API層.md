# Phase 5: API 層（Core Service + BFF）

## 目的

Core Service の内部 API と BFF の公開 API に承認/却下エンドポイントを追加する。

## 変更内容

### 1. Core Service ハンドラ

```rust
// apps/core-service/src/handler/workflow.rs

#[derive(Debug, Deserialize)]
pub struct ApproveRejectRequest {
    pub version:   i32,
    pub comment:   Option<String>,
    pub tenant_id: Uuid,
    pub user_id:   Uuid,
}

pub async fn approve_step<D, I, S>(
    State(state): State<Arc<WorkflowState<D, I, S>>>,
    Path(params): Path<StepPathParams>,
    Json(req): Json<ApproveRejectRequest>,
) -> Result<Response, CoreError>
```

### 2. BFF クライアント

```rust
// apps/bff/src/client/core_service.rs

#[async_trait]
pub trait CoreServiceClient: Send + Sync {
    // ... 既存 ...
    async fn approve_step(&self, workflow_id: Uuid, step_id: Uuid, req: ApproveRejectRequest) -> Result<(), CoreServiceError>;
    async fn reject_step(&self, workflow_id: Uuid, step_id: Uuid, req: ApproveRejectRequest) -> Result<(), CoreServiceError>;
}

pub enum CoreServiceError {
    // ... 既存 ...
    StepNotFound,
    Forbidden(String),
    Conflict(String),
}
```

### 3. BFF ハンドラ

```rust
// apps/bff/src/handler/workflow.rs

pub async fn approve_step<C, S>(
    State(state): State<Arc<WorkflowState<C, S>>>,
    jar: CookieJar,
    Path(params): Path<StepPathParams>,
    Json(req): Json<ApproveRejectRequest>,
) -> impl IntoResponse
```

### 4. ルーティング

| レイヤー | パス | メソッド |
|---------|------|---------|
| Core Service | `/internal/workflows/{id}/steps/{step_id}/approve` | POST |
| Core Service | `/internal/workflows/{id}/steps/{step_id}/reject` | POST |
| BFF | `/api/v1/workflows/{id}/steps/{step_id}/approve` | POST |
| BFF | `/api/v1/workflows/{id}/steps/{step_id}/reject` | POST |

## 設計判断

### なぜ Core Service と BFF で責務を分離するか

```
Browser → BFF → Core Service
```

| レイヤー | 責務 |
|---------|------|
| BFF | セッション管理、Cookie からユーザー情報抽出、CSRF 検証 |
| Core Service | ビジネスロジック実行、楽観的ロック検証、DB 操作 |

BFF はフロントエンドのためのファサード。
セキュリティ境界（認証・認可）を BFF で処理し、Core Service はビジネスロジックに集中する。

### エラーハンドリングの流れ

```
Domain Error → CoreError → HTTP Response
```

| ドメインエラー | CoreError | HTTP Status |
|---------------|-----------|-------------|
| `InvalidStateTransition` | `BadRequest` | 400 |
| 担当者以外 | `Forbidden` | 403 |
| ステップ未発見 | `NotFound` | 404 |
| バージョン不一致 | `Conflict` | 409 |

BFF では Core Service のエラーを RFC 9457 Problem Details 形式でクライアントに返す。

### なぜ RFC 9457 を採用するか

```rust
fn conflict_response(detail: &str) -> Response {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "type": "about:blank",
            "title": "Conflict",
            "status": 409,
            "detail": detail,
        })),
    ).into_response()
}
```

メリット:
1. 標準化されたエラー形式で、クライアント実装が統一できる
2. `detail` フィールドで詳細情報を提供できる
3. `type` フィールドで問題の種類を識別できる（将来拡張）

### リクエストボディの設計

```rust
pub struct ApproveRejectRequest {
    pub version: i32,      // 楽観的ロック用
    pub comment: Option<String>,  // コメントは任意
}
```

- `version` は競合検出のため必須
- `comment` は承認理由や却下理由を記録するための任意フィールド

BFF と Core Service で同名・同構造の DTO を定義。
将来的には共有クレートに移すことも検討できるが、MVP では複雑さを避けるため各サービスで定義。

## テスト

### BFF 単体テスト

既存の `StubCoreServiceClient` に `approve_step` / `reject_step` のスタブを追加。
本 Phase ではユースケース層のテストで十分カバーしているため、API 層のテストは最小限。

```rust
// auth.rs のテスト用スタブ
async fn approve_step(&self, _workflow_id: Uuid, _step_id: Uuid, _req: ApproveRejectRequest) -> Result<(), CoreServiceError> {
    unimplemented!("approve_step is not used in auth tests")
}
```

### 統合テスト

Phase 6（フロントエンド）完了後に E2E テストで検証予定。

## 次のステップ

Phase 6: フロントエンド（承認/却下 UI の実装）
