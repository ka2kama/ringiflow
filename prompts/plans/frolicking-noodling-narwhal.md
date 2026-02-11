# Story #429: ロール管理 API 実装計画

## Context

Issue #403 Phase 2-2 の Story #429 として、テナント管理者向けのロール管理 API を実装する。Story #428（ユーザー CRUD API）で確立したパターンを踏襲し、ロールの一覧・作成・編集・削除 API を Core Service と BFF に追加する。

**スコープ**:
- ロール一覧取得 API（システムロール + カスタムロール、ユーザー数付き）
- カスタムロール作成 API
- カスタムロール編集 API
- カスタムロール削除 API（使用中チェック付き）

**対象外**:
- ユーザーへのロール割り当て変更 API（Story #428 で `UpdateUserInput.role_name` として実装済み）
- フロントエンド（Story #431 で対応）

## 設計判断

### 1. RoleRepository の分離

現在 `UserRepository` にロール関連メソッド（`find_role_by_name`, `insert_user_role` 等）が混在している。新規の Role CRUD メソッドは別の `RoleRepository` トレイトに分離する。

理由: SRP に基づく。User のユースケースと Role のユースケースは独立した変更理由を持つ。

既存のユーザーロール関連メソッド（`insert_user_role`, `replace_user_roles`, `find_roles_for_users` 等）は UserRepository に残す（移動はスコープ外）。

### 2. 権限マッピング

ロール管理は `user:*` 権限の傘下とし、新しい権限リソースは追加しない。

| 操作 | 必要な権限 |
|------|---------|
| ロール一覧・詳細 | `user:read` |
| ロール作成 | `user:create` |
| ロール編集・削除 | `user:update` |

理由: 機能仕様書に「テナント管理者の権限チェックは `user:*` 権限が必要」と明記。新しいリソース権限を追加すると DB マイグレーション（シードデータ変更）が必要になり、スコープを超える。

### 3. ロール識別子

BFF の URL パスでは UUID（文字列表現）を使用する。ロールにはワークフローやユーザーのような `display_number` がない。管理者 API なので UUID で十分。

```
GET    /api/v1/roles
POST   /api/v1/roles
GET    /api/v1/roles/{role_id}
PATCH  /api/v1/roles/{role_id}
DELETE /api/v1/roles/{role_id}
```

### 4. system_admin ロールの非表示

機能仕様書: `system_admin` ロールはテナント管理画面に表示しない。リポジトリの SQL で除外する。

### 5. CoreServiceRoleClient サブトレイト

BFF クライアントの ISP パターンを踏襲し、新しい `CoreServiceRoleClient` サブトレイトを作成する。`CoreServiceClient` スーパートレイトに追加する。

## Phase 分解

### Phase 1: Domain Model（Role 更新メソッド）

#### 確認事項
- パターン: `User::with_name()`, `User::with_status()` → `backend/crates/domain/src/user.rs`

#### テストリスト
- [ ] `Role::with_name()` で名前と updated_at が更新される
- [ ] `Role::with_description()` で説明と updated_at が更新される
- [ ] `Role::with_permissions()` で権限と updated_at が更新される

#### 実装内容
`Role` に不変更新メソッドを追加:

```rust
pub fn with_name(&self, name: String, now: DateTime<Utc>) -> Self {
    Self { name, updated_at: now, ..self.clone() }
}

pub fn with_description(&self, description: Option<String>, now: DateTime<Utc>) -> Self {
    Self { description, updated_at: now, ..self.clone() }
}

pub fn with_permissions(&self, permissions: Vec<Permission>, now: DateTime<Utc>) -> Self {
    Self { permissions, updated_at: now, ..self.clone() }
}
```

### Phase 2: Repository（RoleRepository）

#### 確認事項
- 型: `Role`, `RoleId`, `Permission` → `domain/src/role.rs`
- パターン: 既存リポジトリ構造 → `infra/src/repository/user_repository.rs`
- パターン: 統合テスト → `infra/tests/user_repository_test.rs`
- ライブラリ: sqlx JSONB → Grep `permissions` in `user_repository.rs`
- RLS: roles テーブルポリシー → `tenant_id = current OR tenant_id IS NULL`

#### テストリスト
- [ ] `find_all_by_tenant_with_user_count` でシステムロール + テナントロールが取得できる
- [ ] `find_all_by_tenant_with_user_count` で system_admin が除外される
- [ ] `find_all_by_tenant_with_user_count` でユーザー数が正しく集計される
- [ ] `find_by_id` でロールが取得できる
- [ ] `find_by_id` で存在しない ID は None を返す
- [ ] `insert` でカスタムロールを作成できる
- [ ] `insert` でテナント内の同名ロールは重複エラー
- [ ] `update` でロール名・説明・権限を更新できる
- [ ] `delete` でカスタムロールを削除できる
- [ ] `count_users_with_role` でロールに割り当てられたユーザー数を返す

#### 実装内容

新しいファイル `backend/crates/infra/src/repository/role_repository.rs`:

```rust
#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn find_all_by_tenant_with_user_count(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<(Role, i64)>, InfraError>;

    async fn find_by_id(&self, id: &RoleId) -> Result<Option<Role>, InfraError>;

    async fn insert(&self, role: &Role) -> Result<(), InfraError>;

    async fn update(&self, role: &Role) -> Result<(), InfraError>;

    async fn delete(&self, id: &RoleId) -> Result<(), InfraError>;

    async fn count_users_with_role(&self, role_id: &RoleId) -> Result<i64, InfraError>;
}
```

`find_all_by_tenant_with_user_count` の SQL:

```sql
SELECT r.id, r.tenant_id, r.name, r.description, r.permissions,
       r.is_system, r.created_at, r.updated_at,
       COUNT(ur.id) as user_count
FROM roles r
LEFT JOIN user_roles ur ON r.id = ur.role_id AND ur.tenant_id = $1
WHERE (r.tenant_id = $1 OR r.is_system = true) AND r.name != 'system_admin'
GROUP BY r.id
ORDER BY r.is_system DESC, r.name ASC
```

注: RLS が有効な環境では `app.tenant_id` 設定で自動フィルタされるが、テスト（superuser 接続）では WHERE 句が必要。明示的条件で二重防御する。

テストファイル: `backend/crates/infra/tests/role_repository_test.rs`

`repository.rs` に `pub mod role_repository;` と re-export を追加。`just sqlx-prepare` を実行。

### Phase 3: Core Service（Usecase + Handler + Router）

#### 確認事項
- 型: `CoreError` バリアント → `core-service/src/error.rs`
- パターン: `UserUseCaseImpl` → `core-service/src/usecase/user.rs`
- パターン: Core Service ハンドラ → `core-service/src/handler/auth.rs`
- パターン: Core Service ルーター → `core-service/src/main.rs`

#### テストリスト
- [ ] `list_roles` でロール一覧が返る
- [ ] `get_role` で存在するロールが返る
- [ ] `get_role` で存在しないロールは 404
- [ ] `create_role` でカスタムロールが作成される
- [ ] `create_role` で名前重複は 409
- [ ] `create_role` で権限が空の場合は 400
- [ ] `update_role` でカスタムロールが更新される
- [ ] `update_role` でシステムロールは 400
- [ ] `update_role` で存在しないロールは 404
- [ ] `delete_role` でカスタムロールが削除される
- [ ] `delete_role` でシステムロールは 400
- [ ] `delete_role` でユーザー割り当てありは 409
- [ ] `delete_role` で存在しないロールは 404

#### 実装内容

**Usecase**: `backend/apps/core-service/src/usecase/role.rs`

```rust
pub struct RoleUseCaseImpl {
    role_repository: Arc<dyn RoleRepository>,
    clock: Arc<dyn Clock>,
}

pub struct CreateRoleInput {
    pub tenant_id: TenantId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

pub struct UpdateRoleInput {
    pub role_id: RoleId,
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}
```

ビジネスルール:
- 作成: ロール名テナント内一意チェック（`insert` の DB 制約 + アプリ層エラーハンドリング）
- 作成: 権限が空でないことを検証
- 編集: システムロールは編集拒否（`is_system` チェック）
- 削除: システムロールは削除拒否（`is_system` チェック）
- 削除: ユーザー割り当てチェック（`count_users_with_role`）

**Handler**: `backend/apps/core-service/src/handler/role.rs` を新規作成

```rust
pub struct RoleState {
    pub role_repository: Arc<dyn RoleRepository>,
    pub usecase: RoleUseCaseImpl,
}
```

ハンドラ: `list_roles`, `get_role`, `create_role`, `update_role`, `delete_role`

**Router**: `backend/apps/core-service/src/main.rs` に追加

```rust
.route("/internal/roles", get(list_roles).post(create_role))
.route("/internal/roles/{role_id}", get(get_role).patch(update_role).delete(delete_role))
.with_state(role_state)
```

### Phase 4: BFF Client（CoreServiceRoleClient）

#### 確認事項
- パターン: `CoreServiceUserClient` → `bff/src/client/core_service/user_client.rs`
- パターン: `CoreServiceClient` スーパートレイト → `bff/src/client/core_service/client_impl.rs`
- パターン: `handle_response` → `bff/src/client/core_service/response.rs`
- パターン: DTO 型 → `bff/src/client/core_service/types.rs`

#### テストリスト
確認事項: なし（HTTP クライアントの実装パターン踏襲、既存テストのスタブ更新のみ）

#### 実装内容

**新規ファイル**: `backend/apps/bff/src/client/core_service/role_client.rs`

```rust
#[async_trait]
pub trait CoreServiceRoleClient: Send + Sync {
    async fn list_roles(&self, tenant_id: Uuid)
        -> Result<ApiResponse<Vec<RoleItemDto>>, CoreServiceError>;

    async fn get_role(&self, role_id: Uuid, tenant_id: Uuid)
        -> Result<RoleDetailDto, CoreServiceError>;

    async fn create_role(&self, req: CreateRoleCoreRequest)
        -> Result<RoleDetailDto, CoreServiceError>;

    async fn update_role(&self, role_id: Uuid, req: UpdateRoleCoreRequest)
        -> Result<RoleDetailDto, CoreServiceError>;

    async fn delete_role(&self, role_id: Uuid, tenant_id: Uuid)
        -> Result<(), CoreServiceError>;
}
```

