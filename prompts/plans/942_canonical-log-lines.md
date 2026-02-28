# #942 Canonical Log Line の導入

## Context

現在のログ戦略ではビジネスイベント（`log_business_event!`）とスパン（`tracing::instrument`）が整備されているが、HTTP リクエスト単位のサマリログがない。1リクエストの全体像を把握するには `request_id` でのフィルタが必要で、複数行の結合が必要になる。

Stripe が提唱した Canonical Log Lines パターンを導入し、リクエスト完了時に1行で全重要情報を集約する。

## 設計方針

### アプローチ: Tower Layer（shared クレート）

`CanonicalLogLineLayer` を `shared/src/canonical_log.rs` に tower `Layer`/`Service` として実装する。

**選択理由:**
- shared クレートは axum に依存しない（`from_fn` は使えない）
- `tower = "0.5"` はワークスペース依存に既存
- 3サービスで共通の Layer として再利用可能

**却下した代替案:**
- TraceLayer の `on_response` 拡張: `on_response` コールバックからリクエスト情報（パス）にアクセスできず、ヘルスチェックフィルタが困難
- `axum::middleware::from_fn` per service: shared クレートに axum 依存が必要、3サービスでコード重複

### 既存 TraceLayer との責務分離

| 層 | 責務 | レベル |
|---|------|--------|
| TraceLayer | スパン作成（method, uri, request_id 等）。リクエストスコープのコンテキスト管理 | スパン |
| CanonicalLogLineLayer | リクエスト完了サマリ（status, latency）。1行で全体像を提供 | INFO イベント |

TraceLayer のデフォルト `on_request`/`on_response` は DEBUG レベル。Canonical log line は INFO レベルで `log.type = "canonical"` マーカー付き。

### JSON 出力スキーマ

`with_current_span(true)` + `flatten_event(true)` により、スパンフィールドが自動的にイベントに含まれる。Canonical log line イベントには `http.status_code`, `http.latency_ms`, `log.type` のみを追加し、残りはスパンから供給。

```json
{
  "timestamp": "2026-02-27T12:34:56.789Z",
  "level": "INFO",
  "target": "ringiflow_shared::canonical_log",
  "message": "リクエスト完了",
  "span": { "name": "request", "service": "bff" },
  "request_id": "019501a0-1234-7abc-8000-000000000001",
  "method": "POST",
  "uri": "/api/v1/workflows",
  "tenant_id": "019501a0-0000-7000-8000-000000000001",
  "user_id": "019501a0-9abc-7012-8000-000000000003",
  "log.type": "canonical",
  "http.status_code": 201,
  "http.latency_ms": 45
}
```

### レイヤー配置

CanonicalLogLineLayer は TraceLayer の内側（スパン内）に配置し、スパンフィールドを活用する。

BFF:
```
SetRequestIdLayer（最外）→ TraceLayer → CanonicalLogLineLayer → PropagateRequestIdLayer → store_request_id → no_cache → csrf → authz → handler
```

Core Service / Auth Service:
```
TraceLayer → CanonicalLogLineLayer → handler
```

axum の `.layer()` 記法（下が外側）:
```rust
// BFF
.layer(from_fn(store_request_id))
.layer(PropagateRequestIdLayer::x_request_id())
.layer(CanonicalLogLineLayer)  // 追加
.layer(TraceLayer::new_for_http().make_span_with(make_request_span))
.layer(SetRequestIdLayer::x_request_id(MakeRequestUuidV7))
```

### ヘルスチェックの除外

`path.starts_with("/health")` で判定。`/health`（liveness）と `/health/ready`（readiness）の両方を除外。

### tenant_id / user_id のスパン注入

- `tenant_id`: `make_request_span` で `X-Tenant-ID` ヘッダーから取得（全サービス共通。ヘッダー不在なら `"-"`）
- `user_id`: `make_request_span` で `tracing::field::Empty` として宣言。BFF の `authenticate()` 成功後に `Span::current().record()` で記録

## 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/shared/Cargo.toml` | `tower` 依存追加（observability feature） |
| `backend/crates/shared/src/lib.rs` | `canonical_log` モジュール追加 |
| `backend/crates/shared/src/canonical_log.rs` | 新規: `CanonicalLogLineLayer` / `CanonicalLogLineService` |
| `backend/crates/shared/src/observability.rs` | `make_request_span` 拡張（tenant_id, user_id）、`record_user_id` ヘルパー追加 |
| `backend/apps/bff/src/main.rs` | ミドルウェアスタックに `CanonicalLogLineLayer` 追加 |
| `backend/apps/core-service/src/main.rs` | 同上 |
| `backend/apps/auth-service/src/main.rs` | 同上 |
| `backend/apps/bff/src/error.rs` | `authenticate()` に `record_user_id` 呼び出し追加 |
| `docs/06_ナレッジベース/backend/log-schema.md` | Canonical Log Line フィールドスキーマ追加 |
| `docs/03_詳細設計書/14_Observability設計.md` | Canonical Log Line セクション追加、コンポーネント図更新 |

## 対象外

- TraceLayer のデフォルト `on_request`/`on_response` の変更（既存動作を維持）
- メトリクス（Phase 4 で実装予定）
- Canonical log line のログレベル動的変更

