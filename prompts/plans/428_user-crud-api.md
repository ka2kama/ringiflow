# Story #428: ユーザー CRUD API

## Context

Phase 2-2（ユーザー管理 + 監査ログ）の Story #428。
Story #427 で認可ミドルウェアが完成したため、その基盤の上にユーザー管理の CRUD API を構築する。

### 現状（As-Is）

- `UserRepository` は読み取り + `update_last_login` のみ
- Core Service のユーザー API は `list_users`（active のみ）、`get_user_by_email`、`get_user` の 3 つ
- BFF のユーザー API は `list_users` のみ（`WorkflowState` を使用）
- `AuthServiceClient` は `verify_password` のみ（`create_credentials` なし）
- `User` ドメインモデルに `with_name()` がない（名前更新不可）
- `RoleRepository` は存在しない（ロール検索は `find_with_roles` 内の JOIN のみ）

### 理想（To-Be）

- テナント管理者がユーザーを一覧・作成・編集・無効化/有効化できる
- ユーザー作成時に初期パスワードが自動生成され、Auth Service に登録される
- ステータスフィルタ付きユーザー一覧（active/inactive、deleted は除外）
- 自己無効化防止と最後のテナント管理者保護

## スコープ

### 対象

- Domain: `User::with_name()` メソッド
- Repository: UserRepository の CRUD メソッド追加 + ロール操作
- Core Service: ユーザー CRUD ハンドラ + UserUseCase
- BFF Client: CoreServiceUserClient + AuthServiceClient 拡張
- BFF Handler: ユーザー CRUD ハンドラ + ルーター更新
- 初期パスワード自動生成

### 対象外

- ロール CRUD（Story #429）
- フロントエンド（Story #431）
- 監査ログ（Story #430）
- パスワードリセット（別 Story）

## Phase 1: Domain Model 拡張

### 確認事項

- 型: `User` の定義 → `backend/crates/domain/src/user.rs:198`（確認済み）
- パターン: `with_status()`, `with_last_login_updated()` → 同ファイル（確認済み）

### 設計

`User` に `with_name()` メソッドを追加する。既存の `with_status()` と同じパターン。

```rust
/// ユーザー名を変更した新しいインスタンスを返す
pub fn with_name(self, name: UserName, now: DateTime<Utc>) -> Self {
    Self {
        name,
        updated_at: now,
        ..self
    }
}
```

### テストリスト

- [ ] `with_name()` で名前が変更され updated_at が更新される

### 変更ファイル

- `backend/crates/domain/src/user.rs` — `with_name()` メソッドとテスト追加

## Phase 2: Repository 拡張

### 確認事項

- 型: `UserRepository` トレイト → `backend/crates/infra/src/repository/user_repository.rs:28`（確認済み）
- パターン: `insert()` → `workflow_instance_repository.rs:171`（確認済み）
- パターン: sqlx クエリ → 既存の `find_by_email`, `find_by_id` 等（確認済み）
- 型: `InfraError` → `backend/crates/infra/src/error.rs`（確認済み）
- DB スキーマ: `users` テーブル（確認済み）、`user_roles` テーブル（確認済み）、`roles` テーブル（確認済み）

### 設計

`UserRepository` トレイトに以下のメソッドを追加する。

**ユーザー CRUD:**

```rust
/// ユーザーを挿入する
async fn insert(&self, user: &User) -> Result<(), InfraError>;

/// ユーザー情報を更新する（name, updated_at）
async fn update(&self, user: &User) -> Result<(), InfraError>;

/// ユーザーステータスを更新する
async fn update_status(&self, user: &User) -> Result<(), InfraError>;

/// 表示用連番でユーザーを検索する
async fn find_by_display_number(
    &self,
    tenant_id: &TenantId,
    display_number: DisplayNumber,
) -> Result<Option<User>, InfraError>;

/// テナント内のユーザー一覧を取得する（deleted 除外、ステータスフィルタ可）
async fn find_all_by_tenant(
    &self,
    tenant_id: &TenantId,
    status_filter: Option<UserStatus>,
) -> Result<Vec<User>, InfraError>;
```

**ユーザーロール操作:**

```rust
/// ユーザーにロールを割り当てる
async fn insert_user_role(
    &self,
    user_id: &UserId,
    role_id: &RoleId,
    tenant_id: &TenantId,
) -> Result<(), InfraError>;

/// ユーザーのロールを置き換える（既存削除 + 新規追加）
async fn replace_user_roles(
    &self,
    user_id: &UserId,
    role_id: &RoleId,
    tenant_id: &TenantId,
) -> Result<(), InfraError>;
```

