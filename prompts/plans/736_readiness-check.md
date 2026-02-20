# #736 Readiness Check エンドポイント (`/health/ready`) の実装計画

## Context

外部 API 連携の増加に備え、各サービスの依存サービスへの接続状態を確認する Readiness Check エンドポイントを実装する。現在の `/health`（Liveness）は常に `"healthy"` を返すのみで、依存サービスの状態を検知できない。

既存の API テスト `tests/api/hurl/health_ready.hurl` がレスポンス形状を定義済み。

## 対象

- BFF: `/health/ready`（Redis, Core Service 接続, Core 経由の DB 確認）
- Core Service: `/health/ready`（PostgreSQL 接続確認）
- Auth Service: `/health/ready`（PostgreSQL 接続確認）
- 共有型: `ReadinessResponse`, `CheckStatus`, `ReadinessStatus`
- OpenAPI 仕様に `/health/ready` を追加

## 対象外

- Auth Service の BFF からのチェック（API テストが期待するキーに含まれない）
- DynamoDB チェック（監査ログは主要機能ではなく、一時障害で全体を not_ready にすべきでない）
- フロントエンドの変更（`/health/ready` はインフラ向けエンドポイント）
- Lightsail 環境の healthcheck 改善（別 Issue で対応）

## 設計判断

### 1. レスポンス型: `ReadinessResponse` を新設する

`HealthResponse` とはフィールド構造が異なる（`version` なし、`checks` あり）ため別の型とする。

### 2. 型の活用: enum `CheckStatus` / `ReadinessStatus`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus { Ok, Error }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStatus { Ready, NotReady }

pub struct ReadinessResponse {
    pub status: ReadinessStatus,
    pub checks: HashMap<String, CheckStatus>,
}
```

`Deserialize` は BFF が Core のレスポンスをパースするために必要。

### 3. checks の key 設計

| サービス | checks keys | チェック対象 |
|----------|------------|------------|
| BFF | `redis`, `core_api`, `database` | Redis PING, Core GET /health/ready, Core の DB 結果をマッピング |
| Core | `database` | PostgreSQL `SELECT 1` |
| Auth | `database` | PostgreSQL `SELECT 1` |

### 4. 部分的失敗時: HTTP 503 + レスポンスボディ

全チェック OK → 200、1 つでも失敗 → 503。503 でもボディに `ReadinessResponse` を含める。

### 5. State 設計: 各サービスに `ReadinessState` を定義

BFF は Redis/Core URL/HTTP クライアント、Core/Auth は PgPool を保持。`.merge()` で既存ルーターに追加。

### 6. タイムアウト: 各チェック 5 秒

`tokio::time::timeout` で各チェックを wrap。ハングによるブロックを防止。

---

## Phase 1: 共通型定義（`ringiflow_shared`）

### 対象ファイル
- `backend/crates/shared/src/health.rs` — 型追加
- `backend/crates/shared/src/lib.rs` — re-export 追加

### 作業内容
1. `CheckStatus` enum を追加（`Serialize`, `Deserialize`, `Clone`, `PartialEq`, `ToSchema`）
2. `ReadinessStatus` enum を追加（同上）
3. `ReadinessResponse` 構造体を追加（`status: ReadinessStatus`, `checks: HashMap<String, CheckStatus>`）
4. `lib.rs` に `CheckStatus`, `ReadinessStatus`, `ReadinessResponse` の re-export を追加

### 確認事項
- [ ] 型: `HealthResponse` の既存定義 → `backend/crates/shared/src/health.rs`
- [ ] パターン: `#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]` → `HealthResponse`, `ApiResponse`, `ErrorResponse` で使用
- [ ] ライブラリ: `serde(rename_all = "lowercase")` → Grep で既存パターン確認
- [ ] パターン: `lib.rs` の re-export 形式 → `pub use health::HealthResponse;`

### テストリスト

