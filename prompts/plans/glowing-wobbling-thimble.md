# Issue #650: Request ID の生成・伝播とログへの自動注入

## コンテキスト

Epic #648（Observability 基盤）の Story 2。#649（ログ共通化 + JSON 対応）完了後の次のステップとして、マルチサービス間のリクエスト追跡基盤を構築する。

現状（As-Is）:
- 3 サービスすべてで `TraceLayer::new_for_http()` がデフォルト設定
- BFF の reqwest クライアントにヘッダー伝播メカニズムなし
- サービス横断のリクエスト追跡が不可能

目標（To-Be）:
- 1 つのユーザーリクエストに対する全サービスのログが同一の `request_id` で検索可能
- レスポンスヘッダーの `X-Request-Id` でリクエストを特定可能

## 設計判断

### 判断 1: tower-http の `request-id` モジュールを活用する

tower-http 0.6 が提供する `SetRequestIdLayer`（生成）+ `PropagateRequestIdLayer`（レスポンスへの伝播）を使用する。独自ミドルウェアの実装を避け、フレームワークの機能を活用する（KISS）。

UUID v7 が必要（時系列ソート可能）だが、組み込みの `MakeRequestUuid` は UUID v4 を使用するため、`MakeRequestId` trait の独自実装（5 行程度）が必要。

### 判断 2: `TraceLayer` の `make_span_with` をカスタマイズする

`DefaultMakeSpan` は `request_id` フィールドを含まない。カスタム `make_span_with` で `X-Request-Id` ヘッダーからリクエスト ID を読み取り、スパンに `request_id` フィールドとして記録する。JSON ログの `.with_current_span(true)` により自動的にログ出力に含まれる。

3 サービス共通のため `ringiflow-shared` の `observability` モジュールに配置する。

### 判断 3: `tokio::task_local!` で BFF → 内部サービスへの Request ID 伝播

BFF のハンドラーからクライアントメソッドに Request ID を渡す方法:

| 選択肢 | 評価 |
|-------|------|
| A: 全メソッド引数に `request_id` を追加 | 型安全だが 34 箇所のメソッドシグネチャ変更が必要。侵襲的 |
| B: `tokio::task_local!` で保存・取得 | ハンドラーのシグネチャ変更なし。横断的関心事に適切 |
| C: `reqwest-middleware` クレート | 新しい依存追加。現段階では過剰 |

**選択: B** — ミドルウェアで `SetRequestIdLayer` が設定した `RequestId` を task-local に保存し、クライアントコードで取得してヘッダーに注入する。自由関数 `inject_request_id(builder)` を提供し、Core/Auth 両クライアントで共用する。

### 判断 4: レイヤー順序

tower のレイヤーは後に追加されたものが外側（リクエスト最初、レスポンス最後）。

```
SetRequestIdLayer (最外)   ← リクエスト: ID 生成 + ヘッダー設定
  └→ TraceLayer            ← リクエスト: カスタムスパン作成（request_id 含む）
       └→ PropagateRequestId ← レスポンス: X-Request-Id をレスポンスヘッダーにコピー
            └→ store_request_id ← task-local に保存
                 └→ CSRF → authz → handler → client
```

```rust
.layer(from_fn_with_state(csrf_state, csrf_middleware))
.layer(axum::middleware::from_fn(store_request_id))
.layer(PropagateRequestIdLayer::x_request_id())
.layer(TraceLayer::new_for_http().make_span_with(make_request_span))
.layer(SetRequestIdLayer::x_request_id(MakeRequestUuidV7))
```

## スコープ

対象:
- BFF: Request ID 生成（UUID v7）、レスポンスヘッダー、task-local 伝播、クライアントヘッダー注入
- Core Service / Auth Service: `X-Request-Id` ヘッダーからスパンへの注入
- 共有モジュール: `MakeRequestUuidV7`、`make_request_span`、ヘッダー名定数

対象外:
- フロントエンド変更
- 監査ログとの統合（後続 Story）
- 内部サービスのレスポンスヘッダー（BFF が既に持っているため不要）

## Phase 構成

### Phase 1: 共有 observability 拡張（Request ID ユーティリティ）

共有クレートに Request ID 生成器とカスタムスパン作成関数を追加する。

変更ファイル:
- `backend/Cargo.toml` — `tower-http` に `request-id` フィーチャ追加
- `backend/crates/shared/Cargo.toml` — `uuid`, `http`, `tower-http`, `tracing` 依存追加（`observability` フィーチャ）
- `backend/crates/shared/src/observability.rs` — `MakeRequestUuidV7`, `make_request_span`, `REQUEST_ID_HEADER` 追加

主要コード:

```rust
use http::Request;
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Clone, Copy, Default)]
pub struct MakeRequestUuidV7;

impl MakeRequestId for MakeRequestUuidV7 {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = Uuid::now_v7().to_string().parse().unwrap();
        Some(RequestId::new(id))
    }
}

pub fn make_request_span<B>(request: &Request<B>) -> tracing::Span {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");
    tracing::info_span!(
        "request",
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
        request_id = %request_id,
    )
}
```

#### 確認事項
- [x] trait: `tower_http::request_id::MakeRequestId` のシグネチャ → docs.rs 確認済み: `fn make_request_id<B>(&mut self, request: &Request<B>) -> Option<RequestId>`
- [x] 型: `RequestId::new(HeaderValue)` → docs.rs 確認済み: `HeaderValue` をラップ
- [x] パターン: `Uuid::now_v7()` → プロジェクト内で広く使用（ドメインモデル ID 生成等）
- [x] ライブラリ: `http::Request::headers()` → 既存コード（CSRF ミドルウェア `csrf.rs` L81）で使用済み

#### テストリスト

ユニットテスト:
- [ ] `MakeRequestUuidV7` が有効な UUID v7 形式の `RequestId` を返す
- [ ] `MakeRequestUuidV7` が連続呼び出しで異なる ID を生成する
- [ ] `make_request_span` が `X-Request-Id` ヘッダーの値をスパンに含める
- [ ] `make_request_span` がヘッダー未設定時に `"-"` をフォールバックとして使用する

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 2: BFF Request ID 設定（生成・レスポンス・スパン）

BFF のルーターに `SetRequestIdLayer`、`PropagateRequestIdLayer`、カスタム `make_span_with` を追加する。

変更ファイル:
- `backend/apps/bff/src/main.rs` — レイヤー追加・順序変更

#### 確認事項
- [ ] ライブラリ: `SetRequestIdLayer::x_request_id(impl MakeRequestId)` → docs.rs
- [ ] ライブラリ: `PropagateRequestIdLayer::x_request_id()` → docs.rs
- [ ] パターン: BFF `main.rs` のレイヤー構成 → L259-373 確認済み

#### テストリスト

ユニットテスト（該当なし — main.rs 設定）

ハンドラテスト:
- [ ] ヘルスチェックのレスポンスに `X-Request-Id` ヘッダーが含まれる
- [ ] クライアント提供の `X-Request-Id` がそのまま返される
- [ ] 自動生成の `X-Request-Id` が UUID 形式である

API テスト（該当なし — Phase 5 でまとめて実施）

E2E テスト（該当なし）

### Phase 3: Core Service / Auth Service のスパン注入

両サービスの `TraceLayer` にカスタム `make_span_with` を適用する。

変更ファイル:
- `backend/apps/core-service/src/main.rs` — `TraceLayer` カスタマイズ
- `backend/apps/auth-service/src/main.rs` — `TraceLayer` カスタマイズ

#### 確認事項
- [ ] パターン: Core Service `main.rs` の `TraceLayer` 位置 → L337 確認済み
- [ ] パターン: Auth Service `main.rs` の `TraceLayer` 位置 → L127 確認済み

#### テストリスト

ユニットテスト（該当なし — main.rs 設定）

ハンドラテスト（該当なし — スパンフィールドはトレーシング内部）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 4: BFF → 内部サービスへの Request ID 伝播

task-local ストレージとクライアントヘッダー注入を実装する。

変更・作成ファイル:
- `backend/apps/bff/src/middleware/request_id.rs` — **新規**: task-local、`store_request_id` ミドルウェア、`inject_request_id` ヘルパー
- `backend/apps/bff/src/middleware.rs` — モジュール追加
- `backend/apps/bff/src/main.rs` — `store_request_id` レイヤー追加
- `backend/apps/bff/src/client/core_service/user_client.rs` — `inject_request_id` 適用（7 箇所）
- `backend/apps/bff/src/client/core_service/workflow_client.rs` — `inject_request_id` 適用（16 箇所）
- `backend/apps/bff/src/client/core_service/task_client.rs` — `inject_request_id` 適用（4 箇所）
- `backend/apps/bff/src/client/core_service/role_client.rs` — `inject_request_id` 適用（5 箇所）
- `backend/apps/bff/src/client/auth_service.rs` — `inject_request_id` 適用（2 箇所）

主要コード:

```rust
// backend/apps/bff/src/middleware/request_id.rs

tokio::task_local! {
    static REQUEST_ID: String;
}

pub fn current_request_id() -> Option<String> {
    REQUEST_ID.try_with(|id| id.clone()).ok()
}

pub async fn store_request_id(request: Request<Body>, next: Next) -> Response {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .unwrap_or("-")
        .to_string();
    REQUEST_ID.scope(request_id, next.run(request)).await
}

pub fn inject_request_id(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match current_request_id() {
        Some(id) => builder.header("x-request-id", id),
        None => builder,
    }
}
```