**ロール検索（Story #429 で RoleRepository に移行予定）:**

```rust
/// ロール名でロールを検索する
async fn find_role_by_name(&self, name: &str) -> Result<Option<Role>, InfraError>;

/// テナント内の特定ロールを持つアクティブユーザー数をカウントする
async fn count_active_users_with_role(
    &self,
    tenant_id: &TenantId,
    role_name: &str,
    excluding_user_id: Option<&UserId>,
) -> Result<i64, InfraError>;
```

### 設計判断

**ロール操作を UserRepository に含める理由**: Story #429 で RoleRepository を導入する予定だが、現時点ではユーザー CRUD に必要な最小限のロール操作のみ。分離は Story #429 で行う。

**`find_all_active_by_tenant` との関係**: 既存メソッドは変更しない。承認者選択用途で引き続き使用する。管理者向けユーザー一覧には新しい `find_all_by_tenant` を使用する。

**`update` と `update_status` の分離**: ユーザー情報更新とステータス変更は異なるユースケース（異なる権限、異なるバリデーション）のため、メソッドを分離する。

### テストリスト

- [ ] insert でユーザーを挿入し find_by_id で取得できる
- [ ] find_by_display_number でテナント内のユーザーを検索できる
- [ ] find_all_by_tenant でステータスフィルタが機能する（active のみ、inactive のみ、フィルタなし）
- [ ] find_all_by_tenant で deleted ユーザーは除外される
- [ ] update でユーザー名が更新される
- [ ] update_status でステータスが更新される
- [ ] insert_user_role でロールを割り当てられる
- [ ] replace_user_roles でロールが置き換わる
- [ ] find_role_by_name でシステムロールを検索できる
- [ ] count_active_users_with_role が正しくカウントする

### 変更ファイル

- `backend/crates/infra/src/repository/user_repository.rs` — トレイト + 実装追加
- `backend/crates/infra/tests/user_repository_test.rs` — 新規作成（統合テスト）

## Phase 3: Core Service ユーザー CRUD

### 確認事項

- 型: `UserState` → `backend/apps/core-service/src/handler/auth.rs:32`（確認済み）
- パターン: `WorkflowUseCaseImpl` → `backend/apps/core-service/src/usecase/workflow/command.rs`（確認済み）
- パターン: `CoreError` → `backend/apps/core-service/src/error.rs`（確認済み）
- 型: `DisplayIdCounterRepository::next_display_number` → `backend/crates/infra/src/repository/display_id_counter_repository.rs:41`（確認済み）

### 設計

#### UserUseCase

新しいユースケースを作成する。ワークフローの `WorkflowUseCaseImpl` と同じパターン。

```rust
// backend/apps/core-service/src/usecase/user.rs

pub struct UserUseCaseImpl {
    user_repository: Arc<dyn UserRepository>,
    display_id_counter_repository: Arc<dyn DisplayIdCounterRepository>,
    clock: Arc<dyn Clock>,
}

impl UserUseCaseImpl {
    /// ユーザーを作成する
    /// 1. display_number 採番
    /// 2. User ドメインオブジェクト作成
    /// 3. users テーブルに挿入
    /// 4. user_roles テーブルにロール割り当て
    pub async fn create_user(&self, input: CreateUserInput) -> Result<(User, String), CoreError>;

    /// ユーザー情報を更新する（名前、ロール）
    pub async fn update_user(&self, input: UpdateUserInput) -> Result<User, CoreError>;

    /// ユーザーステータスを変更する
    /// - 自己無効化防止（requester_id == target_id のチェック）
    /// - 最後のテナント管理者保護
    pub async fn update_user_status(&self, input: UpdateUserStatusInput) -> Result<User, CoreError>;
}
```

入力型:

```rust
pub struct CreateUserInput {
    pub tenant_id: TenantId,
    pub email: Email,
    pub name: UserName,
    pub role_name: String,
}

pub struct UpdateUserInput {
    pub user_id: UserId,
    pub name: Option<UserName>,
    pub role_name: Option<String>,
}

pub struct UpdateUserStatusInput {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub status: UserStatus,
    pub requester_id: UserId,
}
```

#### Core Service ハンドラ

`UserState` を拡張して `UserUseCaseImpl` を持たせる。

```rust
pub struct UserState {
    pub user_repository: Arc<dyn UserRepository>,
    pub tenant_repository: Arc<dyn TenantRepository>,
    pub usecase: UserUseCaseImpl,  // 追加
}
```