ユニットテスト:
- [ ] `ReadinessResponse` を status=Ready, checks={database: Ok} で Serialize すると `{"status":"ready","checks":{"database":"ok"}}` になること
- [ ] `ReadinessResponse` を status=NotReady, checks={database: Error} で Serialize すると `{"status":"not_ready","checks":{"database":"error"}}` になること
- [ ] `CheckStatus::Ok` / `CheckStatus::Error` の Serialize 結果
- [ ] `ReadinessStatus::Ready` / `ReadinessStatus::NotReady` の Serialize 結果

OpenAPI テスト (`#[cfg(all(test, feature = "openapi"))]`):
- [ ] `ReadinessResponse` に ToSchema が実装されていること

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 2: Core Service `/health/ready` 実装

### 対象ファイル
- `backend/apps/core-service/src/handler/health.rs` — `ReadinessState` + `readiness_check` ハンドラ追加
- `backend/apps/core-service/src/handler.rs` — re-export 追加
- `backend/apps/core-service/src/main.rs` — ルート追加 + State 作成

### 作業内容

1. `handler/health.rs` に追加:

```rust
pub struct ReadinessState {
    pub pool: PgPool,
}

pub async fn readiness_check(
    State(state): State<Arc<ReadinessState>>,
) -> impl IntoResponse {
    let db_check = check_database(&state.pool).await;
    let mut checks = HashMap::new();
    checks.insert("database".to_string(), db_check);
    let all_ok = checks.values().all(|s| matches!(s, CheckStatus::Ok));
    let status = if all_ok { ReadinessStatus::Ready } else { ReadinessStatus::NotReady };
    let http_status = if all_ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (http_status, Json(ReadinessResponse { status, checks }))
}

async fn check_database(pool: &PgPool) -> CheckStatus {
    match tokio::time::timeout(
        Duration::from_secs(5),
        sqlx::query("SELECT 1").execute(pool),
    ).await {
        Ok(Ok(_)) => CheckStatus::Ok,
        _ => CheckStatus::Error,
    }
}
```

2. `handler.rs` に re-export: `pub use health::{ReadinessState, readiness_check};`

3. `main.rs` ルーター構築（L258 の `/health` の直後に `.merge()`）:
```rust
let readiness_state = Arc::new(ReadinessState { pool: pool.clone() });

let app = Router::new()
    .route("/health", get(health_check))
    .merge(
        Router::new()
            .route("/health/ready", get(readiness_check))
            .with_state(readiness_state)
    )
    .route("/internal/users", ...)
    // ... 既存ルート
```

### 確認事項
- [ ] 型: `PgPool` の Clone → 既存リポジトリ初期化で `pool.clone()` が多数使用
- [ ] パターン: `(StatusCode, Json(...))` の返し方 → Core Service の既存ハンドラ
- [ ] ライブラリ: `sqlx::query("SELECT 1").execute(pool)` → docs.rs で確認（`fetch_one` より execute が軽量）
- [ ] ライブラリ: `tokio::time::timeout` → tokio features = ["full"] で利用可能（確認済み）

### テストリスト

ユニットテスト（該当なし — DB 接続が必要なため）

ハンドラテスト（該当なし — `PgPool` のモックが困難。API テストで検証）

API テスト（該当なし — BFF 経由の API テストで間接検証）

E2E テスト（該当なし）

---

## Phase 3: Auth Service `/health/ready` 実装

### 対象ファイル
- `backend/apps/auth-service/src/handler/health.rs` — Phase 2 と同一パターン
- `backend/apps/auth-service/src/handler.rs` — re-export 追加
- `backend/apps/auth-service/src/main.rs` — ルート追加 + State 作成

### 作業内容

Phase 2 と同一パターン。注意点:

- `main.rs` L110 で `pool` が `PostgresCredentialsRepository::new(pool)` に move される
- `pool.clone()` を **L110 の前**に実行して ReadinessState 用の参照を確保する必要がある

```rust
// L110 の前に追加
let readiness_state = Arc::new(ReadinessState { pool: pool.clone() });

// 既存: pool が move される
let credentials_repo: Arc<dyn CredentialsRepository> =
    Arc::new(PostgresCredentialsRepository::new(pool));
```

