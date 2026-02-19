# #657 ビジネスイベントログとエラーコンテキスト設計

## Context

Issue #648 (Observability Epic) の Story。AI エージェントが本番障害を `jq` で効率的に調査できるよう、ビジネスイベントの構造化ログとエラーコンテキストの拡充を実装する。

前提: #649 (JSON ログ基盤)、#650 (Request ID)、#651 (PII マスキング) はすべて完了済み。`observability.rs` で `flatten_event(true)` + `with_current_span(true)` の JSON 出力が有効であり、Request ID はスパンフィールドとして自動注入される。

## 設計判断

### 1. ビジネスイベントのログ層: Core Service ユースケース層

| 案 | メリット | デメリット |
|---|---|---|
| A. ハンドラ層 | HTTP コンテキストあり | display_number 委譲でパスが重複 |
| B. ユースケース層 | UUID/display_number 両パスをカバー | HTTP コンテキストなし |
| C. ドメイン層 | 最も深い | ログ依存がドメインに侵入 |

**B を選択。** `*_by_display_number` バリアントは UUID バリアントに委譲するため（`approve.rs` L183、`submit.rs` L189）、ユースケース層の UUID バリアントにログを配置すれば両パスをカバーできる。

### 2. フィールド命名: ドット記法

tracing のフィールド名は `$($field:ident).+` パターンでドット区切りを許容する。`event.category`、`error.kind` のようなドット記法を使用。JSON 出力では `jq 'select(.["event.category"] == "workflow")'` で参照。

### 3. `error.kind` を採用（`error.type` ではなく）

`type` は Rust の予約語。`error.kind` は `std::io::ErrorKind` と一貫性があり、Rust エコシステムで自然。

### 4. マクロ設計: `log_business_event!`

`tracing::info!` のラッパー。`event.kind = "business_event"` マーカーを自動付与。Grep で全ビジネスイベントを一括検索可能にする。エラーコンテキストは専用マクロ不要（既存の `tracing::error!` にフィールド追加するだけ）。

### 5. 認証イベントのログ層: BFF ハンドラ

認証は BFF の責務。Core Service は認証を知らないため、BFF の auth ハンドラでログを出力する。

### 6. エラーコンテキスト強化のスコープ

中央エラーハンドリング（3 サービスの error.rs）+ 重要ハンドラ（BFF auth login）に限定。残りのハンドラはこの PR で確立したパターンを基に後続 Issue で対応。

## スコープ

対象:
- `log_business_event!` マクロと定数モジュール（`shared` クレート）
- ログスキーマ文書（`docs/06_ナレッジベース/backend/`）
- ワークフロー操作のビジネスイベントログ（6 ユースケース）
- 認証操作のビジネスイベントログ（login/logout）
- エラーコンテキスト強化（中央エラーハンドリング + auth login ハンドラ）

対象外:
- フロントエンド
- BFF のその他ハンドラ（user, role, audit_log, session）のエラーコンテキスト
- Infra 層のエラーログ強化
- ログの自動テスト基盤（tracing-test 等の導入）

## Phase 計画

### Phase 1: ヘルパーマクロ + ログスキーマ文書

#### 確認事項
- [x] パターン: `observability.rs` の `#[cfg(feature = "observability")]` ゲーティング → `observability.rs` L88 で確認、関数単位でゲート
- [x] ライブラリ: tracing のフィールド名でドット記法が使えるか → tracing の `$($field:ident).+` パターン、コンパイルで検証
- [x] パターン: `#[macro_export]` マクロの使用慣例 → プロジェクト初。Rust 慣例に従う

#### 実装内容

1. **`backend/crates/shared/src/event_log.rs`（新規）**

