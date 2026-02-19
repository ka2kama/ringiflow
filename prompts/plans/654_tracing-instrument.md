# 計画: Issue #654 アプリケーション計装（tracing::instrument）の導入

## コンテキスト

Epic #648（Observability 基盤）の Story 4。`tracing::instrument` マクロを主要なアプリケーションパスに導入し、関数レベルのスパン構造を確立する。

依存 Story:
- #649（ログ共通化 + JSON）: 完了済み — 共通 observability モジュール `ringiflow_shared::observability` が存在
- #651（PII マスキング）: 完了済み — Email, UserName, LoginRequest, SessionData にカスタム Debug 実装

現状（As-Is）:
- `tracing::instrument` 使用: 0 箇所
- `TraceLayer::new_for_http()` による HTTP リクエスト/レスポンスレベルのトレースは存在
- 関数内部の処理時間やネスト構造は可視化されていない

理想（To-Be）:
- HTTP リクエスト → ハンドラ → リポジトリ → DB クエリ の各層にスパンが存在
- スパンの親子関係により、1 リクエスト内の処理フローが追跡可能

## 設計判断

### 1. skip パターン: `skip_all` を全レイヤーで採用

全関数で `skip_all` を使い、`fields()` で安全なフィールドのみ明示的に追加する。

理由:
- PII 漏洩リスクがゼロ（デフォルト安全）
- 新フィールド追加時にも skip 漏れが起きない
- ハンドラの State, HeaderMap, CookieJar 等は Debug 出力しても無意味
- Story 3 の PII マスキング方針（型レベルで防止）と一貫

### 2. スパンレベル

| レイヤー | レベル | 理由 |
|---------|--------|------|
| ハンドラ | INFO（デフォルト） | リクエスト処理の入口、本番でも有効 |
| BFF クライアント | DEBUG | サービス間通信の詳細、トラブルシュート用 |
| リポジトリ / セッション | DEBUG | DB/Redis アクセスの詳細、トラブルシュート用 |

デフォルトフィルタ `info,ringiflow=debug` のため、ringiflow モジュールでは DEBUG スパンも記録される。

### 3. `fields()` で記録するフィールド

有用かつ安全なパラメータのみ。以下のルールで判定する:

| 記録する | 記録しない |
|---------|-----------|
| Path パラメータ（display_number, UUID ID） | State, HeaderMap, CookieJar |
| テナント ID（リポジトリ/クライアントの引数） | リクエストボディ全体 |
| エンティティ ID | PII（email, password, credential_data） |
| | 大きなオブジェクト（User, SessionData） |

`fields()` 記法:
- `fields(display_number)` — パラメータ名と同名のフィールドを Debug で記録
- `fields(%tenant_id)` — Display で記録（TenantId, UserId 等の newtype ID）
- `fields(display_number = params.display_number)` — 構造体フィールドから取得

### 4. テストアプローチ

`#[tracing::instrument]` は関数のシグネチャも挙動も変えない属性マクロ。新しいテストは書かない。

検証方法:
- コンパイル: `cargo check` — 属性の正当性（skip 対象の存在、型の Debug 実装）
- 既存テスト: `cargo test` — 振る舞いが変わっていないこと
- 品質ゲート: `just check-all` — 全テスト通過
- 手動検証: `RUST_LOG=debug just dev-all` でスパンのネスト構造を確認

理由:
- PII 非漏洩は Story 3 のカスタム Debug テスト（`test_ログインリクエストのdebug出力はメールアドレスとパスワードをマスクする` 等）で担保済み
- `#[instrument]` の動作は tracing クレート側の責任
- 属性の有無をテストする方法は reflection 的でアンチパターン

### 5. 属性の配置順序

```rust
// ハンドラ: utoipa → instrument → fn
#[utoipa::path(...)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn get_workflow(...) -> ... { ... }

// リポジトリ/クライアント: #[async_trait] impl 内のメソッドに付与
#[async_trait]
impl UserRepository for PostgresUserRepository {
    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn find_by_email(&self, tenant_id: &TenantId, email: &Email) -> ... { ... }
}
```

## 対象・対象外

対象:
- 全サービスの HTTP ハンドラ（health_check を除く）
- BFF → Core Service / Auth Service の HTTP クライアント
- PostgreSQL リポジトリ実装（`PostgresXxxRepository`）
- DynamoDB 監査ログリポジトリ（`DynamoDbAuditLogRepository`）
- Redis セッションマネージャ（`RedisSessionManager`）

