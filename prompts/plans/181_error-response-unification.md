# #181 ErrorResponse 統一計画

## Context

BFF・Core Service・Auth Service で `ErrorResponse` 構造体が 5 箇所に完全同一の定義で重複している。また `error_type` URI も各所にハードコードされている。`ringiflow-shared` crate に統一し、DRY かつ一貫したエラーレスポンスを実現する。

## 現状の重複（As-Is）

| 場所 | ErrorResponse | error_type URI | ヘルパー関数 |
|------|:---:|:---:|:---:|
| `apps/auth-service/src/error.rs` | 定義あり | 5種ハードコード | — |
| `apps/core-service/src/error.rs` | 定義あり | 6種ハードコード | — |
| `apps/core-service/src/handler/auth.rs` | 定義あり | 4種ハードコード | インライン |
| `apps/bff/src/handler/auth.rs` | 定義あり | 4種ハードコード | 4関数 |
| `apps/bff/src/handler/workflow.rs` | 定義あり | 6種ハードコード | 6関数 |
| `apps/bff/src/middleware/csrf.rs` | `CsrfErrorResponse`（同構造） | 1種ハードコード | — |

さらに `TenantIdError` が BFF の auth.rs と workflow.rs で重複定義。

## 理想状態（To-Be）

- `ErrorResponse` 構造体は `ringiflow-shared` に 1 箇所のみ
- error_type URI は `ErrorResponse` の便利コンストラクタで一元管理
- 各サービスの `IntoResponse` 実装は便利コンストラクタを使って簡潔に
- BFF のヘルパー関数・`TenantIdError` は `bff/src/error.rs` に集約

## 設計判断

### 1. ErrorResponse の配置先: `ringiflow-shared`

**理由**: 全サービスから参照される純粋なデータ構造。依存は `serde` のみで shared の設計方針（最小依存）に合致。既存の `ApiResponse` と対になる。

### 2. axum 依存を shared に追加しない

**理由**: shared の設計方針「外部クレートへの依存は最小限」に従う。`IntoResponse` の変換は各サービスの責務として残す。`ErrorResponse` は純粋なデータ構造に留める。

### 3. error_type URI は便利コンストラクタで管理

`ErrorResponse::not_found(detail)` のように、よく使うパターンをコンストラクタとして提供。URI のベースパスは定数として一元管理。

```rust
const ERROR_TYPE_BASE: &str = "https://ringiflow.example.com/errors";

impl ErrorResponse {
    pub fn not_found(detail: impl Into<String>) -> Self { ... }
    pub fn bad_request(detail: impl Into<String>) -> Self { ... }
    pub fn internal_error() -> Self { ... }
    // ...
}
```

**代替案と却下理由**:
- enum で error_type を定義 → 過度な型安全。サービス固有の error_type（`credential-not-found` 等）まで shared に持つのは責務違反
- const 文字列 → 個別定数は冗長。コンストラクタに内包する方が使いやすい

### 4. サービス固有の error_type

`authentication-failed`, `credential-not-found` 等のサービス固有のエラーは、汎用コンストラクタ `ErrorResponse::new()` で各サービスが自由に作成する。

### 5. BFF ヘルパー関数の集約

BFF の各ハンドラに散らばっていたヘルパー関数と `TenantIdError` を `bff/src/error.rs` に集約する。ヘルパー関数は `ErrorResponse` の便利コンストラクタを使うため大幅に簡潔化される。

## スコープ

### 対象

- `ErrorResponse` 構造体の shared への移動
- error_type URI の一元管理（便利コンストラクタ）
- 各サービスの `IntoResponse` 実装の更新
- BFF ヘルパー関数の `bff/src/error.rs` 集約
- `TenantIdError` の `bff/src/error.rs` 集約
- CSRF middleware の `CsrfErrorResponse` → shared `ErrorResponse` 統一
- Auth Service に `ringiflow-shared` 依存を追加

### 対象外

- `IntoResponse` trait 実装の shared への移動（axum 依存を避ける）
- エラーハンドリングロジック自体の変更（リファクタリングのみ）

## 実装計画

TDD リファクタリング: 既存テストを維持しつつ構造を改善する。