新規ハンドラ:

```rust
/// POST /internal/users — ユーザー作成
pub async fn create_user(
    State(state): State<Arc<UserState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, CoreError>;

/// PATCH /internal/users/{user_id} — ユーザー情報更新
pub async fn update_user(
    State(state): State<Arc<UserState>>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, CoreError>;

/// PATCH /internal/users/{user_id}/status — ステータス変更
pub async fn update_user_status(
    State(state): State<Arc<UserState>>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> Result<impl IntoResponse, CoreError>;

/// GET /internal/users/by-display-number/{display_number} — 表示用連番で取得
pub async fn get_user_by_display_number(
    State(state): State<Arc<UserState>>,
    Path(display_number): Path<i64>,
    Query(query): Query<TenantQuery>,
) -> impl IntoResponse;
```

`list_users` ハンドラを拡張: `TenantQuery` に `status` フィールドを追加。

```rust
#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
    pub status: Option<String>,  // 追加
}
```

`UserItemDto` にステータスとロール名を追加:

```rust
pub struct UserItemDto {
    pub id: Uuid,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub status: String,     // 追加
    pub roles: Vec<String>, // 追加
}
```

ロール情報の取得: `find_all_by_tenant` で取得したユーザーの ID 群に対して `find_with_roles` を使うと N+1 問題が発生する。代わりに、一括で user_roles + roles を JOIN して取得するクエリをリポジトリに追加する。

```rust
/// ユーザー ID のリストに対応するロール名を一括取得する
async fn find_roles_for_users(
    &self,
    user_ids: &[UserId],
    tenant_id: &TenantId,
) -> Result<HashMap<UserId, Vec<String>>, InfraError>;
```

#### Core Service ルーター更新

```rust
// 既存
.route("/internal/users", get(list_users))
// 追加
.route("/internal/users", get(list_users).post(create_user))
.route("/internal/users/{user_id}", get(get_user).patch(update_user))
.route("/internal/users/{user_id}/status", patch(update_user_status))
.route(
    "/internal/users/by-display-number/{display_number}",
    get(get_user_by_display_number),
)
```

### テストリスト

- [ ] create_user: 正常系（ユーザー作成 + ロール割り当て + display_number 採番）
- [ ] create_user: メールアドレス重複で 400
- [ ] create_user: 存在しないロール名で 400
- [ ] update_user: 名前変更が成功する
- [ ] update_user: ロール変更が成功する
- [ ] update_user: 存在しないユーザーで 404
- [ ] update_user_status: 無効化が成功する
- [ ] update_user_status: 有効化が成功する
- [ ] update_user_status: 自己無効化で 400
- [ ] update_user_status: 最後のテナント管理者の無効化で 400
- [ ] list_users: ステータスフィルタが機能する
- [ ] get_user_by_display_number: 正常系

### 変更ファイル

- `backend/apps/core-service/src/usecase/user.rs` — 新規作成
- `backend/apps/core-service/src/usecase.rs` — user モジュール追加
- `backend/apps/core-service/src/handler/auth.rs` — ハンドラ追加 + UserState 拡張 + リクエスト/レスポンス型
- `backend/apps/core-service/src/main.rs` — ルーター更新 + UserState 初期化
- `backend/crates/infra/src/repository/user_repository.rs` — `find_roles_for_users` 追加

## Phase 4: BFF Client 拡張

### 確認事項

- 型: `CoreServiceUserClient` トレイト → `backend/apps/bff/src/client/core_service/user_client.rs:16`（確認済み）
- 型: `AuthServiceClient` トレイト → `backend/apps/bff/src/client/auth_service.rs:67`（確認済み）
- パターン: `handle_response` → `backend/apps/bff/src/client/core_service/response.rs:17`（確認済み）
- パターン: `CoreServiceError` → `backend/apps/bff/src/client/core_service/error.rs`（確認済み）

### 設計

#### CoreServiceUserClient 拡張

```rust
/// ユーザーを作成する
async fn create_user(
    &self,
    tenant_id: Uuid,
    req: &CreateUserCoreRequest,
) -> Result<ApiResponse<CreateUserCoreResponse>, CoreServiceError>;

/// ユーザー情報を更新する
async fn update_user(
    &self,
    user_id: Uuid,
    req: &UpdateUserCoreRequest,
) -> Result<ApiResponse<UserResponse>, CoreServiceError>;

/// ユーザーステータスを変更する
async fn update_user_status(
    &self,
    user_id: Uuid,
    req: &UpdateUserStatusCoreRequest,
) -> Result<ApiResponse<UserResponse>, CoreServiceError>;

/// 表示用連番でユーザーを取得する
async fn get_user_by_display_number(
    &self,
    tenant_id: Uuid,
    display_number: i64,
) -> Result<ApiResponse<UserWithPermissionsData>, CoreServiceError>;
```

