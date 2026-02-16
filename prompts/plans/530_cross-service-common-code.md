# Issue #530: サービス間共通コード抽出

## Context

Epic #467（jscpd 警告ゼロ）の Story 7。先行 Story (#525-#529, #533) でサービス内クローン削減が完了。本 Story はサービス間の共通コード抽出を扱う。

As-Is 検証の結果、元の13クローンのうち2件は先行 Story で解消済み、5件はテストコード（#531 スコープ）。プロダクションコードで対処が必要な4件を評価し、以下の方針を決定した。

## スコープ

対象:
- クローン1: Health Check ハンドラ（auth ↔ core、BFF も類似）→ **HealthResponse 型を shared に抽出**
- クローン2: main.rs 起動コード（3サービスで酷似）→ **jscpd:ignore で意図的重複をマーク**
- クローン3: BFF auth_service.rs HTTP match パターン → **対処不要**（jscpd 未検出、2回の重複で許容範囲）
- クローン4: BFF types ↔ Core handler 型 → **対処不要**（jscpd 未検出、マイクロサービスの独立性を優先）

対象外:
- テストコード内の重複 → #531（テストコード共通化）
- 先行 Story で解消済みの2クローン（authz.rs ↔ role.rs、auth handler auth↔core）

## 設計判断

### Health Check: 型のみ抽出、ハンドラ関数は各サービスに残す

shared に axum 依存を入れない。shared の設計方針「外部クレートへの依存は最小限に抑える」を維持する。

HealthResponse（8行）を shared に移動すれば、残る共通コードは health_check 関数本体（5行程度）のみとなり、jscpd の `--min-lines 10` 閾値を下回る。

`env!("CARGO_PKG_VERSION")` は各サービスのハンドラ関数内に残るため、workspace version で管理されている限り正しいバージョンが返る。

### main.rs: 共通化せず、jscpd:ignore で意図的重複をマーク

tracing init は `tracing-subscriber` に依存する。subscriber 設定はアプリケーション層の責務であり、shared（ライブラリ層）に置くべきではない。

main.rs の起動コードは安定したボイラープレートであり、変更頻度が低い。DRY が排除すべき「変更の同期が必要な重複」ではなく、可読性を優先して各サービスに残す。

### BFF client HTTP match / BFF types: 対処不要

jscpd で検出されておらず（閾値未満 or 差異あり）、クローン3は2回の重複（「3回まで許容」の原則内）、クローン4はマイクロサービスの独立性を優先する設計判断。

---

## Phase 1: HealthResponse 型を shared crate に抽出

### 確認事項
- [x] 型: HealthResponse の derive/フィールド → `auth-service/handler/health.rs` L30-36、`#[derive(Debug, Serialize)]` + status: String, version: String
- [x] パターン: `cfg_attr(feature = "openapi")` → `shared/src/api_response.rs` L24、`#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]`
- [x] パターン: shared モジュール追加 → `shared/src/lib.rs` L12-18、`pub mod xxx; pub use xxx::Xxx;` パターン

### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/shared/src/health.rs` | **新規作成**: HealthResponse 型（`cfg_attr` で openapi 対応） |
| `backend/crates/shared/src/lib.rs` | モジュール追加 + re-export |
| `backend/apps/auth-service/src/handler/health.rs` | `ringiflow_shared::HealthResponse` を使用、ローカル定義を削除 |
| `backend/apps/core-service/src/handler/health.rs` | 同上 |
| `backend/apps/bff/src/handler/health.rs` | 同上（`ToSchema` derive は shared の openapi feature で対応済み） |

### 設計

shared/src/health.rs:
```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}
```

各サービスの handler/health.rs:
```rust
use axum::Json;
use ringiflow_shared::HealthResponse;

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
```

BFF のみ `#[utoipa::path]` アノテーションが残る。utoipa の `responses` で `body = HealthResponse` を参照するが、shared の openapi feature で `ToSchema` が derive されているので問題なし。

### テストリスト

ユニットテスト:
- [ ] HealthResponse の JSON シリアライズが `{ "status": "...", "version": "..." }` 形式になること
- [ ] openapi feature 有効時に ToSchema が実装されていること（`api_response.rs` のテストパターン準拠）

ハンドラテスト（該当なし — 各サービスの health_check 関数は変更なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 2: main.rs に jscpd:ignore を適用

### 確認事項
- [x] ツール: jscpd の ignore コメント構文 → `// jscpd:ignore-start` / `// jscpd:ignore-end`。Rust は `//` コメント形式

### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/auth-service/src/main.rs` | startup boilerplate に `jscpd:ignore-start/end` 追加 |
| `backend/apps/core-service/src/main.rs` | 同上 |
| `backend/apps/bff/src/main.rs` | 同上（server startup 部分のみ） |

### 対象範囲

auth-service/main.rs L78-141: dotenvy → tracing init → config → DB pool → migration → router → server start
core-service/main.rs L148-354: 同上
bff/main.rs L128-141: dotenvy → tracing init（DB 部分なし）、L382-393: server start

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `just check-duplicates` でサービス間 main.rs クローンが検出されなくなること

## Phase 3: ADR 記録 + Issue 更新

設計判断（共通化する/しないの判断）を ADR に記録する。Issue #530 のチェックリストを更新する。

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | shared に axum を追加すべきか未決定 | シンプルさ | 型のみ抽出、ハンドラ関数は各サービスに残す方針に決定 |
| 2回目 | `env!("CARGO_PKG_VERSION")` が shared のバージョンを返す問題 | 技術的前提 | ハンドラ関数を各サービスに残すため問題回避。env! は各サービスの main crate で展開 |
| 3回目 | main.rs の tracing init を shared に抽出すべきか | アーキテクチャ不整合 | subscriber 設定はアプリ層の責務。shared はライブラリ層。抽出しない |
| 4回目 | BFF types ↔ Core 型の共通化がマイクロサービス独立性を損なう | アーキテクチャ不整合 | API contract テストで整合性担保。共通化しない |
| 5回目 | クローン3, 4 が jscpd で実際に検出されていない | 既存手段の見落とし | jscpd 未検出のため対処不要。Phase 2 は main.rs のみに限定 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 4クローンすべてに対処方針がある | OK | クローン1: shared に型抽出、クローン2: jscpd:ignore、クローン3: 対処不要、クローン4: 対処不要 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各クローンに「共通化する/しない」の明確な判断と理由がある |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | BFF utoipa 差異、shared 依存方針、env! マクロ、マイクロサービス型共有、jscpd 検出状況 |
| 4 | スコープ境界 | 対象と対象外が明記されている | OK | 対象4クローン明記、テストコード(#531)除外、先行Story解消済み除外 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | env! 展開先、workspace version 管理、jscpd 閾値(10行/50トークン)、jscpd:ignore 構文 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | shared 設計方針(lib.rs)、ADR-042(jscpd)、ADR-023(レイヤー構造) |

## 検証

1. `cargo test -p ringiflow-shared` — shared crate のテスト通過
2. `cargo test -p ringiflow-shared --features openapi` — openapi テスト通過
3. `just check` — 全体の lint + テスト通過
4. `just check-duplicates` — health.rs クローンが消失、main.rs クローンが jscpd:ignore で非検出
5. `just check-all` — 最終確認