### Phase 1: shared に ErrorResponse を追加

`backend/crates/shared/src/error_response.rs` を新規作成:

- `ErrorResponse` 構造体（`Serialize`, `Deserialize`）
- `ERROR_TYPE_BASE` 定数
- `new()` 汎用コンストラクタ
- 便利コンストラクタ: `not_found`, `bad_request`, `forbidden`, `conflict`, `unauthorized`, `internal_error`, `service_unavailable`, `validation_error`
- `lib.rs` に export 追加

テストリスト:
- [ ] `ErrorResponse::new` で全フィールドが正しく設定される
- [ ] `ErrorResponse::not_found` が 404 + 正しい error_type を返す
- [ ] `ErrorResponse::internal_error` が 500 + 固定 detail を返す
- [ ] JSON シリアライズで `type` フィールド名（serde rename）が正しい

変更ファイル:
- `backend/crates/shared/src/error_response.rs`（新規）
- `backend/crates/shared/src/lib.rs`

### Phase 2: Auth Service の統一

- `Cargo.toml` に `ringiflow-shared` 依存を追加
- `error.rs` の `ErrorResponse` 定義を削除、shared から import
- `IntoResponse for AuthError` を便利コンストラクタで書き換え
- `sqlx prepare` の更新

テストリスト:
- [ ] 既存テスト全 pass（10件）

変更ファイル:
- `backend/apps/auth-service/Cargo.toml`
- `backend/apps/auth-service/src/error.rs`

### Phase 3: Core Service の統一

- `error.rs` の `ErrorResponse` 定義を削除、shared から import
- `handler/auth.rs` の `ErrorResponse` 定義を削除、shared から import
- `IntoResponse for CoreError` を便利コンストラクタで書き換え
- ハンドラのインラインエラーレスポンスを便利コンストラクタに置き換え

テストリスト:
- [ ] 既存テスト全 pass（29件 + ハンドラテスト）

変更ファイル:
- `backend/apps/core-service/src/error.rs`
- `backend/apps/core-service/src/handler/auth.rs`

### Phase 4: BFF の統一

- `error.rs` に `TenantIdError` + `IntoResponse` 実装を集約
- `error.rs` にヘルパー関数を集約（`ErrorResponse` の便利コンストラクタ使用）
- `handler/auth.rs` の `ErrorResponse` 定義・`TenantIdError`・ヘルパー関数を削除
- `handler/workflow.rs` の `ErrorResponse` 定義・`TenantIdError`・ヘルパー関数を削除
- `middleware/csrf.rs` の `CsrfErrorResponse` → shared `ErrorResponse` に置き換え

テストリスト:
- [ ] 既存テスト全 pass（16件 + ハンドラテスト）
- [ ] API テスト全 pass

変更ファイル:
- `backend/apps/bff/src/error.rs`
- `backend/apps/bff/src/handler/auth.rs`
- `backend/apps/bff/src/handler/workflow.rs`
- `backend/apps/bff/src/middleware/csrf.rs`

## 検証

- `just check-all` が通ること（lint + 全テスト + API テスト）
- `grep -r "pub struct ErrorResponse"` でヒットするのが shared の 1 箇所のみ
- `grep -r "pub struct CsrfErrorResponse"` でヒットゼロ

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | ErrorResponse の全定義箇所（5+1）を探索で特定。BFF ヘルパー関数・TenantIdError の重複も含む |
| 2 | 曖昧さ排除 | OK | 変更対象ファイルを Phase ごとに明示。「必要に応じて」等の曖昧表現なし |
| 3 | 設計判断の完結性 | OK | 配置先・axum 依存・URI 管理方法・サービス固有 error_type の扱いについて判断と理由を記載 |
| 4 | スコープ境界 | OK | 対象（構造体統一・URI 一元管理・BFF 集約）と対象外（IntoResponse の shared 移動・ロジック変更）を明記 |
| 5 | 技術的前提 | OK | shared の設計方針（最小依存）、orphan rule（IntoResponse は各サービスに残す）を考慮 |
| 6 | 既存ドキュメント整合 | OK | shared の設計方針コメント、ApiResponse の既存パターンと整合 |
