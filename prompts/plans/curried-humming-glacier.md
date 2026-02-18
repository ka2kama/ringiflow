# #649 ログ初期化の共通モジュール化と構造化ログ（JSON）対応

## Context

3サービス（BFF / Core Service / Auth Service）で同一のログ初期化コードが重複している。ADR-049 で「起動コードは安定したボイラープレートで変更頻度が低い」としてjscpd:ignore で意図的重複としたが、Epic #648（Observability 基盤）により状況が変わった:

- 環境変数による出力形式切替（JSON/Pretty）という設定ロジックが加わる
- JSON 出力のフィールド構成を3サービスで統一する必要がある
- 後続 Story（#650 Request ID、#651 PII マスキング、#654 計装）の基盤として変更頻度が上がる

## スコープ

対象:
- ログ初期化の共通モジュール化（`ringiflow_shared::observability`）
- `LOG_FORMAT` 環境変数による JSON/Pretty 出力切替
- 3サービスの main.rs を共通モジュール利用に移行
- ADR-049 補遺の記録

対象外:
- correlationId / tenantId / actorId 等（#650, #657）
- PII マスキング（#651）
- `tracing::instrument` の導入（#654）

## 設計判断

### 1. 異なるフォーマッタの型をどう扱うか → `.boxed()` による型消去

JSON と Pretty のフォーマッタは異なる型を生成する。`.boxed()` で `Box<dyn Layer<S>>` に統一する。

- tracing-subscriber が公式に提供する API
- 動的ディスパッチのオーバーヘッドはログ I/O に比べて無視できる
- 後続 Story でレイヤー追加時に1箇所の修正で済む

### 2. LogFormat のパース方法 → `std::str::FromStr` の手動実装

strum を shared に追加せず、2バリアント（Json/Pretty）の手動実装で十分。shared の設計方針「外部クレート依存の最小化」を維持。

### 3. service フィールドの追加 → トップレベルスパン方式

`init_tracing()` は初期化のみ。サービス名のスパンは呼び出し元（main.rs）で `tracing::info_span!("app", service = "bff").entered()` を設定。JSON 出力では `span.service` として含まれる。

### 4. feature gate → `observability` feature を採用

`openapi` feature の先例に倣い、tracing/tracing-subscriber を optional 依存にする。

## モジュール構造

```
backend/crates/shared/src/
├── lib.rs              # pub mod observability を追加（cfg_attr で feature gate）
├── observability/
│   └── mod.rs          # LogFormat, TracingConfig, init_tracing()
└── (既存モジュール)
```

## 公開 API

```rust
// observability/mod.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    Json,
    #[default]
    Pretty,
}

impl LogFormat {
    pub fn parse(s: &str) -> Self { ... }      // 純粋関数、テスト対象
    pub fn from_env() -> Self { ... }           // LOG_FORMAT 環境変数を読む
}

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub log_format: LogFormat,
}

impl TracingConfig {
    pub fn new(service_name: impl Into<String>, log_format: LogFormat) -> Self { ... }
    pub fn from_env(service_name: impl Into<String>) -> Self { ... }
}

pub fn init_tracing(config: TracingConfig) { ... }
```

## init_tracing の実装方針

```rust
pub fn init_tracing(config: TracingConfig) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,ringiflow=debug".into());

    let fmt_layer = match config.log_format {
        LogFormat::Json => tracing_subscriber::fmt::layer()
            .json()
            .flatten_event(true)
            .with_target(true)
            .with_current_span(true)
            .with_span_list(false)
            .boxed(),
        LogFormat::Pretty => tracing_subscriber::fmt::layer()
            .boxed(),
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
```

## 各サービスの main.rs 変更

変更前:
```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// ...
// jscpd:ignore-start
tracing_subscriber::registry()
    .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,ringiflow=debug".into()))
    .with(tracing_subscriber::fmt::layer())
    .init();
// jscpd:ignore-end
```

変更後:
```rust
use ringiflow_shared::observability::TracingConfig;
// ...
let tracing_config = TracingConfig::from_env("bff");  // or "core-service", "auth-service"
ringiflow_shared::observability::init_tracing(tracing_config);
let _tracing_guard = tracing::info_span!("app", service = "bff").entered();
```

サービス名: BFF → `"bff"`, Core → `"core-service"`, Auth → `"auth-service"`

## Cargo.toml 変更

shared/Cargo.toml:
```toml
[features]
openapi = ["dep:utoipa"]
observability = ["dep:tracing", "dep:tracing-subscriber"]

[dependencies]
tracing = { workspace = true, optional = true }
tracing-subscriber = { workspace = true, optional = true }
```

各 app の Cargo.toml:
- BFF: `ringiflow-shared = { workspace = true, features = ["openapi", "observability"] }`
- Core/Auth: `ringiflow-shared = { workspace = true, features = ["observability"] }`
- 各 app から `tracing-subscriber` の直接依存を削除（main.rs 以外で使用なし、確認済み）

