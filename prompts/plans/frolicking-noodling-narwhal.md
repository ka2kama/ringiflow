# Story #430: DynamoDB 基盤 + 監査ログ記録・閲覧 API 実装計画

## Context

Issue #403 Phase 2-2 の Story #430 として、DynamoDB をローカル開発環境に導入し、監査ログの記録・閲覧 API を実装する。Story #428（ユーザー CRUD）と #429（ロール管理）で実装した操作の監査ログを BFF レベルで記録し、テナント管理者が一覧・検索できる API を提供する。

**スコープ**:
- DynamoDB Local の導入（docker-compose）
- Rust AWS SDK（DynamoDB クライアント）の導入
- audit_logs テーブルの自動作成
- 監査ログ記録（ユーザー管理・ロール管理操作、成功時のみ）
- 監査ログ閲覧 API（一覧、フィルタ、カーソルベースページネーション）
- 認可ミドルウェアの適用（`user:read` 権限）

**対象外**:
- 認証イベント（login/logout）の監査ログ（後続 Story で対応）
- ワークフロー操作の監査ログ（後続 Story で対応）
- 失敗時の記録（成功時のみ。失敗ログは後続で拡張）
- `role.assign` の監査（user.update に含めて記録）
- `source_ip` の取得（`ConnectInfo` 導入は axum::serve 変更を伴うため先送り、現時点では None）
- GSI の追加（FilterExpression で十分なデータ量）
- テナント退会時の DynamoDB データ削除（Issue 化して追跡。TTL で1年後に自動削除）
- Terraform による本番 DynamoDB テーブル定義

## 設計判断

### 1. 記録場所: BFF ハンドラレベル

BFF が DynamoDB に直接書き込む。Core Service を経由しない。

理由:
- BFF はセッションデータ（actor_id, actor_name, tenant_id）を保持しており、監査ログに必要な全コンテキストがある
- Redis の SessionManager と同じパターン（BFF → データストア直接）
- Core Service への変更不要。Core Service のクライアントは BFF のみなので全操作がカバーされる

代替案: Core Service に DynamoDB 依存を追加し、BFF からアクターコンテキストをヘッダーで渡す → Core Service 側の変更が大きく、クライアントが BFF のみの現状では過剰。

### 2. DynamoDB テーブル設計

| 項目 | 値 |
|------|-----|
| テーブル名 | `audit_logs` |
| PK | `tenant_id` (String, UUID) |
| SK | `{ISO8601_timestamp}#{uuid}` (例: `2026-02-11T10:30:00.123Z#550e8400-...`) |
| TTL | `ttl` (Number, epoch seconds, created_at + 1年) |

SK 設計の根拠:
- ISO 8601 はレキシカル順でソート可能 → 時系列クエリに最適
- UUID サフィックスで同一ミリ秒のエントリも一意性を保証
- カーソルベースページネーションに SK 値をそのまま使用可能

### 3. ページネーション

DynamoDB ネイティブのカーソルベースページネーション。

- `LastEvaluatedKey` を base64 エンコードしてクライアントに返す（opaque cursor）
- クライアントは `cursor` パラメータで次ページを要求
- `ScanIndexForward=false`（新しい順）、デフォルト 50 件/ページ
- base64 エンコード/デコードはリポジトリ層に閉じ込め、BFF は opaque 文字列を透過

`PaginatedResponse<T>` を shared クレートに新設する（既存の `ApiResponse<T>` はページネーション非対応）。

### 4. 権限マッピング

監査ログ閲覧は `user:read` 権限を使用する。新しい権限リソースは追加しない。

理由: 機能仕様書で「テナント管理者のみ閲覧可能」と規定。tenant_admin ロールは全権限を持つ。`user:read` は tenant_admin のみが持つ権限であり、要件を満たす。新権限の追加はシードデータ変更を伴い、スコープを超える。

### 5. InfraError の DynamoDB バリアント

```rust
#[error("DynamoDB エラー: {0}")]
DynamoDb(String),
```

理由: AWS SDK のエラー型は `SdkError<ServiceError<E>>` でジェネリクスが深い。`#[from]` 自動変換が困難なため、手動で String にマップする。

### 6. 監査ログ記録の非ブロッキング

記録失敗時はレスポンスに影響させず、`tracing::error!` でログ出力のみ。

