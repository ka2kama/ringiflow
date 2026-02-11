# utoipa — OpenAPI Code First ライブラリ

## 概要

utoipa は Rust の型定義から OpenAPI 仕様を自動生成する Code First ライブラリ。derive マクロにより、Rust の構造体や関数に属性を付与するだけで OpenAPI 仕様（JSON/YAML）を出力できる。

手動で OpenAPI YAML を管理する Design First アプローチに対し、Code First は実装を Single Source of Truth とすることで仕様と実装の乖離を防ぐ。

## 主な機能

### ToSchema — スキーマ定義

構造体を OpenAPI のスキーマコンポーネントに変換する。

```rust
#[derive(ToSchema)]
struct UserData {
    id: Uuid,
    name: String,
}
```

スキーマ名のカスタマイズ（Rust 型名と異なる名前で公開したい場合）:

```rust
#[derive(ToSchema)]
#[schema(as = ProblemDetails)]  // OpenAPI 上は "ProblemDetails" として登録
struct ErrorResponse {
    r#type: String,
    title: String,
}
```

ジェネリック型の ToSchema:

```rust
#[derive(ToSchema)]
struct ApiResponse<T> {
    data: T,  // T: ToSchema が自動的に要求される
}

// 使用時は具体型を指定
components(schemas(
    ApiResponse<UserData>,  // → "ApiResponse_UserData" として登録
))
```

### IntoParams — パラメータ定義

Path/Query パラメータを定義する。

```rust
#[derive(IntoParams)]
#[into_params(parameter_in = Path)]
struct StepPathParams {
    display_number: i32,
    step_display_number: i32,
}

#[derive(IntoParams)]
#[into_params(parameter_in = Query)]
struct ListUsersQuery {
    status: Option<String>,
}
```

### `#[utoipa::path]` — エンドポイント定義

ハンドラ関数に OpenAPI のパス情報を付与する。

```rust
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "users",
    params(ListUsersQuery),
    security(("session_auth" = [])),
    responses(
        (status = 200, description = "ユーザー一覧", body = ApiResponse<Vec<UserItemData>>),
        (status = 401, description = "未認証", body = ErrorResponse),
    )
)]
async fn list_users(...) -> impl IntoResponse { ... }
```

注意: `body =` にはスキーマ名（`schema(as = ...)` で指定した名前）ではなく、Rust の型名を指定する。

### `#[derive(OpenApi)]` — ルート定義

全パス・コンポーネント・タグを集約する。

```rust
#[derive(OpenApi)]
#[openapi(
    paths(list_users, create_user),
    components(schemas(UserData, ApiResponse<UserData>)),
    tags((name = "users", description = "ユーザー管理")),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;
```

### Modify トレイト — カスタム変更

生成された OpenAPI にプログラムで変更を加える。セキュリティスキームの追加などに使用。

```rust
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_default();
        components.add_security_scheme(
            "session_auth",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("session_id"))),
        );
    }
}
```

## Feature Flags

| Feature | 用途 |
|---------|------|
| `chrono` | `chrono::NaiveDateTime` 等の ToSchema サポート |
| `uuid` | `uuid::Uuid` の ToSchema サポート |
| `time` | `time` クレートの型のサポート |
| `yaml` | `OpenApi::to_yaml()` メソッドの有効化 |
| `preserve_order` | スキーマのフィールド順序を定義順に保持 |

## プロジェクトでの使用箇所

### shared クレート（条件付き）

`openapi` feature flag で条件付きコンパイル。BFF のみが有効化する。

- `backend/crates/shared/src/api_response.rs` — `ApiResponse<T>`
- `backend/crates/shared/src/error_response.rs` — `ErrorResponse`（`as = ProblemDetails`）

### BFF

- `backend/apps/bff/src/handler/*.rs` — ToSchema / IntoParams / utoipa::path
- `backend/apps/bff/src/openapi.rs` — `ApiDoc` ルート定義
- `backend/apps/bff/src/bin/generate_openapi.rs` — YAML 生成バイナリ
- `backend/apps/bff/tests/openapi_spec.rs` — スナップショットテスト

### 開発フロー

```bash
just openapi-generate  # openapi/openapi.yaml を utoipa から再生成
just openapi-check     # 生成結果と openapi.yaml の同期を確認
```

CI の `rust` ジョブで自動的に同期チェックを実行する。

## 関連リソース

- [utoipa 公式ドキュメント](https://docs.rs/utoipa/latest/utoipa/)
- [utoipa GitHub](https://github.com/juhaku/utoipa)
- ADR: 計画ファイル `prompts/plans/137_openapi-code-first.md` に設計判断を記録