---

## Phase 1: make_request_span の拡張と record_user_id ヘルパー

`make_request_span` に `tenant_id` と `user_id` フィールドを追加し、`record_user_id` ヘルパーを実装する。

### 確認事項

- 型: `tracing::field::Empty` → Grep 既存使用確認 + docs.rs
- 型: `tracing::Span::record` → Grep 既存使用確認 + docs.rs
- パターン: 既存 `make_request_span` → `backend/crates/shared/src/observability.rs:149-163`

### 操作パス: 該当なし（インフラ層の変更、ユーザー操作なし）

### テストリスト

ユニットテスト:
- [ ] `make_request_span`: X-Tenant-ID ヘッダーありでスパンが作成されること
- [ ] `make_request_span`: X-Tenant-ID ヘッダーなしでスパンが作成されること（tenant_id = "-"）
- [ ] `record_user_id`: 現在のスパンの user_id フィールドに値を記録できること

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: CanonicalLogLineLayer の実装

Tower `Layer`/`Service` として Canonical Log Line ミドルウェアを実装する。

### 確認事項

- 型: `tower_service::Service` trait → `tower` crate（ワークスペース依存 0.5）
- 型: `tower_layer::Layer` trait → `tower` crate
- パターン: tower Service の clone-swap パターン → docs.rs tower crate
- ライブラリ: `http::Request`, `http::Response` → `backend/crates/shared/Cargo.toml` 既存依存

### 操作パス: 該当なし（インフラ層の変更）

### テストリスト

ユニットテスト:
- [ ] 正常リクエスト: canonical log line が INFO レベルで出力されること
- [ ] canonical log line に `log.type = "canonical"` が含まれること
- [ ] canonical log line に `http.status_code` が含まれること
- [ ] canonical log line に `http.latency_ms`（>= 0）が含まれること
- [ ] `/health` パス: canonical log line が出力されないこと
- [ ] `/health/ready` パス: canonical log line が出力されないこと
- [ ] Service エラー時: ERROR レベルで出力されること
- [ ] レスポンスが透過的に返されること（ステータスコード・ボディが変わらない）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 3: 各サービスへの組み込み + BFF user_id 記録

3サービスのミドルウェアスタックに `CanonicalLogLineLayer` を追加し、BFF の `authenticate()` に `record_user_id` 呼び出しを追加する。

### 確認事項

- パターン: BFF ミドルウェアスタック → `backend/apps/bff/src/main.rs:491-499`
- パターン: Core Service TraceLayer → `backend/apps/core-service/src/main.rs:479`
- パターン: Auth Service TraceLayer → `backend/apps/auth-service/src/main.rs:143`
- パターン: `authenticate()` → `backend/apps/bff/src/error.rs:94-101`

### 操作パス: 該当なし（インフラ層の変更、ユーザー操作への影響なし）

### テストリスト

ユニットテスト（該当なし — Phase 2 でカバー済み）

ハンドラテスト（該当なし）

API テスト (Hurl):
- [ ] BFF: 既存の API テストが CanonicalLogLineLayer 追加後もパスすること（レグレッションなし）
- [ ] Core Service: 同上
- [ ] Auth Service: 同上

E2E テスト（該当なし — 機能変更なし）

## Phase 4: ドキュメント更新

### 確認事項: なし（ドキュメントのみ）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

更新内容:
- `docs/06_ナレッジベース/backend/log-schema.md`: Canonical Log Line フィールドセクション追加、jq クエリ例追加
- `docs/03_詳細設計書/14_Observability設計.md`: Canonical Log Line セクション追加、ファイル構成更新

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `X-Tenant-ID` ヘッダーは Core/Auth Service に伝播しない場合がある | 不完全なパス | `make_request_span` でヘッダー不在時は `"-"` にフォールバック。3サービス共通で安全 |
| 2回目 | shared クレートに axum 依存がなく `from_fn` が使えない | アーキテクチャ不整合 | tower Layer/Service で実装。`tower = "0.5"` はワークスペース依存に既存 |
| 3回目 | `on_response` コールバックではリクエストパスにアクセスできない | 既存手段の見落とし | TraceLayer 拡張ではなく、独立した tower Layer として実装 |
| 4回目 | スパンフィールドとイベントフィールドで method/uri が重複する | シンプルさ | canonical log line イベントには `http.status_code`, `http.latency_ms`, `log.type` のみ追加。残りはスパンから自動供給 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了基準が全て計画に含まれている | OK | method/path/status/latency/request_id/tenant_id/user_id/error → Phase 1-2 でカバー。ログスキーマ更新 → Phase 4。ヘルスチェック除外 → Phase 2 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | JSON スキーマ、レイヤー配置、フィルタ条件すべて具体的に定義 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | Tower Layer vs from_fn vs on_response の比較、重複フィールドの扱い、ヘルスチェックフィルタ方式を決定 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象外セクションに TraceLayer デフォルト変更なし、メトリクスなしを明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | tower Service の clone-swap パターン、`tracing::field::Empty` の動作、`with_current_span(true)` のフラット化を確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | Observability 設計書のログポリシー（高頻度イベント抑制基準）、ログスキーマのフィールド命名規約と整合 |