```rust
if let Err(e) = state.audit_log_repository.record(&audit_log).await {
    tracing::error!("監査ログ記録に失敗: {}", e);
}
```

理由: 監査ログの記録失敗がビジネスオペレーションを妨げるべきではない。

### 7. detail フィールドの内容

アクション別に記録する内容:

| アクション | detail の内容 |
|-----------|-------------|
| user.create | `{"email": "...", "name": "...", "role": "..."}` |
| user.update | `{"name": "...", "role_name": "..."}` (リクエスト内容) |
| user.deactivate | `{"user_name": "..."}` |
| user.activate | `{"user_name": "..."}` |
| role.create | `{"name": "...", "permissions": [...]}` |
| role.update | `{"name": "...", "permissions": [...]}` (リクエスト内容) |
| role.delete | `{"role_id": "..."}` |

変更前後の差分（before/after）は BFF では取得コスト高のため、リクエスト内容のみ記録する。

## Phase 分解

### Phase 1: DynamoDB インフラ基盤

#### 確認事項
- ライブラリ: `aws-sdk-dynamodb::Client` の生成パターン → docs.rs で確認（プロジェクト内に既存使用なし）
- ライブラリ: `aws-config` の `defaults(BehaviorVersion::latest())` → docs.rs で確認
- パターン: Redis クライアント初期化 → `infra/src/redis.rs`
- パターン: Docker Compose サービス定義 → `infra/docker/docker-compose.yaml`
- パターン: InfraError バリアント追加 → `infra/src/error.rs`
- パターン: CI サービス定義 → `.github/workflows/ci.yaml`
- 確認: DynamoDB Local の健全性チェック方法（curl 利用可否）

#### テストリスト
- [ ] DynamoDB Local コンテナが `just dev-deps` で起動する
- [ ] `create_dynamodb_client` がエンドポイントに接続できる
- [ ] `ensure_audit_log_table` が初回呼び出しでテーブルを作成する
- [ ] `ensure_audit_log_table` が既存テーブルに対して冪等に動作する（エラーにならない）

#### 実装内容

**Docker Compose** (`infra/docker/docker-compose.yaml`):
```yaml
dynamodb:
  image: amazon/dynamodb-local:latest
  ports:
    - "${DYNAMODB_PORT}:8000"
  command: ["-jar", "DynamoDBLocal.jar", "-sharedDb", "-inMemory"]
  restart: unless-stopped
```

API テスト用 (`infra/docker/docker-compose.api-test.yaml`): ポート 18001 で追加。

**Cargo 依存** (`backend/Cargo.toml` ワークスペース):
```toml
aws-config = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-dynamodb = "1"
base64 = "0.22"
```

**DynamoDB モジュール** (`backend/crates/infra/src/dynamodb.rs`):
```rust
pub async fn create_dynamodb_client(endpoint: &str, region: &str) -> Client { ... }
pub async fn ensure_audit_log_table(client: &Client, table_name: &str) -> Result<(), InfraError> { ... }
```

`ensure_audit_log_table` は DescribeTable で存在確認 → なければ CreateTable。冪等。

**環境変数追加**:
- `.env.template`: `DYNAMODB_PORT=18000`
- `backend/.env.template`: `DYNAMODB_ENDPOINT=http://localhost:${DYNAMODB_PORT}`
- `backend/.env.api-test`: `DYNAMODB_ENDPOINT=http://localhost:18001`

**CI** (`.github/workflows/ci.yaml`): rust-integration ジョブの services に DynamoDB Local 追加。

### Phase 2: ドメインモデル + リポジトリ

#### 確認事項
- 型: `TenantId::as_uuid()`, `UserId::as_uuid()` → `domain/src/tenant.rs`, `domain/src/user.rs`
- パターン: ドメインモデルの Newtype + enum パターン → `domain/src/role.rs`
- パターン: リポジトリトレイト定義 → `infra/src/repository/role_repository.rs`
- パターン: 統合テストのセットアップ → `infra/tests/role_repository_test.rs`
- ライブラリ: `aws_sdk_dynamodb::types::AttributeValue` の変換パターン → docs.rs
- ライブラリ: `base64::engine::general_purpose::STANDARD` → docs.rs
- 型: `ApiResponse<T>` の構造 → `shared/src/lib.rs`