`list_users` の既存メソッドを拡張: `status` クエリパラメータを追加。

#### AuthServiceClient 拡張

```rust
/// 認証情報を作成する
async fn create_credentials(
    &self,
    tenant_id: Uuid,
    user_id: Uuid,
    credential_type: &str,
    credential_data: &str,
) -> Result<CreateCredentialsResponse, AuthServiceError>;
```

`AuthServiceError` に `BadRequest` バリアントを追加:

```rust
/// リクエストエラー（400）
#[error("リクエストエラー: {0}")]
BadRequest(String),
```

### テストリスト

- [ ] create_credentials の正常系
- [ ] create_credentials のエラーハンドリング

### 変更ファイル

- `backend/apps/bff/src/client/core_service/user_client.rs` — 新メソッド追加
- `backend/apps/bff/src/client/core_service/types.rs` — リクエスト/レスポンス型追加
- `backend/apps/bff/src/client/auth_service.rs` — `create_credentials` 追加
- `backend/apps/bff/src/client/core_service/error.rs` — `EmailAlreadyExists` バリアント追加（409 対応）

## Phase 5: BFF Handler + Router

### 確認事項

- パターン: BFF ハンドラ → `backend/apps/bff/src/handler/user.rs`（確認済み）
- パターン: 認可ミドルウェア適用 → `backend/apps/bff/src/main.rs:231`（確認済み）
- パターン: `get_session`, `extract_tenant_id` → `backend/apps/bff/src/error.rs`（確認済み）
- ライブラリ: `rand` クレート → `Cargo.toml:63`（`rand = "0.9"` 確認済み）

### 設計

#### 初期パスワード生成

BFF 層で初期パスワードを生成する。Auth Service はパスワードハッシュのみ管理するため、平文パスワードの生成は BFF の責務。

```rust
// backend/apps/bff/src/handler/user.rs 内に定義

/// 初期パスワードを生成する（16文字、英数字 + 特殊文字）
fn generate_initial_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%&*";
    let mut rng = rand::rng();
    (0..16)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
```

#### UserState（BFF）

```rust
pub struct UserState {
    pub core_service_client: Arc<dyn CoreServiceUserClient>,
    pub auth_service_client: Arc<dyn AuthServiceClient>,
    pub session_manager: Arc<dyn SessionManager>,
}
```

既存の `list_users` は `WorkflowState` から `UserState` に移行する。

#### ハンドラ

```rust
/// POST /api/v1/users — ユーザー作成
/// フロー:
/// 1. 入力バリデーション
/// 2. 初期パスワード生成
/// 3. Core Service でユーザー作成
/// 4. Auth Service で認証情報作成
/// 5. 初期パスワード付きレスポンス返却
pub async fn create_user(...) -> impl IntoResponse;

/// GET /api/v1/users/{display_number} — ユーザー詳細
pub async fn get_user(...) -> impl IntoResponse;

/// PATCH /api/v1/users/{display_number} — ユーザー更新
pub async fn update_user(...) -> impl IntoResponse;

/// PATCH /api/v1/users/{display_number}/status — ステータス変更
/// セッションから自身の user_id を取得し、自己無効化チェックを BFF 側でも行う
pub async fn update_user_status(...) -> impl IntoResponse;
```

`list_users` を拡張: ステータスフィルタのクエリパラメータ対応。

BFF レスポンス型:

```rust
/// ユーザー作成レスポンス（初期パスワード付き）
#[derive(Debug, Serialize)]
pub struct CreateUserResponseData {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub role: String,
    pub initial_password: String,
}

/// ユーザー詳細レスポンス
#[derive(Debug, Serialize)]
pub struct UserDetailData {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub roles: Vec<String>,
    pub created_at: String,
    pub last_login_at: Option<String>,
}
```

`UserItemData` にステータスとロールを追加:

```rust
pub struct UserItemData {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub status: String,     // 追加
    pub roles: Vec<String>, // 追加
}
```

#### ルーター