対象外:
- `health_check` 関数 — ロードバランサーの高頻度呼び出しでノイズになる
- ユースケース層 — ハンドラ（入口）とリポジトリ（出口）で十分。将来拡張可能
- ミドルウェア（authz, csrf, request_id） — Tower TraceLayer で別途カバー
- テスト用スタブ/モック — テストの関心事ではない
- `dev_auth.rs` — 開発用モジュール
- deletion モジュール — バッチ処理用、リクエストパス外

---

## Phase 1: ハンドラ計装（全サービス）

### 確認事項

- [ ] パターン: BFF ハンドラの関数シグネチャ → `backend/apps/bff/src/handler/auth/login.rs`（代表例）
- [ ] パターン: Core Service ハンドラの関数シグネチャ → `backend/apps/core-service/src/handler/auth/mod.rs`
- [ ] パターン: Auth Service ハンドラの関数シグネチャ → `backend/apps/auth-service/src/handler/auth.rs`
- [ ] 型: BFF `StepPathParams` のフィールド名 → `backend/apps/bff/src/handler/workflow.rs`
- [ ] 型: Core `StepPathParams`, `StepByDisplayNumberPathParams` のフィールド名 → `backend/apps/core-service/src/handler/workflow.rs`
- [ ] ライブラリ: `tracing::instrument` と `skip_all` + `fields()` の組み合わせ → Grep 既存使用（0 件のため docs.rs で確認）

### テストリスト

ユニットテスト（該当なし）
- `#[instrument]` は属性マクロであり振る舞いを変えない。既存テストが validation を兼ねる

ハンドラテスト（該当なし）
- 既存ハンドラテストが通過すれば、instrument 追加による影響なし

API テスト（該当なし）

E2E テスト（該当なし）

### 実装パターン

**BFF ハンドラ:**

```rust
// パターン A: パラメータなし（logout, me, csrf, list_xxx 等）
#[utoipa::path(...)]
#[tracing::instrument(skip_all)]
pub async fn logout(State(state): State<Arc<AuthState>>, headers: HeaderMap, jar: CookieJar) -> impl IntoResponse { ... }

// パターン B: Path(i64)
#[utoipa::path(...)]
#[tracing::instrument(skip_all, fields(display_number))]
pub async fn get_workflow(..., Path(display_number): Path<i64>) -> ... { ... }

// パターン C: Path(Uuid)
#[utoipa::path(...)]
#[tracing::instrument(skip_all, fields(%definition_id))]
pub async fn get_workflow_definition(..., Path(definition_id): Path<Uuid>) -> ... { ... }

// パターン D: Path(StepPathParams)
#[utoipa::path(...)]
#[tracing::instrument(skip_all, fields(
    display_number = params.display_number,
    step_display_number = params.step_display_number,
))]
pub async fn approve_step(..., Path(params): Path<StepPathParams>, ...) -> ... { ... }
```

**Core Service ハンドラ:**

```rust
// パターン E: State + Query(TenantQuery)
#[tracing::instrument(skip_all)]
pub async fn list_users(State(state): State<Arc<UserState>>, Query(query): Query<TenantQuery>) -> ... { ... }

// パターン F: State + Path(Uuid)
#[tracing::instrument(skip_all, fields(%user_id))]
pub async fn get_user(State(state): State<Arc<UserState>>, Path(user_id): Path<Uuid>) -> ... { ... }

// パターン G: State + Path(StepPathParams) — Core の StepPathParams は id + step_id (UUID)
#[tracing::instrument(skip_all, fields(%params.id, %params.step_id))]
pub async fn approve_step(..., Path(params): Path<StepPathParams>, ...) -> ... { ... }

// パターン H: State + Path(StepByDisplayNumberPathParams)
#[tracing::instrument(skip_all, fields(
    display_number = params.display_number,
    step_display_number = params.step_display_number,
))]
pub async fn approve_step_by_display_number(..., Path(params): Path<StepByDisplayNumberPathParams>, ...) -> ... { ... }
```

**Auth Service ハンドラ:**

```rust
// パターン I: PII リクエスト → skip_all のみ
#[tracing::instrument(skip_all)]
pub async fn verify(State(state): State<Arc<AuthState>>, Json(req): Json<VerifyRequest>) -> ... { ... }

// パターン J: Path((Uuid, Uuid)) → 安全なID を記録
#[tracing::instrument(skip_all, fields(%tenant_id, %user_id))]
pub async fn delete_credentials(..., Path((tenant_id, user_id)): Path<(Uuid, Uuid)>) -> ... { ... }
```