#### テストリスト
- [ ] `AuditAction` の各バリアントが `.` 区切りの文字列に変換される（例: `UserCreate` → `"user.create"`）
- [ ] `AuditAction` が文字列からパースできる
- [ ] `AuditLog::new` が正しい TTL を計算する（created_at + 1年）
- [ ] `record` が監査ログを DynamoDB に書き込める
- [ ] `find_by_tenant` がテナント ID で検索でき、新しい順に返る
- [ ] `find_by_tenant` がカーソルベースページネーションで正しく動作する
- [ ] `find_by_tenant` が日付範囲フィルタで絞り込める
- [ ] `find_by_tenant` が `actor_id` フィルタで絞り込める
- [ ] `find_by_tenant` が `actions` フィルタ（複数選択）で絞り込める
- [ ] `find_by_tenant` が `result` フィルタで絞り込める
- [ ] 異なるテナントの監査ログが分離されている
- [ ] `PaginatedResponse` の `next_cursor` が null のとき最後のページを示す

#### 実装内容

**ドメインモデル** (`backend/crates/domain/src/audit_log.rs`):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditAction {
    UserCreate,
    UserUpdate,
    UserDeactivate,
    UserActivate,
    RoleCreate,
    RoleUpdate,
    RoleDelete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditResult {
    Success,
    Failure,
}

pub struct AuditLog {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub actor_id: UserId,
    pub actor_name: String,
    pub action: AuditAction,
    pub result: AuditResult,
    pub resource_type: String,
    pub resource_id: String,
    pub detail: Option<serde_json::Value>,
    pub source_ip: Option<String>,
    pub created_at: DateTime<Utc>,
    pub ttl: i64, // epoch seconds
}
```

AuditAction の文字列変換は `Display` + `FromStr` を手動実装（`strum` は `"user.create"` のような `.` 区切りに対応が難しいため）。

**PaginatedResponse** (`backend/crates/shared/src/lib.rs` に追加):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
}
```

**リポジトリ** (`backend/crates/infra/src/repository/audit_log_repository.rs`):

```rust
pub struct AuditLogFilter {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub actor_id: Option<UserId>,
    pub actions: Option<Vec<AuditAction>>,
    pub result: Option<AuditResult>,
}

pub struct AuditLogPage {
    pub items: Vec<AuditLog>,
    pub next_cursor: Option<String>,
}

#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn record(&self, log: &AuditLog) -> Result<(), InfraError>;
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        cursor: Option<&str>,
        limit: i32,
        filter: &AuditLogFilter,
    ) -> Result<AuditLogPage, InfraError>;
}
```

**DynamoDB 実装**: `DynamoDbAuditLogRepository`
- SK 形式: `{ISO8601_timestamp}#{uuid}`
- `record`: PutItem
- `find_by_tenant`: Query with `ScanIndexForward=false`, FilterExpression for actor_id/actions/result
- 日付範囲: KeyConditionExpression `sk BETWEEN :start AND :end`
- カーソル: `LastEvaluatedKey` を base64 エンコード/デコード

**統合テスト** (`backend/crates/infra/tests/audit_log_repository_test.rs`):
- テスト毎にランダムな `TenantId` を生成して分離（`#[sqlx::test]` のような自動ロールバックは不可）
- DynamoDB Local（`http://localhost:18000`）に接続

### Phase 3: BFF 監査ログ記録

#### 確認事項
- 型: `UserState`, `RoleState` のフィールド構成 → `bff/src/handler/user.rs`, `bff/src/handler/role.rs`
- パターン: BFF main.rs の依存関係初期化 → `bff/src/main.rs`
- パターン: `BffConfig::from_env()` → `bff/src/config.rs`
- パターン: BFF 統合テストスタブ → `bff/tests/auth_integration_test.rs`
- 型: `SessionData` のアクセサ → `infra/src/session.rs`

#### テストリスト
- [ ] `create_user` 成功時に `user.create` の監査ログが記録される
- [ ] `update_user` 成功時に `user.update` の監査ログが記録される
- [ ] `update_user_status`（inactive）で `user.deactivate` が記録される
- [ ] `update_user_status`（active）で `user.activate` が記録される
- [ ] `create_role` 成功時に `role.create` の監査ログが記録される
- [ ] `update_role` 成功時に `role.update` の監査ログが記録される
- [ ] `delete_role` 成功時に `role.delete` の監査ログが記録される
- [ ] 監査ログ記録失敗時にレスポンスが正常に返される（非ブロッキング）

#### 実装内容

**BffConfig** (`bff/src/config.rs`): `dynamodb_endpoint`, `aws_region` フィールド追加

**UserState** に `audit_log_repository: Arc<dyn AuditLogRepository>` 追加:
```rust
pub struct UserState {
    pub core_service_client: Arc<dyn CoreServiceUserClient>,
    pub auth_service_client: Arc<dyn AuthServiceClient>,
    pub session_manager: Arc<dyn SessionManager>,
    pub audit_log_repository: Arc<dyn AuditLogRepository>, // NEW
}
```

**RoleState** に同様に追加。

**各ハンドラの成功パスに記録コードを挿入**（例: create_user）:
```rust
Ok(core_response) => {
    let user_data = core_response.data;

    // 監査ログ記録
    let audit_log = AuditLog::new(
        *session_data.tenant_id(),
        *session_data.user_id(),
        session_data.name().to_string(),
        AuditAction::UserCreate,
        "user",
        user_data.id.to_string(),
        Some(serde_json::json!({
            "email": &user_data.email,
            "name": &user_data.name,
            "role": &user_data.role,
        })),
        None, // source_ip
    );
    if let Err(e) = state.audit_log_repository.record(&audit_log).await {
        tracing::error!("監査ログ記録に失敗: {}", e);
    }

    // レスポンス返却（既存コード）
    ...
}
```

**main.rs**: DynamoDB クライアント初期化 + `audit_log_repository` の生成 + 各 State への注入

**テストスタブ**: `auth_integration_test.rs` の `StubCoreServiceClient` に `AuditLogRepository` スタブを追加

### Phase 4: BFF 監査ログ閲覧 API + OpenAPI

#### 確認事項
- パターン: ルーターの認可ミドルウェア適用 → `bff/src/main.rs` の `user_read_authz` + `merge`
- パターン: OpenAPI エンドポイント定義 → `openapi/openapi.yaml` の `/api/v1/roles`
- パターン: ハンドラモジュールの re-export → `bff/src/handler.rs`
- 型: `AuthzState` の構成 → `bff/src/middleware.rs`

#### テストリスト
- [ ] `GET /api/v1/audit-logs` が監査ログ一覧を正しい JSON 形式で返す
- [ ] `limit` パラメータが結果数を制限する（デフォルト 50）
- [ ] `cursor` パラメータで次ページを取得できる
- [ ] `from`/`to` パラメータで日付範囲フィルタが動作する
- [ ] `actor_id` フィルタが動作する
- [ ] `action` フィルタが動作する（カンマ区切りで複数指定可）
- [ ] `result` フィルタが動作する
- [ ] 異なるテナントの監査ログが返されない
- [ ] `user:read` 権限なしで 403 が返る
- [ ] OpenAPI 仕様が redocly lint を通過する

#### 実装内容

**ハンドラ** (`backend/apps/bff/src/handler/audit_log.rs`):

```rust
pub struct AuditLogState {
    pub audit_log_repository: Arc<dyn AuditLogRepository>,
    pub session_manager: Arc<dyn SessionManager>,
}

#[derive(Debug, Deserialize)]
pub struct ListAuditLogsQuery {
    pub cursor: Option<String>,
    pub limit: Option<i32>,
    pub from: Option<String>,   // ISO 8601
    pub to: Option<String>,     // ISO 8601
    pub actor_id: Option<Uuid>,
    pub action: Option<String>, // カンマ区切り
    pub result: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogItemData {
    pub id: String,
    pub actor_id: String,
    pub actor_name: String,
    pub action: String,
    pub result: String,
    pub resource_type: String,
    pub resource_id: String,
    pub detail: Option<serde_json::Value>,
    pub source_ip: Option<String>,
    pub created_at: String,
}
```

**ルーター** (`bff/src/main.rs`):
```rust
let audit_log_state = Arc::new(AuditLogState {
    audit_log_repository: audit_log_repository.clone(),
    session_manager: session_manager.clone(),
});

let audit_read_authz = AuthzState {
    session_manager: session_manager.clone(),
    required_permission: "user:read".to_string(),
};

.merge(
    Router::new()
        .route("/api/v1/audit-logs", get(list_audit_logs))
        .layer(from_fn_with_state(audit_read_authz, require_permission))
        .with_state(audit_log_state),
)
```

**OpenAPI** (`openapi/openapi.yaml`):
- tags に `audit-logs` 追加
- `GET /api/v1/audit-logs` エンドポイント（クエリパラメータ: cursor, limit, from, to, actor_id, action, result）
- `AuditLogListResponse`（`PaginatedResponse<AuditLogItem>` に対応）スキーマ追加
- `AuditLogItem` スキーマ追加

## 暫定値・ワークアラウンドの影響パス

### DynamoDB Local の `-inMemory` モード

| パス | 影響 | 判定 |
|------|------|------|
| ユニットテスト（Mock） | DynamoDB 不使用 | OK |
| 統合テスト | テスト毎にランダム tenant_id で分離 | OK |
| API テスト | `-inMemory` でコンテナ再起動時にリセット | OK |
| 開発サーバー | コンテナ再起動でデータ消失 | 許容（監査ログは閲覧確認用） |

### ポート番号

| 環境 | DynamoDB ポート | 設定先 |
|------|----------------|--------|
| 開発 | 18000 | .env.template, docker-compose.yaml |
| API テスト | 18001 | .env.api-test, docker-compose.api-test.yaml |
| CI | services で直接起動（8000） | ci.yaml |

## 検証方法

1. 各 Phase 完了後: `cargo test --package <対象パッケージ>`
2. Phase 1 完了後: `just dev-deps` で DynamoDB Local が起動することを確認
3. Phase 2 完了後: `just test-rust-integration` で DynamoDB 統合テスト通過
4. Phase 4 完了後: `just check-all`（リント + テスト + API テスト）
5. OpenAPI 仕様: `redocly lint` が既存 warning 以外にエラーを出さないこと

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `PaginatedResponse` が `ApiResponse` と別型として必要 | 未定義 | shared クレートに `PaginatedResponse<T>` を新設 |
| 2回目 | 統合テストでの DynamoDB テスト分離戦略が未定義 | 不完全なパス | テスト毎にランダム `TenantId` を生成する方式を採用 |
| 3回目 | API テスト用 DynamoDB ポートが開発環境と衝突する可能性 | 競合・エッジケース | API テスト用は別ポート 18001 で DynamoDB を追加 |
| 4回目 | テナント退会時の DynamoDB データ削除が未設計 | 不完全なパス | Issue 化して追跡。TTL で自動削除されるため即時リスクは低い |
| 5回目 | `source_ip` 取得の ConnectInfo が `axum::serve` 変更を要求 | 技術的前提 | 先送り、現時点では None を記録 |
| 6回目 | `base64` 依存の配置先が曖昧 | 曖昧 | エンコード/デコードはリポジトリ層（infra クレート）に閉じ込め |
| 7回目 | CI の rust-integration ジョブに DynamoDB Local サービス追加が必要 | 未定義 | ci.yaml の services に追加 |
| 8回目 | `AuditLogFilter` に日付範囲フィルタが必要（機能仕様書の「期間」フィルタ） | 既存手段の見落とし | `from`, `to` フィールドを `AuditLogFilter` に追加。SK の KeyConditionExpression で実現 |
| 9回目 | `detail` フィールドの型: String vs serde_json::Value | シンプルさ | ドメインモデルでは `serde_json::Value`（構造化データ）、DynamoDB には JSON 文字列で格納 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | 完了基準5項目すべてに対応する Phase が存在。Docker/SDK/ドメイン/リポジトリ/BFF記録/BFF閲覧/OpenAPI を網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更ファイル・型定義・関数シグネチャが具体的。source_ip は「None」、失敗ログは「対象外」と明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 記録場所・テーブル設計・ページネーション・権限・エラー型・非ブロッキング・detail 内容の7判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象外セクションに auth/workflow/failure/source_ip/GSI/退会削除を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | DynamoDB Local の `-inMemory` 動作、ConnectInfo の axum::serve 変更、AWS SDK エラー型の複雑さ、base64 カーソル層を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 機能仕様書 03_監査ログ、data-store.md のテナント退会チェック、redis.rs / repository パターンとの一貫性を確認 |
