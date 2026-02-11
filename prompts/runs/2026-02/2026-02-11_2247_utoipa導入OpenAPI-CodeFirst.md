# utoipa 導入による OpenAPI Code First 移行

## 概要

Issue #137 Phase 2 として、utoipa を BFF に導入し、Rust の型を Single Source of Truth として OpenAPI 仕様を自動生成する仕組みを構築した。5 Phase に分けて段階的に実装した。

## 実施内容

### Phase 1: utoipa 依存追加 + shared クレートの ToSchema 導入

- ワークスペースに utoipa v5 を追加（features: chrono, uuid, yaml, preserve_order）
- shared クレートに `openapi` feature flag を追加し、`ApiResponse<T>` と `ErrorResponse` に条件付き `ToSchema` derive を追加
- `ErrorResponse` には `schema(as = ProblemDetails)` で RFC 9457 準拠のスキーマ名を設定

### Phase 2: BFF ハンドラ型に ToSchema / IntoParams 追加

- 全 24 型に `ToSchema` derive を追加
- `StepPathParams`（Path）と `ListUsersQuery`（Query）に `IntoParams` derive を追加

### Phase 3: `#[utoipa::path]` アノテーション追加

- 全 20 ハンドラに `#[utoipa::path]` アノテーションを追加
- tag, security, responses（200/400/401/404/409/500）を設定

### Phase 4: OpenApi ルート定義 + YAML 生成バイナリ + テスト

- `ApiDoc` 構造体に `#[derive(OpenApi)]` で全パス・スキーマ・タグを集約
- `SecurityAddon` で Cookie ベースの `session_auth` セキュリティスキームを追加
- `generate-openapi` バイナリで YAML を標準出力に出力
- insta によるスナップショットテスト 6 件を追加

### Phase 5: justfile コマンド + CI 統合 + openapi.yaml 置換

- `just openapi-generate` / `just openapi-check` コマンドを追加
- `just check` に `openapi-check` を含めた
- CI の `rust` ジョブに OpenAPI 同期チェックステップを追加
- `openapi/openapi.yaml` を utoipa 生成結果で置換

## 判断ログ

- `ErrorResponse` の `body =` 参照に Rust 型名を使用: `schema(as = ProblemDetails)` はスキーマ名の変更のみで、utoipa マクロ内の `body =` 参照は Rust の型名（`ErrorResponse`）を使う必要がある
- パス数 18（ハンドラ数 20）: 同一パスに複数メソッド（GET/POST）がある場合、パスとしては 1 つにまとまる
- `openapi-check` の stderr 抑制: `cargo run` のコンパイルメッセージが stdout に混入しないよう `2>/dev/null` を使用
- CI では `rust` ジョブに同期チェックを配置: Cargo ビルドが必要なため、既存の `openapi` ジョブ（Redocly lint のみ）ではなく `rust` ジョブに追加

## 成果物

### コミット

- `b9ac97c` #137 Add utoipa dependency and ToSchema derives to shared crate
- `2abc58e` #137 Add ToSchema and IntoParams derives to BFF handler types
- `0dc8e48` #137 Add #[utoipa::path] annotations to all 16 BFF handlers
- `dcb01ef` #137 Add OpenApi root definition, YAML generator binary, and snapshot tests
- `3a11dd1` #137 Add justfile commands, CI sync check, and replace openapi.yaml with utoipa output

### 作成・更新ファイル

- `backend/crates/shared/Cargo.toml` — openapi feature 追加
- `backend/crates/shared/src/api_response.rs` — ToSchema derive + テスト
- `backend/crates/shared/src/error_response.rs` — ToSchema + ProblemDetails alias + テスト
- `backend/apps/bff/Cargo.toml` — utoipa 依存、generate-openapi バイナリ、insta
- `backend/apps/bff/src/handler/*.rs` — ToSchema / IntoParams / utoipa::path
- `backend/apps/bff/src/openapi.rs` — ApiDoc + SecurityAddon（新規）
- `backend/apps/bff/src/bin/generate_openapi.rs` — YAML 生成バイナリ（新規）
- `backend/apps/bff/tests/openapi_spec.rs` — スナップショットテスト（新規）
- `justfile` — openapi-generate / openapi-check / check 更新
- `.github/workflows/ci.yaml` — rust ジョブに同期チェック追加
- `openapi/openapi.yaml` — utoipa 生成結果で置換