クライアント変更パターン:

```rust
// Before:
let response = self.client.get(&url).send().await?;
// After:
let response = inject_request_id(self.client.get(&url)).send().await?;
```

#### 確認事項
- [ ] 型: `tower_http::request_id::RequestId::header_value()` → docs.rs 確認済み: `&HeaderValue` を返す
- [ ] ライブラリ: `tokio::task_local!` マクロの使用法 → tokio docs
- [ ] ライブラリ: `axum::middleware::from_fn`（state なし）→ Grep 既存使用 or docs
- [ ] パターン: `self.client.get/post/patch` 呼び出しパターン → 34 箇所確認済み

#### テストリスト

ユニットテスト:
- [ ] `inject_request_id` が task-local 設定時にヘッダーを付与する
- [ ] `inject_request_id` が task-local 未設定時にビルダーを変更しない
- [ ] `current_request_id` が task-local スコープ外で `None` を返す

ハンドラテスト（該当なし — 透過的ミドルウェア）

API テスト（該当なし — Phase 5 でまとめて実施）

E2E テスト（該当なし）

### Phase 5: API テスト検証

API テストで `X-Request-Id` ヘッダーの存在とフォーマットを検証する。

変更ファイル:
- `tests/api/hurl/health.hurl` — `X-Request-Id` アサーション追加
- `tests/api/hurl/auth/login.hurl` — `X-Request-Id` アサーション追加

API テスト規約により、`X-Request-Id` は非決定的だが形式が既知（UUID v7）のため `matches` で検証する。

```hurl
# UUID v7 パターン: xxxxxxxx-xxxx-7xxx-[89ab]xxx-xxxxxxxxxxxx
header "x-request-id" matches "^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"
```

#### 確認事項
- [ ] パターン: Hurl ヘッダーアサーション構文 → `login.hurl` の `header "Set-Cookie" contains` パターン確認済み

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト:
- [ ] `health.hurl`: レスポンスに UUID v7 形式の `X-Request-Id` ヘッダーが含まれる
- [ ] `auth/login.hurl`: 全リクエストのレスポンスに `X-Request-Id` ヘッダーが含まれる

E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `MakeRequestUuid` は UUID v4 を使用、要件は UUID v7 | 競合・エッジケース | カスタム `MakeRequestUuidV7` を実装 |
| 2回目 | BFF → 内部サービスへの Request ID 伝播で 34 メソッドのシグネチャ変更が必要 | シンプルさ | `tokio::task_local!` で横断的関心事として伝播。シグネチャ変更なし |
| 3回目 | `make_request_span` を BFF に置くと Core/Auth で重複 | 既存手段の見落とし | `ringiflow-shared` の `observability` モジュールに共通関数として配置 |
| 4回目 | Core/Auth に `SetRequestIdLayer` は不要（BFF が生成） | アーキテクチャ不整合 | Core/Auth は `make_span_with` のみ適用。`SetRequestIdLayer` は BFF のみ |
| 5回目 | `inject_request_id` を CoreServiceClient のメソッドにすると AuthServiceClient で使えない | 既存手段の見落とし | 自由関数としてミドルウェアモジュールに配置し、両クライアントで共用 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 完了基準の全項目が計画に含まれている | OK | 6 項目すべてに対応する Phase がある（生成: P1-2、伝播: P3-4、ログ注入: P1-3、レスポンスヘッダー: P2、check-all: P5） |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | コードスニペット、ファイルパス、行番号で一意に確定 |
| 3 | 設計判断の完結性 | 全ての選択肢に判断が記載されている | OK | 4 つの設計判断に選択肢・理由・トレードオフを記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 「スコープ」セクションで対象・対象外を明示 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | tower レイヤー順序、tower-http `request-id` フィーチャ、UUID v7 フォーマットを docs.rs で確認済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Issue #650 要件、Epic #648 設計原則、運用設計書 9.4 と整合 |

## 検証方法

```bash
# 1. 全チェック
just check-all

# 2. 手動検証（開発サーバー起動後）
# レスポンスヘッダーに X-Request-Id が含まれることを確認
curl -v http://localhost:3000/health

# クライアント提供の X-Request-Id が維持されることを確認
curl -v -H "X-Request-Id: test-123" http://localhost:3000/health

# JSON ログに request_id が含まれることを確認（LOG_FORMAT=json で起動）
LOG_FORMAT=json just dev-bff
# → ログ出力に "request_id":"..." が含まれる
```