ルーター（L118 の `/health` の直後に `.merge()`）:
```rust
let app = Router::new()
    .route("/health", get(health_check))
    .merge(
        Router::new()
            .route("/health/ready", get(readiness_check))
            .with_state(readiness_state)
    )
    .route("/internal/auth/verify", post(verify))
    // ... 既存ルート
```

### 確認事項
- [ ] 型: Auth Service の `pool` 変数の所有権の流れ → `main.rs` L97-110（clone が必要）
- [ ] パターン: Phase 2 との差異は `pool` の clone タイミングのみ

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし — BFF 経由で間接検証）

E2E テスト（該当なし）

---

## Phase 4: BFF `/health/ready` 実装 + OpenAPI 更新

### 対象ファイル
- `backend/apps/bff/src/handler/health.rs` — `ReadinessState` + `readiness_check` ハンドラ
- `backend/apps/bff/src/handler.rs` — re-export 追加
- `backend/apps/bff/src/main.rs` — ルート追加 + State 作成
- `backend/apps/bff/src/openapi.rs` — パス + タグ追加

### 作業内容

1. `handler/health.rs` に追加:

```rust
pub struct ReadinessState {
    pub redis_conn: ConnectionManager,
    pub core_service_url: String,
    pub http_client: reqwest::Client,
}

#[utoipa::path(
   get,
   path = "/health/ready",
   tag = "health",
   responses(
      (status = 200, description = "全依存サービス稼働中", body = ReadinessResponse),
      (status = 503, description = "一部の依存サービスが利用不可", body = ReadinessResponse)
   )
)]
pub async fn readiness_check(
    State(state): State<Arc<ReadinessState>>,
) -> impl IntoResponse {
    // Redis と Core Service を並行チェック
    let (redis_result, core_result) = tokio::join!(
        check_redis(state.redis_conn.clone()),
        check_core_service(&state.http_client, &state.core_service_url),
    );
    let mut checks = HashMap::new();
    checks.insert("redis".to_string(), redis_result);
    checks.insert("core_api".to_string(), core_result.core_api);
    checks.insert("database".to_string(), core_result.database);
    // ...（Phase 2 と同じパターンで status / http_status 決定）
}
```

2. Redis チェック:
```rust
async fn check_redis(mut conn: ConnectionManager) -> CheckStatus {
    match tokio::time::timeout(
        Duration::from_secs(5),
        redis::cmd("PING").query_async::<String>(&mut conn),
    ).await {
        Ok(Ok(_)) => CheckStatus::Ok,
        _ => CheckStatus::Error,
    }
}
```

3. Core Service チェック（GET `/health/ready` を呼び、結果をマッピング）:
```rust
struct CoreCheckResult {
    core_api: CheckStatus,
    database: CheckStatus,
}

async fn check_core_service(client: &reqwest::Client, base_url: &str) -> CoreCheckResult {
    let url = format!("{base_url}/health/ready");
    match tokio::time::timeout(Duration::from_secs(5), client.get(&url).send()).await {
        Ok(Ok(response)) => {
            // 503 でもボディをパースする（Core は 503 + ReadinessResponse を返す）
            match response.json::<ReadinessResponse>().await {
                Ok(body) => CoreCheckResult {
                    core_api: CheckStatus::Ok,
                    database: body.checks.get("database").cloned().unwrap_or(CheckStatus::Error),
                },
                Err(_) => CoreCheckResult { core_api: CheckStatus::Error, database: CheckStatus::Error },
            }
        }
        _ => CoreCheckResult { core_api: CheckStatus::Error, database: CheckStatus::Error },
    }
}
```

4. `main.rs` での State 構築（L159 の Redis 初期化の後に追加）:
```rust
// Readiness Check 用の Redis 接続（SessionManager とは別の接続）
let readiness_redis_conn = redis::create_connection_manager(&config.redis_url)
    .await
    .expect("Redis への接続に失敗しました（readiness check 用）");

let readiness_state = Arc::new(ReadinessState {
    redis_conn: readiness_redis_conn,
    core_service_url: config.core_url.clone(),
    http_client: reqwest::Client::new(),
});
```