### 対象ファイル一覧

BFF:
- `backend/apps/bff/src/handler/auth/login.rs` — login, logout
- `backend/apps/bff/src/handler/auth/session.rs` — me, csrf
- `backend/apps/bff/src/handler/user.rs` — list_users, create_user, get_user_detail, update_user, update_user_status
- `backend/apps/bff/src/handler/workflow/command.rs` — create_workflow, submit_workflow, approve_step, reject_step, request_changes_step, resubmit_workflow, post_comment
- `backend/apps/bff/src/handler/workflow/query.rs` — list_workflow_definitions, get_workflow_definition, list_my_workflows, get_workflow, get_task_by_display_numbers, list_comments
- `backend/apps/bff/src/handler/dashboard.rs` — get_dashboard_stats
- `backend/apps/bff/src/handler/role.rs` — list_roles, get_role, create_role, update_role, delete_role
- `backend/apps/bff/src/handler/task.rs` — list_my_tasks
- `backend/apps/bff/src/handler/audit_log.rs` — list_audit_logs

Core Service:
- `backend/apps/core-service/src/handler/auth/mod.rs` — list_users, get_user_by_email, get_user, create_user, get_user_by_display_number, update_user, update_user_status
- `backend/apps/core-service/src/handler/workflow/command.rs` — create_workflow, submit_workflow, approve_step, reject_step, request_changes_step, + by_display_number 系, resubmit_workflow, post_comment
- `backend/apps/core-service/src/handler/workflow/query.rs` — list_workflow_definitions, get_workflow_definition, list_my_workflows, get_workflow, get_workflow_by_display_number, list_comments
- `backend/apps/core-service/src/handler/dashboard.rs` — get_dashboard_stats
- `backend/apps/core-service/src/handler/role.rs` — list_roles, get_role, create_role, update_role, delete_role
- `backend/apps/core-service/src/handler/task.rs` — list_my_tasks, get_task, get_task_by_display_numbers

Auth Service:
- `backend/apps/auth-service/src/handler/auth.rs` — verify, create_credentials, delete_credentials

---

## Phase 2: BFF クライアント計装

### 確認事項

- [ ] パターン: Auth Service クライアントの関数シグネチャ → `backend/apps/bff/src/client/auth_service.rs`
- [ ] パターン: Core Service クライアントの関数シグネチャ → `backend/apps/bff/src/client/core_service/user_client.rs`（代表例）
- [ ] ライブラリ: `#[async_trait]` impl 内での `#[instrument]` 配置 → Grep 既存使用（0 件のため docs.rs で確認）

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### 実装パターン

```rust
#[async_trait]
impl AuthServiceClient for AuthServiceClientImpl {
    // PII パラメータ（password）あり → tenant_id, user_id のみ記録
    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id, %user_id))]
    async fn verify_password(&self, tenant_id: Uuid, user_id: Uuid, password: &str) -> ... { ... }
}

#[async_trait]
impl CoreServiceUserClient for CoreServiceClientImpl {
    // 全パラメータが安全
    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn list_users(&self, tenant_id: Uuid, status: Option<&str>) -> ... { ... }

    // email パラメータは PII → tenant_id のみ記録
    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn get_user_by_email(&self, tenant_id: Uuid, email: &str) -> ... { ... }

    // ID のみ
    #[tracing::instrument(skip_all, level = "debug", fields(%user_id))]
    async fn get_user(&self, user_id: Uuid) -> ... { ... }
}
```

### 対象ファイル一覧

- `backend/apps/bff/src/client/auth_service.rs` — verify_password, create_credentials
- `backend/apps/bff/src/client/core_service/user_client.rs` — list_users, get_user_by_email, get_user, create_user, update_user, update_user_status, get_user_by_display_number
- `backend/apps/bff/src/client/core_service/workflow_client.rs` — 全メソッド（~16 個）
- `backend/apps/bff/src/client/core_service/role_client.rs` — list_roles, get_role, create_role, update_role, delete_role
- `backend/apps/bff/src/client/core_service/task_client.rs` — list_my_tasks, get_task, get_dashboard_stats, get_task_by_display_numbers

---

## Phase 3: インフラ層計装（リポジトリ + セッション）

### 確認事項

