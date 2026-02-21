# Readiness Check エンドポイント実装

## 概要

#736 として、各サービスの依存サービスへの接続状態を確認する Readiness Check エンドポイント (`/health/ready`) を 3 サービス（Core Service、Auth Service、BFF）に実装した。

## 実施内容

### Phase 1: 共通型定義（`ringiflow_shared`）

`CheckStatus`（Ok/Error）、`ReadinessStatus`（Ready/NotReady）、`ReadinessResponse` を `health.rs` に追加。`Serialize` + `Deserialize` + `utoipa::ToSchema` を derive。ユニットテスト（Serialize/Deserialize）と OpenAPI スキーマテストを作成。

### Phase 2: Core Service `/health/ready`

`ReadinessState`（`PgPool`）を保持する State を作成し、`sqlx::query("SELECT 1").execute(pool)` で DB 接続を確認。`tokio::time::timeout(5秒)` でタイムアウトを設定。`.merge()` + `.with_state()` で既存ルーターに追加。

### Phase 3: Auth Service `/health/ready`

Core Service と同一パターン。`pool` が `PostgresCredentialsRepository::new(pool)` で move されるため、その前に `pool.clone()` で ReadinessState 用の参照を確保。

### Phase 4: BFF `/health/ready`

Redis PING と Core Service `/health/ready` を `tokio::join!` で並行チェック。Core の HTTP レスポンスボディ（`ReadinessResponse`）をパースし、`database` キーを BFF のレスポンスにマッピング。Redis は `SessionManager` とは別の `ConnectionManager` を `create_connection_manager` で作成。OpenAPI 仕様にエンドポイントとタグを追加。

### Phase 5: 品質ゲート

`just check-all` で全チェック通過（exit code 0）。Cargo incremental compilation cache の stale 問題に遭遇し、`cargo clean` で解決。

## 判断ログ

- `ReadinessResponse` を `HealthResponse` とは別の型として新設（フィールド構造が異なるため）
- HTTP 503 + ボディパターン: 全チェック OK → 200、1 つでも失敗 → 503。503 でもボディに `ReadinessResponse` を含める
- BFF の Redis 接続を `SessionManager` から分離（`SessionManager` は内部に `ConnectionManager` を持つが公開されていない）
- `reqwest::Client::new()` で BFF 内に新しい HTTP クライアントを作成（Core Service チェック専用）
- Refactor: Core/Auth の `check_database` 関数は同一パターンだが、サービス間で共有するほどの複雑さではないため各サービスに配置

## 成果物

### コミット

```
b4e29a1 #736 WIP: Implement readiness check endpoint
a07f4b9 #736 Add readiness check shared types
67f52b3 #736 Add readiness check endpoint to Core Service
ee6bc05 #736 Add readiness check endpoint to Auth Service
56ffaa0 #736 Add readiness check endpoint to BFF with OpenAPI spec
28ac633 #736 Update OpenAPI spec with readiness check endpoint
bda3925 #736 Fix OpenAPI spec tests and Redocly lint for readiness check
d75ded7 #736 Add Cargo.lock updates and implementation plan
```

### 作成・更新ファイル

- `backend/crates/shared/src/health.rs` — 共通型追加
- `backend/apps/core-service/src/handler/health.rs` — Core readiness check
- `backend/apps/auth-service/src/handler/health.rs` — Auth readiness check
- `backend/apps/bff/src/handler/health.rs` — BFF readiness check
- `backend/apps/bff/src/openapi.rs` — OpenAPI パス・タグ追加
- `openapi/openapi.yaml` — 仕様生成結果
- `openapi/.redocly.lint-ignore.yaml` — `/health/ready` の 4XX レスポンス除外設定

### 関連ドキュメント

- 計画ファイル: [736_readiness-check.md](../../../prompts/plans/736_readiness-check.md)
- 実装解説: [PR737_ReadinessCheck](../../../docs/07_実装解説/PR737_ReadinessCheck/)
- 調査記録: [Cargo incremental cache stale 問題](../../../process/investigations/2026-02/2026-02-20_2115_Cargo-incremental-cache-stale問題.md)
- PR: #737