**DTO 型（types.rs に追加）**:

```rust
pub struct RoleItemDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system: bool,
    pub user_count: i64,
}

pub struct RoleDetailDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub struct CreateRoleCoreRequest {
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

pub struct UpdateRoleCoreRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}
```

**CoreServiceClient スーパートレイトに追加**:

```rust
pub trait CoreServiceClient:
    CoreServiceUserClient + CoreServiceWorkflowClient + CoreServiceTaskClient + CoreServiceRoleClient
{ }
```

**エラー型（error.rs に追加）**: `RoleNotFound` バリアント

**テストスタブ更新**: BFF ハンドラテスト、統合テストの `CoreServiceRoleClient` スタブを追加

### Phase 5: BFF Handler + Router + OpenAPI

#### 確認事項
- パターン: BFF `UserState` → `bff/src/handler/user.rs`
- パターン: BFF ルーター → `bff/src/main.rs`
- パターン: OpenAPI 仕様 → `openapi/openapi.yaml`

#### テストリスト
確認事項: BFF ハンドラテストは既存パターンを踏襲（認証テストが中心、ビジネスロジックは Core Service 側でテスト済み）

#### 実装内容

**新規ファイル**: `backend/apps/bff/src/handler/role.rs`

```rust
pub struct RoleState {
    pub core_service_client: Arc<dyn CoreServiceRoleClient>,
    pub session_manager: Arc<dyn SessionManager>,
}
```

ハンドラ: `list_roles`, `get_role`, `create_role`, `update_role`, `delete_role`

各ハンドラのフロー: セッション取得 → テナント ID 抽出 → Core Service 呼び出し → BFF レスポンス変換

**Router（main.rs）**:

```rust
// RoleState 初期化
let role_state = Arc::new(RoleState {
    core_service_client: core_service_client.clone(),
    session_manager: session_manager.clone(),
});

// 権限別ルートグループ
.merge(
    Router::new()
        .route("/api/v1/roles", get(list_roles))
        .route("/api/v1/roles/{role_id}", get(get_role))
        .layer(from_fn_with_state(role_read_authz, require_permission))
        .with_state(role_state.clone()),
)
.merge(
    Router::new()
        .route("/api/v1/roles", post(create_role))
        .layer(from_fn_with_state(role_create_authz, require_permission))
        .with_state(role_state.clone()),
)
.merge(
    Router::new()
        .route("/api/v1/roles/{role_id}", patch(update_role).delete(delete_role))
        .layer(from_fn_with_state(role_update_authz, require_permission))
        .with_state(role_state),
)
```

注: delete と update は同じ `user:update` 権限を使用するため、1つの merge ブロックにまとめる。

**OpenAPI 仕様更新**: `openapi/openapi.yaml` にロール管理エンドポイント 5 個を追加

**handler.rs re-export 更新**: `RoleState` と各ハンドラを re-export

## 検証方法

1. 各 Phase 完了後: `cargo test --package <対象パッケージ>`
2. Phase 2 完了後: `just sqlx-prepare` + `just test-rust-integration`
3. 全 Phase 完了後: `just check-all`（リント + テスト + API テスト）
4. OpenAPI 仕様: `redocly lint` が既存の warning 以外に新しいエラーを出さないこと

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `system_admin` ロールの非表示条件がリポジトリ SQL に必要 | 不完全なパス | `find_all_by_tenant_with_user_count` の WHERE 句に `r.name != 'system_admin'` を追加 |
| 1回目 | ユーザーロール割り当て API が Story #428 で実装済みか | 既存手段の見落とし | `UpdateUserInput.role_name` + `replace_user_roles` で対応済み。スコープから除外を明記 |
| 2回目 | `CoreServiceClient` スーパートレイトへの `CoreServiceRoleClient` 追加が必要 | 未定義 | Phase 4 に明記。ブランケット impl の修正含む |
| 2回目 | DELETE と PATCH を同じ権限グループにまとめられる | シンプルさ | Phase 5 のルーター設計で `patch(update_role).delete(delete_role)` を1つのルートに集約 |
| 3回目 | Role CRUD テスト用のフィクスチャ（テナント + ロール）の作成方法 | テストパターン | `#[sqlx::test(migrations)]` でシードデータ投入。テスト内でカスタムロール INSERT |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Issue #429 の全完了基準をカバー。ユーザーロール割り当ては #428 で実装済みを確認 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase のファイルパス、型名、SQL を具体的に記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | RoleRepository 分離、権限マッピング、ロール識別子、system_admin 非表示の4つの判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象外にユーザーロール割り当て API とフロントエンドを明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | RLS ポリシー（roles テーブル: tenant_id OR NULL）、UNIQUE 制約（tenant_id, name）を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 機能仕様書 4.6-4.9、権限マトリクス（セクション 5）と整合 |