ルーター（L292 の `/health` の直後に `.merge()`）:
```rust
let app = Router::new()
    .route("/health", get(health_check))
    .merge(
        Router::new()
            .route("/health/ready", get(readiness_check))
            .with_state(readiness_state)
    )
    .route("/api/v1/auth/login", post(login))
    // ... 既存ルート
```

5. `openapi.rs` に追加:
- `paths(...)` に `crate::handler::health::readiness_check,` を追加
- `tags(...)` に `(name = "health", description = "ヘルスチェック"),` を追加

6. `handler.rs` L30 の re-export 更新:
```rust
pub use health::{ReadinessState, health_check, readiness_check};
```

### 確認事項
- [ ] 型: `redis::aio::ConnectionManager` の Clone → `session.rs` で `self.conn.clone()` パターン確認済み
- [ ] パターン: BFF の `.merge()` → L348-425 で多数使用
- [ ] ライブラリ: `redis::cmd("PING").query_async()` → Grep 既存使用 or docs.rs
- [ ] ライブラリ: `reqwest::Client::get().send()` → `client/core_service.rs` に既存パターン
- [ ] パターン: `ringiflow_infra::redis::create_connection_manager` → `redis.rs` L86-91
- [ ] パターン: `openapi.rs` のパス/タグ追加形式 → 既存パターン確認済み

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし — Redis + HTTP クライアントのモック困難。API テストで検証）

API テスト:
- [ ] 既存 `health_ready.hurl`: 200 + `status == "ready"` + `checks.database == "ok"` + `checks.redis == "ok"` + `checks.core_api == "ok"`

E2E テスト（該当なし）

---

## Phase 5: OpenAPI 仕様生成 + 品質ゲート

### 作業内容
1. `just openapi-generate` で `openapi.yaml` を更新
2. `git diff openapi/openapi.yaml` で `/health/ready` エンドポイントが含まれることを確認
3. `just check-all` で全テスト通過を確認

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト:
- [ ] `just check-all` が通過すること（`health_ready.hurl` 含む）

E2E テスト（該当なし）

---

## 検証方法

1. `just check` で各 Phase のコンパイル確認
2. `cd backend && cargo test` でユニットテスト（Phase 1）
3. `just check-all` で API テスト含む全テスト（Phase 5）
4. `just openapi-generate && git diff openapi/openapi.yaml` で OpenAPI 確認

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|--------|----------------|------|------|
| 1回目 | `ReadinessResponse` に `Deserialize` が必要（BFF → Core パース） | 不完全なパス | Phase 1 で `Serialize` + `Deserialize` 両方を derive |
| 2回目 | `RedisSessionManager` から `ConnectionManager` を取り出せない | 既存手段の見落とし | `ringiflow_infra::redis::create_connection_manager` で別接続を作成 |
| 3回目 | Auth Service の `pool` が L110 で move される | 不完全なパス | L110 の前に `pool.clone()` を追加する手順を明示 |
| 4回目 | Core 503 レスポンスでもボディをパースする必要がある | 曖昧 | `response.json()` は HTTP ステータスに関係なくボディをパースする旨を明示 |
| 5回目 | OpenAPI に `health` タグが未登録 | 未定義 | `openapi.rs` の `tags()` に追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|-------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 3サービス + 共通型 + OpenAPI + API テスト。除外は理由付き |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase にコードスニペットで実装が一意に確定 |
| 3 | 設計判断の完結性 | 全差異に判断が記載 | OK | 型設計、HTTP ステータス、タイムアウト、チェック対象、State 設計の 6 判断 |
| 4 | スコープ境界 | 対象・対象外が明記 | OK | 対象外セクションに Auth/DynamoDB/フロントエンド/Lightsail を明記 |
| 5 | 技術的前提 | 非コード前提が考慮 | OK | tokio time feature、PgPool Clone、ConnectionManager Clone、reqwest workspace dep |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 既存 API テストの期待値と一致。HealthResponse に変更なし |