- [ ] パターン: PostgresUserRepository の impl → `backend/crates/infra/src/repository/user_repository.rs`
- [ ] パターン: RedisSessionManager の impl → `backend/crates/infra/src/session.rs`
- [ ] パターン: DynamoDbAuditLogRepository の impl → `backend/crates/infra/src/repository/audit_log_repository.rs`
- [ ] 型: TenantId, UserId 等が Display を実装しているか → `backend/crates/domain/src/`（`%` プレフィックス使用のため）

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### 実装パターン

```rust
#[async_trait]
impl UserRepository for PostgresUserRepository {
    // ID パラメータ → 記録、PII（email）→ skip_all で除外
    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn find_by_email(&self, tenant_id: &TenantId, email: &Email) -> Result<Option<User>, InfraError> { ... }

    #[tracing::instrument(skip_all, level = "debug", fields(%id))]
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, InfraError> { ... }

    // エンティティ全体は skip（大きい + PII 含む可能性）
    #[tracing::instrument(skip_all, level = "debug")]
    async fn insert(&self, user: &User) -> Result<(), InfraError> { ... }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn find_all_active_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<User>, InfraError> { ... }
}

#[async_trait]
impl SessionManager for RedisSessionManager {
    // SessionData は PII 含む → skip_all
    #[tracing::instrument(skip_all, level = "debug")]
    async fn create(&self, data: &SessionData) -> Result<String, InfraError> { ... }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn get(&self, tenant_id: &TenantId, session_id: &str) -> Result<Option<SessionData>, InfraError> { ... }
}
```

### 対象ファイル一覧

リポジトリ:
- `backend/crates/infra/src/repository/user_repository.rs` — 全 PostgresUserRepository メソッド
- `backend/crates/infra/src/repository/workflow_instance_repository.rs` — 全メソッド
- `backend/crates/infra/src/repository/workflow_step_repository.rs` — 全メソッド
- `backend/crates/infra/src/repository/workflow_definition_repository.rs` — 全メソッド
- `backend/crates/infra/src/repository/workflow_comment_repository.rs` — 全メソッド
- `backend/crates/infra/src/repository/role_repository.rs` — 全メソッド
- `backend/crates/infra/src/repository/tenant_repository.rs` — 全メソッド
- `backend/crates/infra/src/repository/display_id_counter_repository.rs` — 全メソッド
- `backend/crates/infra/src/repository/audit_log_repository.rs` — 全 DynamoDbAuditLogRepository メソッド

セッション:
- `backend/crates/infra/src/session.rs` — 全 RedisSessionManager メソッド

---

## 検証

### Phase ごと

各 Phase 完了後: `cargo check` → `cargo test`（対象クレート）

### 全 Phase 完了後

1. `just check-all` — lint + test + API test + E2E test
2. 手動検証: `RUST_LOG=debug just dev-all` でリクエストを送信し、スパンのネスト構造を確認

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `skip_all` 使用時に `fields(display_number)` が正しく動作するか未検証 | 技術的前提 | tracing ドキュメントで `skip_all` + `fields()` の組み合わせが有効なことを確認。`fields(param_name)` はパラメータ名と同名のフィールドを Debug で記録する |
| 2回目 | Core Service の `StepPathParams` と BFF の `StepPathParams` でフィールド名が異なる | 未定義 | Core: `id`(Uuid) + `step_id`(Uuid)、BFF: `display_number`(i64) + `step_display_number`(i64)。パターン G/H として分離 |
| 3回目 | ハンドラに `#[utoipa::path]` がない関数もある（Core Service, Auth Service） | 既存手段の見落とし | BFF のみ `#[utoipa::path]` あり。Core/Auth は `#[instrument]` を直接関数の上に配置 |
| 4回目 | テストアプローチ: TDD 要件との整合 | 品質の向上 | `#[instrument]` は振る舞いを変えない属性マクロ。PII 非漏洩は Story 3 のテストで担保済み。新テスト不要と判断 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | ハンドラ（BFF ~27, Core ~20, Auth 3）、クライアント（~34）、リポジトリ（~70）、セッション（~10）を Phase 1-3 に配分。health_check は理由付きで除外 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各パターン（A-J）にコード例あり。skip/fields の判定ルールが明確 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | skip パターン、レベル、fields 記法、属性配置順序、テストアプローチの 5 判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象（ハンドラ/クライアント/リポジトリ/セッション）と対象外（health_check/ユースケース/ミドルウェア/スタブ/deletion）を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | `skip_all` + `fields()` の動作、`#[async_trait]` との併用、Path/Query 型の Debug 実装を確認済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Issue #654 の完了基準、Epic #648 の設計原則（AI エージェント向けログ設計）、Story #651 の PII マスキング方針と整合 |