## 変更対象ファイル一覧

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/shared/src/lib.rs` | `pub mod observability` 追加（feature gate） |
| `backend/crates/shared/src/observability/mod.rs` | 新規作成: LogFormat, TracingConfig, init_tracing |
| `backend/crates/shared/Cargo.toml` | observability feature + tracing 依存追加 |
| `backend/apps/bff/src/main.rs` | init_tracing 呼び出しに置換、jscpd:ignore 削除 |
| `backend/apps/core-service/src/main.rs` | 同上 |
| `backend/apps/auth-service/src/main.rs` | 同上 |
| `backend/apps/bff/Cargo.toml` | shared features 変更、tracing-subscriber 削除 |
| `backend/apps/core-service/Cargo.toml` | 同上 |
| `backend/apps/auth-service/Cargo.toml` | 同上 |
| `backend/.env.template` | LOG_FORMAT 追記 |
| `backend/.env.api-test.template` | LOG_FORMAT 追記 |
| `docs/05_ADR/049_...md` | 補遺: クローン2の判断変更を記録 |

## Phase 分割

### Phase 1: LogFormat enum + TracingConfig（TDD）

対象: `observability/mod.rs`（型とロジック）, `shared/Cargo.toml`, `shared/src/lib.rs`

#### 確認事項
- [x] 型: shared クレートの既存モジュール構造 → `lib.rs`（pub mod + pub use の re-export）、`health.rs`（//! doc comment + #[cfg(test)] mod tests）
- [x] パターン: テスト命名パターン → `test_[日本語]()` 形式
- [x] ライブラリ: `Default` derive → std 標準、確認不要

#### テストリスト

ユニットテスト:
- [x] `LogFormat::parse("json")` が `Json` を返す
- [x] `LogFormat::parse("pretty")` が `Pretty` を返す
- [x] `LogFormat::parse("unknown")` が `Pretty` にフォールバックする
- [x] `LogFormat::default()` が `Pretty` を返す
- [x] `TracingConfig::new` でフィールドが正しく設定される

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 2: init_tracing() の実装

対象: `observability/mod.rs`

#### 確認事項
- [x] ライブラリ: `fmt::layer().json()` の API → docs.rs で確認、`Layer<S, JsonFields, Format<Json, T>, W>` を返す
- [x] ライブラリ: `.boxed()` メソッド → docs.rs で確認、Layer trait のメソッドとして存在
- [x] ライブラリ: `flatten_event(true)`, `with_current_span`, `with_span_list` → docs.rs で確認、全 API 存在

#### テストリスト

ユニットテスト（該当なし — init_tracing はグローバル状態を変更するためユニットテスト不適。コンパイル成功で型の整合性を検証）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動検証:
- `cargo test -p ringiflow-shared --features observability` 通過
- `cargo build -p ringiflow-shared --features observability` 成功

### Phase 3: 3サービス統合 + Cargo.toml 変更

対象: 3つの main.rs、3つの app Cargo.toml、.env テンプレート

#### 確認事項
- [x] パターン: 各 main.rs の tracing 初期化コードの位置と jscpd:ignore マーカー → BFF(L128-143), Core(L149-177), Auth(L79-112) で確認
- [x] 依存: `tracing_subscriber` が main.rs 以外で使用されていないこと → Grep 確認済み（3ファイルのみ）

#### テストリスト

ユニットテスト（該当なし — 既存テストがそのまま通ることを確認）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動検証:
- `just check-all` 通過
- `just dev-core-service` でデフォルト（Pretty）出力確認
- `LOG_FORMAT=json just dev-core-service` で JSON 出力確認
- JSON 出力に timestamp, level, target, message フィールドが含まれること
- `tracing::info_span!` による span.service フィールド確認

### Phase 4: ADR 補遺 + ドキュメント更新

対象: ADR-049 補遺、.env テンプレート

確認事項: なし（既知のパターンのみ）

テストリスト: 該当なし（全テスト層）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ADR-049 のクローン2判断を覆す必要がある | アーキテクチャ不整合 | Phase 4 に ADR 補遺を追加 |
| 2回目 | Rust 2024 edition で `std::env::set_var` が unsafe | 技術的前提 | LogFormat の parse() と from_env() を分離、parse のみテスト対象に |
| 3回目 | service フィールドのトップレベル配置が標準 API では困難 | 既存手段の見落とし | スパン方式を採用、init_tracing の責務からスパン生成を分離 |
| 4回目 | init_tracing はグローバル状態変更のためユニットテスト不適 | 不完全なパス | Phase 2 のテストから自動テストを除外、手動検証に変更 |
| 5回目 | .env テンプレートへの LOG_FORMAT 追記漏れ | 未定義 | Phase 3 の対象ファイルに追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | main.rs 3、shared の lib.rs + 新規 mod.rs、Cargo.toml 4、.env テンプレート 2、ADR 補遺を網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全設計判断に選択肢・理由を記載。コードスニペットで挙動が一意に確定 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 型消去方式、LogFormat パース、service フィールド方式、feature gate の4判断に推奨と理由を明記 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象（ログ初期化共通化 + JSON）、対象外（correlationId, PII, instrument）を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Rust 2024 unsafe set_var、グローバル初期化制約、.boxed() 振る舞い、flatten_event 影響を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | ADR-049 との関係を明示。運用設計書 9.4「構造化ログ必須」に合致。Epic #648 依存グラフと整合 |