```rust
// BFF UserState 作成
let user_state = Arc::new(handler::UserState {
    core_service_client: core_service_client.clone(),
    auth_service_client: auth_service_client.clone(),
    session_manager: session_manager.clone(),
});

// 認可ステート
let user_read_authz = AuthzState {
    session_manager: session_manager.clone(),
    required_permission: "user:read".to_string(),
};
let user_create_authz = AuthzState {
    session_manager: session_manager.clone(),
    required_permission: "user:create".to_string(),
};
let user_update_authz = AuthzState {
    session_manager: session_manager.clone(),
    required_permission: "user:update".to_string(),
};

// 管理者 API（権限別ルートグループ）
.merge(
    Router::new()
        .route("/api/v1/users", get(list_users))
        .route("/api/v1/users/{display_number}", get(get_user_detail))
        .layer(from_fn_with_state(user_read_authz, require_permission))
        .with_state(user_state.clone()),
)
.merge(
    Router::new()
        .route("/api/v1/users", post(create_user))
        .layer(from_fn_with_state(user_create_authz, require_permission))
        .with_state(user_state.clone()),
)
.merge(
    Router::new()
        .route("/api/v1/users/{display_number}", patch(update_user))
        .route("/api/v1/users/{display_number}/status", patch(update_user_status))
        .layer(from_fn_with_state(user_update_authz, require_permission))
        .with_state(user_state),
)
```

### 設計判断

**パスワード生成を BFF に配置する理由**: アーキテクチャ上、BFF は Auth Service と Core Service の両方を知るオーケストレーター。Core Service は認証情報を扱わない設計。BFF で平文パスワードを生成し、Auth Service にハッシュ化を委譲する。

**権限別ルートグループ**: RBAC の正確な適用のため、GET/POST/PATCH で異なる権限を要求する。現時点では tenant_admin（`user:*`）のみが利用するが、将来のカスタムロール対応を見据えた設計。

**自己無効化チェックの二重実施**: BFF（セッション情報で早期リジェクト）と Core Service（ビジネスルールとして）の両方で行う。防御的プログラミング。

### テストリスト

- [ ] create_user: 正常系（初期パスワードが返却される）
- [ ] create_user: バリデーションエラー（メール形式不正）
- [ ] get_user: 表示用連番で詳細を取得できる
- [ ] update_user: 名前変更が成功する
- [ ] update_user_status: 無効化が成功する
- [ ] update_user_status: 自己無効化で 400
- [ ] list_users: ステータスフィルタが機能する

### 変更ファイル

- `backend/apps/bff/src/handler/user.rs` — ハンドラ追加 + UserState 定義 + パスワード生成
- `backend/apps/bff/src/handler.rs` — re-export 追加
- `backend/apps/bff/src/main.rs` — ルーター更新 + UserState 初期化
- `backend/apps/bff/Cargo.toml` — `rand` 依存追加

## 検証方法

1. `just check` — コンパイル + ユニットテスト
2. `just check-all` — リント + テスト + API テスト
3. `just sqlx-prepare` — SQLx キャッシュ更新（リポジトリ変更後）
4. ユニットテスト: 各 Phase のテストリストが全て Green

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ユーザー一覧でロール情報が必要だが、N+1 問題が発生する | 不完全なパス | `find_roles_for_users` バッチクエリを追加し、Phase 3 のリポジトリ変更に含めた |
| 2回目 | `list_users` が `WorkflowState` を使用しており、新しい `UserState` への移行が必要 | 既存手段の見落とし | BFF の `list_users` を新しい `UserState` に移行する設計に変更 |
| 3回目 | AuthServiceClient に `create_credentials` がなく、BFF からの認証情報作成が不可能 | 未定義 | Phase 4 で `AuthServiceClient::create_credentials` を追加 |
| 4回目 | 自己無効化チェックを Core Service のみで行う設計だったが、BFF にセッション情報がある | 不完全なパス | BFF と Core Service の両方でチェックする防御的設計に変更 |
| 5回目 | `UserItemDto` にステータスとロール情報がなく、管理者一覧の要件を満たさない | 不完全なパス | `UserItemDto` にフィールドを追加し、ロール情報の一括取得クエリを追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Domain, Repository, Core Service, BFF Client, BFF Handler, Router の6レイヤーすべてカバー。機能仕様書 ADM-002〜005 の全操作を網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase で具体的なメソッドシグネチャとデータフローを記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | パスワード生成場所、ロール操作の配置、権限分離、自己無効化チェック位置の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象外にロール CRUD、フロントエンド、監査ログ、パスワードリセットを明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | RLS（`app.tenant_id` GUC）、sqlx コンパイル時検証、axum layer 適用範囲を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 機能仕様書 02_ユーザー管理.md の ADM-002〜005、AUTHZ-001 と整合 |