```rust
//! ビジネスイベントログとエラーコンテキストの構造化ヘルパー

/// ビジネスイベントを構造化ログとして出力する。
///
/// `event.kind = "business_event"` マーカーを自動付与し、
/// `tracing::info!` レベルで出力する。
///
/// ## 必須フィールド（慣例）
///
/// - `event.category`: イベントカテゴリ（"workflow", "auth"）
/// - `event.action`: アクション名（"workflow.created", "step.approved"）
/// - `event.tenant_id`: テナント ID
/// - `event.result`: 結果（"success", "failure"）
///
/// ## 推奨フィールド
///
/// - `event.entity_type`: エンティティ種別
/// - `event.entity_id`: エンティティ ID
/// - `event.actor_id`: 操作者 ID
#[macro_export]
macro_rules! log_business_event {
    ($($args:tt)*) => {
        ::tracing::info!(
            event.kind = "business_event",
            $($args)*
        )
    };
}

/// イベントフィールドの定数
pub mod event {
    /// イベントカテゴリ
    pub mod category {
        pub const WORKFLOW: &str = "workflow";
        pub const AUTH: &str = "auth";
    }
    /// イベントアクション
    pub mod action {
        pub const WORKFLOW_CREATED: &str = "workflow.created";
        pub const WORKFLOW_SUBMITTED: &str = "workflow.submitted";
        pub const STEP_APPROVED: &str = "step.approved";
        pub const STEP_REJECTED: &str = "step.rejected";
        pub const STEP_CHANGES_REQUESTED: &str = "step.changes_requested";
        pub const WORKFLOW_RESUBMITTED: &str = "workflow.resubmitted";
        pub const LOGIN_SUCCESS: &str = "auth.login_success";
        pub const LOGIN_FAILURE: &str = "auth.login_failure";
        pub const LOGOUT: &str = "auth.logout";
    }
    /// エンティティ種別
    pub mod entity_type {
        pub const WORKFLOW_INSTANCE: &str = "workflow_instance";
        pub const WORKFLOW_STEP: &str = "workflow_step";
        pub const USER: &str = "user";
        pub const SESSION: &str = "session";
    }
    /// イベント結果
    pub mod result {
        pub const SUCCESS: &str = "success";
        pub const FAILURE: &str = "failure";
    }
}

/// エラーコンテキストフィールドの定数
pub mod error {
    /// エラーカテゴリ
    pub mod category {
        pub const INFRASTRUCTURE: &str = "infrastructure";
        pub const EXTERNAL_SERVICE: &str = "external_service";
    }
    /// エラー種別
    pub mod kind {
        pub const DATABASE: &str = "database";
        pub const SESSION: &str = "session";
        pub const INTERNAL: &str = "internal";
        pub const USER_LOOKUP: &str = "user_lookup";
        pub const PASSWORD_VERIFICATION: &str = "password_verification";
        pub const CSRF_TOKEN: &str = "csrf_token";
        pub const SERVICE_COMMUNICATION: &str = "service_communication";
    }
}
```

2. **`backend/crates/shared/src/lib.rs`** に追加:

```rust
#[cfg(feature = "observability")]
pub mod event_log;
```

3. **`docs/06_ナレッジベース/backend/log-schema.md`（新規）**: ビジネスイベントとエラーコンテキストのフィールドスキーマ、jq クエリ例を記載

#### テストリスト

ユニットテスト:
- [ ] `log_business_event!` マクロがコンパイルできること（doc test）
- [ ] 定数値の網羅性確認（ドキュメントとの一致）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 2: ワークフロー ビジネスイベントログ

#### 確認事項
- [x] 型: 各ユースケースのシグネチャで利用可能な ID → 全6ファイル確認済み。create は instance 経由、submit/resubmit は instance_id param、decision 系は step_id + user_id param
- [x] パターン: `use ringiflow_shared::log_business_event` のインポート方法 → `#[macro_export]` はクレートルートにエクスポート。`use ringiflow_shared::log_business_event;` で利用可能

#### 実装内容

各ユースケースの成功パス（`Ok` 返却直前）に `log_business_event!` を追加:

| ファイル | action | entity_type | entity_id |
|---------|--------|-------------|-----------|
| `lifecycle/create.rs` | `workflow.created` | `workflow_instance` | `result.instance.id()` |
| `lifecycle/submit.rs` | `workflow.submitted` | `workflow_instance` | `instance_id` (param) |
| `decision/approve.rs` | `step.approved` | `workflow_step` | `step_id` (param) |
| `decision/reject.rs` | `step.rejected` | `workflow_step` | `step_id` (param) |
| `decision/request_changes.rs` | `step.changes_requested` | `workflow_step` | `step_id` (param) |
| `lifecycle/resubmit.rs` | `workflow.resubmitted` | `workflow_instance` | `instance_id` (param) |

全ユースケースで `tenant_id` と `user_id` はパラメータとして利用可能。

使用例（`approve.rs`）:

```rust
use ringiflow_shared::log_business_event;
use ringiflow_shared::event_log::event;

// Ok 返却直前
log_business_event!(
    event.category = event::category::WORKFLOW,
    event.action = event::action::STEP_APPROVED,
    event.entity_type = event::entity_type::WORKFLOW_STEP,
    event.entity_id = %step_id,
    event.actor_id = %user_id,
    event.tenant_id = %tenant_id,
    event.result = event::result::SUCCESS,
    "承認ステップ完了"
);
```

#### テストリスト

ユニットテスト: 既存テストの通過確認（ログ追加は副作用のみ）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 3: 認証 ビジネスイベントログ

#### 確認事項
- [x] 型: `login` ハンドラ内で利用可能な ID → `user.id` (Uuid), `user.tenant_id` (Uuid), `session_id` (String)
- [x] パターン: PII マスキング定数 `REDACTED` → `domain/lib.rs` L63, `"[REDACTED]"`

#### 実装内容

`backend/apps/bff/src/handler/auth/login.rs`:

| 箇所 | action | event.result | 補足 |
|------|--------|-------------|------|
| L151 付近（セッション作成成功後） | `auth.login_success` | `success` | `user.id`, `user.tenant_id` 利用可能 |
| L159（パスワード不一致） | `auth.login_failure` | `failure` | `user.id` 利用可能、`event.reason = "password_mismatch"` |
| L176（ユーザー不存在） | `auth.login_failure` | `failure` | `event.entity_id = REDACTED`（PII マスキング）、`event.reason = "user_not_found"` |
| L233 付近（Cookie クリア前） | `auth.logout` | `success` | `session_id` 利用可能 |

#### テストリスト

ユニットテスト: 既存テストの通過確認
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 4: エラーコンテキスト強化

#### 確認事項
- [x] パターン: 各 error.rs の `tracing::error!` のフィールド構文 → core(2), auth(2), bff(2) で計6箇所
- [x] パターン: `log_and_convert_core_error` の使用箇所 → `bff/error.rs` L148, Network/Unexpected のみログ

#### 実装内容

既存の `tracing::error!` に `error.category` + `error.kind` フィールドを追加。

**1. `core-service/error.rs` (2 sites)**

```rust
// Before:
tracing::error!("データベースエラー: {}", e);
// After:
tracing::error!(
    error.category = "infrastructure",
    error.kind = "database",
    "データベースエラー: {}", e
);
```

**2. `auth-service/error.rs` (2 sites)**: 同パターン

**3. `bff/error.rs` (2 sites)**

| 関数 | error.category | error.kind |
|------|---------------|-----------|
| `get_session` (L79) | `infrastructure` | `session` |
| `log_and_convert_core_error` (L146) | `external_service` | `service_communication` |

**4. `bff/handler/auth/login.rs` (5 error sites)**

| 行 | メッセージ | error.category | error.kind |
|----|----------|---------------|-----------|
| L108 | ユーザー情報取得で内部エラー | `external_service` | `user_lookup` |
| L132 | CSRF トークン作成に失敗 | `infrastructure` | `csrf_token` |
| L154 | セッション作成に失敗 | `infrastructure` | `session` |
| L162 | パスワード検証で内部エラー | `external_service` | `password_verification` |
| L179 | ユーザー検索で内部エラー | `external_service` | `user_lookup` |

#### テストリスト

ユニットテスト: 既存テストの通過確認（フィールド追加は出力変更のみ）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 5: 統合検証

#### 確認事項
確認事項: なし（既知のパターンのみ）

#### 実施内容

1. `just check-all` 通過
2. `LOG_FORMAT=json` で開発サーバーを起動し、手動操作で検証:
   - ワークフロー作成 → `jq 'select(.["event.kind"] == "business_event")'` でイベント確認
   - ログイン → auth イベント確認
   - エラーログに `error.category`, `error.kind` が含まれること
3. ログスキーマ文書の内容が実際の出力と一致すること

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — 手動検証で代替）

## 変更ファイル一覧

新規:
- `backend/crates/shared/src/event_log.rs`
- `docs/06_ナレッジベース/backend/log-schema.md`

変更:
- `backend/crates/shared/src/lib.rs`
- `backend/apps/core-service/src/usecase/workflow/command/lifecycle/create.rs`
- `backend/apps/core-service/src/usecase/workflow/command/lifecycle/submit.rs`
- `backend/apps/core-service/src/usecase/workflow/command/decision/approve.rs`
- `backend/apps/core-service/src/usecase/workflow/command/decision/reject.rs`
- `backend/apps/core-service/src/usecase/workflow/command/decision/request_changes.rs`
- `backend/apps/core-service/src/usecase/workflow/command/lifecycle/resubmit.rs`
- `backend/apps/bff/src/handler/auth/login.rs`
- `backend/apps/core-service/src/error.rs`
- `backend/apps/auth-service/src/error.rs`
- `backend/apps/bff/src/error.rs`

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ハンドラ層でのログは display_number 委譲パスで重複する | 不完全なパス | ユースケース層に変更。`approve.rs` L183 の委譲パターンを確認 |
| 2回目 | `error.type` は Rust 予約語で使えない | 技術的前提 | `error.kind` に変更（`std::io::ErrorKind` と一貫） |
| 3回目 | エラーコンテキスト強化の全サイト対応はスコープ過大（34+ sites） | シンプルさ | 中央 error.rs + auth login に限定、残りは後続 Issue で対応 |
| 4回目 | `event_log.rs` の feature gate が未検討 | 既存手段の見落とし | `#[cfg(feature = "observability")]` で observability.rs と同パターンに |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue #657 の要件: ビジネスイベントログ（Ph2-3）、エラーコンテキスト（Ph4）、スキーマ文書（Ph1）、検証（Ph5）。全要素に対応する Phase あり |
| 2 | 曖昧さ排除 | OK | 全イベントのフィールド値をテーブルで具体的に定義。「適宜」「必要に応じて」の記述なし |
| 3 | 設計判断の完結性 | OK | ログ層（3案比較→B）、フィールド命名（ドット記法）、マクロ設計、error.kind 命名、スコープ限定の判断と理由を記載 |
| 4 | スコープ境界 | OK | 対象 5 項目、対象外 4 項目を明記。エラーコンテキスト未対応サイト（23 sites）を対象外に明示 |
| 5 | 技術的前提 | OK | tracing のドット記法（`$($field:ident).+`）、`#[cfg(feature)]` ゲーティング、`#[macro_export]` のクレート間参照、全 apps が `observability` feature を有効化していることを確認 |
| 6 | 既存ドキュメント整合 | OK | 運用設計書の構造化ログ要件（action, resource, result, correlationId）に合致。audit_log.rs の `AuditAction` ドット記法（`user.create` 等）と命名パターンが一貫 |
