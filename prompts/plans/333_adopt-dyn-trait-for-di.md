# #333 リポジトリ DI に動的ディスパッチ（dyn Trait）を採用する

## Context

### 問題

現在のバックエンドはリポジトリの DI に静的ディスパッチ（ジェネリクス）を使用している。これにより「ジェネリクス汚染」が発生し、型パラメータがリポジトリ層 → UseCase → Handler → main.rs へと伝播している。

具体的な問題:
- `WorkflowUseCaseImpl<D, I, S, U, C>` — 5 個の型パラメータ
- `TaskUseCaseImpl<I, S, U>` — 3 個
- core-service の main.rs に 20 個以上の turbofish アノテーション
- BFF の main.rs にも 20 個近い turbofish アノテーション

### 目的

`Arc<dyn Trait>` パターンに移行し、アプリケーションコードからジェネリクスを除去する。

### 完了基準（Issue #333 より）

- [ ] core-service ハンドラ: 型パラメータ除去
- [ ] auth-service ハンドラ: 型パラメータ除去
- [ ] bff ハンドラ: 型パラメータ除去
- [ ] UseCase 層が `Arc<dyn Repository>` を使用
- [ ] main.rs のルート登録に明示的な型指定が不要
- [ ] テストでのモック注入が引き続き動作
- [ ] `just check-all` が通る

## 事前確認

実装前に以下を確認する:

1. 全リポジトリトレイトに `#[async_trait]` が付与されているか（`dyn Trait` のオブジェクト安全性に必須）
2. 全リポジトリトレイトに `Send + Sync` バウンドがあるか（確認済み）
3. BFF の `CoreServiceClient`, `AuthServiceClient` トレイトに `#[async_trait]` が付与されているか
4. `SessionManager`, `PasswordChecker` トレイトに `#[async_trait]` が付与されているか

## 設計方針

### 各サービスのアプローチ

| サービス | UseCase 層 | Handler 層 |
|---------|-----------|------------|
| Core Service | `*UseCaseImpl` のフィールドを `Arc<dyn Repo>` に変更 | State 構造体が具象 UseCaseImpl を保持 |
| Auth Service | `AuthUseCaseImpl` のフィールドを `Arc<dyn Repo>` に変更 | `AuthUseCase` トレイトを維持し `Arc<dyn AuthUseCase>` で保持 |
| BFF | UseCase 層なし | State 構造体が `Arc<dyn Trait>` を保持 |

Auth Service で `AuthUseCase` トレイトを維持する理由:
- 既にハンドラテストが `StubAuthUseCase` を使用しており、テストパターンを保持できる
- ハンドラが UseCase 実装から疎結合のままになる

### `resolve_user_names` ヘルパー

```rust
// Before
pub(crate) async fn resolve_user_names(
   user_repo: &impl UserRepository, ...
// After
pub(crate) async fn resolve_user_names(
   user_repo: &dyn UserRepository, ...
```

呼び出し側: `resolve_user_names(self.user_repo.as_ref(), ...)` （`Arc::as_ref()` で `&dyn Trait` を取得）

### main.rs での共有パターン

`Arc` を使うことで、同一リポジトリインスタンスを複数の State で共有可能:

```rust
let user_repo: Arc<dyn UserRepository> = Arc::new(PostgresUserRepository::new(pool.clone()));
// workflow_usecase, task_usecase, user_state で共有
```

### `WorkflowUseCase` トレイト（dead_code）の扱い

`usecase.rs` に `#[allow(dead_code)]` で定義されている `WorkflowUseCase` トレイトと blanket impl は、この変更で不要になるため削除する。

## Phase 1: Core Service

最も変更量が大きいサービス。UseCase → Handler → main.rs の順に変換する。

### 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/core-service/src/usecase/workflow.rs` | `WorkflowUseCaseImpl` のジェネリクス除去 → `Arc<dyn Repo>` |
| `backend/apps/core-service/src/usecase/task.rs` | `TaskUseCaseImpl` のジェネリクス除去 → `Arc<dyn Repo>` |
| `backend/apps/core-service/src/usecase/dashboard.rs` | `DashboardUseCaseImpl` のジェネリクス除去 → `Arc<dyn Repo>` |
| `backend/apps/core-service/src/usecase.rs` | `resolve_user_names` の引数変更、`WorkflowUseCase` トレイト削除 |
| `backend/apps/core-service/src/handler/workflow.rs` | `WorkflowState` のジェネリクス除去、全ハンドラ関数の非ジェネリック化 |
| `backend/apps/core-service/src/handler/task.rs` | `TaskState` のジェネリクス除去 |
| `backend/apps/core-service/src/handler/dashboard.rs` | `DashboardState` のジェネリクス除去 |
| `backend/apps/core-service/src/handler/auth.rs` | `UserState` のジェネリクス除去 → `Arc<dyn Repo>` 保持 |
| `backend/apps/core-service/src/handler.rs` | re-export の更新（型パラメータ不要に） |
| `backend/apps/core-service/src/main.rs` | turbofish 除去、`Arc<dyn Repo>` での初期化 |

### 変換パターン

#### UseCase 層

```rust
// Before
pub struct WorkflowUseCaseImpl<D, I, S, U, C> {
   definition_repo: D,
   instance_repo:   I,
   step_repo:       S,
   user_repo:       U,
   counter_repo:    C,
}

impl<D, I, S, U, C> WorkflowUseCaseImpl<D, I, S, U, C>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
   U: UserRepository,
   C: DisplayIdCounterRepository,
{
   pub fn new(definition_repo: D, instance_repo: I, ...) -> Self { ... }
}

// After
pub struct WorkflowUseCaseImpl {
   definition_repo: Arc<dyn WorkflowDefinitionRepository>,
   instance_repo:   Arc<dyn WorkflowInstanceRepository>,
   step_repo:       Arc<dyn WorkflowStepRepository>,
   user_repo:       Arc<dyn UserRepository>,
   counter_repo:    Arc<dyn DisplayIdCounterRepository>,
}

impl WorkflowUseCaseImpl {
   pub fn new(
      definition_repo: Arc<dyn WorkflowDefinitionRepository>,
      instance_repo: Arc<dyn WorkflowInstanceRepository>,
      step_repo: Arc<dyn WorkflowStepRepository>,
      user_repo: Arc<dyn UserRepository>,
      counter_repo: Arc<dyn DisplayIdCounterRepository>,
   ) -> Self { ... }
}
```

同様に `TaskUseCaseImpl`, `DashboardUseCaseImpl` も変換。

#### Handler 層

```rust
// Before
pub struct WorkflowState<D, I, S, U, C> {
   pub usecase: WorkflowUseCaseImpl<D, I, S, U, C>,
}
type AppState<D, I, S, U, C> = State<Arc<WorkflowState<D, I, S, U, C>>>;

pub async fn create_workflow<D, I, S, U, C>(
   State(state): AppState<D, I, S, U, C>,
   ...
) -> Response
where D: WorkflowDefinitionRepository, ...

// After
pub struct WorkflowState {
   pub usecase: WorkflowUseCaseImpl,
}

pub async fn create_workflow(
   State(state): State<Arc<WorkflowState>>,
   ...
) -> Response
```

#### UserState（リポジトリ直接保持）

```rust
// Before
pub struct UserState<R: UserRepository, T: TenantRepository> {
   pub user_repository:   R,
   pub tenant_repository: T,
}

// After
pub struct UserState {
   pub user_repository:   Arc<dyn UserRepository>,
   pub tenant_repository: Arc<dyn TenantRepository>,
}
```

#### main.rs

```rust
// Before
let user_repository = PostgresUserRepository::new(pool.clone());
let tenant_repository = PostgresTenantRepository::new(pool.clone());
let user_state = Arc::new(UserState { user_repository, tenant_repository });
...
.route("/internal/users", get(list_users::<PostgresUserRepository, PostgresTenantRepository>))

// After
let user_repo: Arc<dyn UserRepository> = Arc::new(PostgresUserRepository::new(pool.clone()));
let tenant_repo: Arc<dyn TenantRepository> = Arc::new(PostgresTenantRepository::new(pool.clone()));
let user_state = Arc::new(UserState {
   user_repository: user_repo.clone(),
   tenant_repository: tenant_repo,
});
...
.route("/internal/users", get(list_users))
```

#### テストの変換

UseCase テスト（workflow.rs, task.rs, dashboard.rs）:

```rust
// Before
let sut = WorkflowUseCaseImpl::new(
   definition_repo, instance_repo, step_repo, MockUserRepository, MockDisplayIdCounterRepository::new(),
);

// After
let sut = WorkflowUseCaseImpl::new(
   Arc::new(definition_repo),
   Arc::new(instance_repo),
   Arc::new(step_repo),
   Arc::new(MockUserRepository),
   Arc::new(MockDisplayIdCounterRepository::new()),
);
```

Handler テスト（auth.rs — UserState）:

```rust
// Before
let state = Arc::new(UserState {
   user_repository:   user_repo,
   tenant_repository: tenant_repo,
});
Router::new()
   .route("/internal/users/by-email", get(get_user_by_email::<StubUserRepository, StubTenantRepository>))

// After
let state = Arc::new(UserState {
   user_repository:   Arc::new(user_repo) as Arc<dyn UserRepository>,
   tenant_repository: Arc::new(tenant_repo) as Arc<dyn TenantRepository>,
});
Router::new()
   .route("/internal/users/by-email", get(get_user_by_email))
```

## Phase 2: Auth Service

### 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/auth-service/src/usecase/auth.rs` | `AuthUseCaseImpl` のジェネリクス除去 → `Arc<dyn Repo>` |
| `backend/apps/auth-service/src/usecase.rs` | `AuthUseCase` のblanket impl を具象 impl に変更 |
| `backend/apps/auth-service/src/handler/auth.rs` | `AuthState` のジェネリクス除去 → `Arc<dyn AuthUseCase>` |
| `backend/apps/auth-service/src/main.rs` | `Arc<dyn Repo>` での初期化 |

### 変換パターン

#### UseCase 層

```rust
// Before
pub struct AuthUseCaseImpl<R, P> {
   credentials_repository: R,
   password_checker: P,
}

// After
pub struct AuthUseCaseImpl {
   credentials_repository: Arc<dyn CredentialsRepository>,
   password_checker: Arc<dyn PasswordChecker>,
}
```

#### AuthUseCase トレイト impl

```rust
// Before
#[async_trait]
impl<R, P> AuthUseCase for AuthUseCaseImpl<R, P>
where
   R: CredentialsRepository + Send + Sync,
   P: PasswordChecker + Send + Sync,
{ ... }

// After
#[async_trait]
impl AuthUseCase for AuthUseCaseImpl { ... }
```

#### Handler 層

```rust
// Before
pub struct AuthState<U: AuthUseCase> {
   pub usecase: U,
}

pub async fn verify<U: AuthUseCase>(
   State(state): State<Arc<AuthState<U>>>, ...

// After
pub struct AuthState {
   pub usecase: Arc<dyn AuthUseCase>,
}

pub async fn verify(
   State(state): State<Arc<AuthState>>, ...
```

#### main.rs

```rust
// Before
let auth_usecase = AuthUseCaseImpl::new(credentials_repository, password_checker);
let auth_state = Arc::new(AuthState { usecase: auth_usecase });

// After
let credentials_repo: Arc<dyn CredentialsRepository> = Arc::new(PostgresCredentialsRepository::new(pool));
let password_checker: Arc<dyn PasswordChecker> = Arc::new(Argon2PasswordChecker::new());
let auth_usecase = AuthUseCaseImpl::new(credentials_repo, password_checker);
let auth_state = Arc::new(AuthState {
   usecase: Arc::new(auth_usecase),
});
```

#### テスト

UseCase テスト:
```rust
// Before
let sut = AuthUseCaseImpl::new(stub_repo, stub_checker);
// After
let sut = AuthUseCaseImpl::new(Arc::new(stub_repo), Arc::new(stub_checker));
```

Handler テスト（StubAuthUseCase は維持）:
```rust
// Before
let state = Arc::new(AuthState { usecase: StubAuthUseCase { ... } });
Router::new().route("/internal/auth/verify", post(verify::<StubAuthUseCase>))

// After
let state = Arc::new(AuthState { usecase: Arc::new(StubAuthUseCase { ... }) });
Router::new().route("/internal/auth/verify", post(verify))
```

## Phase 3: BFF

### 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/bff/src/handler/auth.rs` | `AuthState` のジェネリクス除去 → `Arc<dyn Trait>` |
| `backend/apps/bff/src/handler/workflow.rs` | `WorkflowState` のジェネリクス除去 → `Arc<dyn Trait>` |
| `backend/apps/bff/src/handler/task.rs` | ハンドラの非ジェネリック化 |
| `backend/apps/bff/src/handler/user.rs` | ハンドラの非ジェネリック化 |
| `backend/apps/bff/src/handler/dashboard.rs` | ハンドラの非ジェネリック化 |
| `backend/apps/bff/src/handler.rs` | re-export の更新 |
| `backend/apps/bff/src/middleware/csrf.rs` | `CsrfState` のジェネリクス除去、`csrf_middleware` の非ジェネリック化 |
| `backend/apps/bff/src/main.rs` | turbofish 除去、`Arc<dyn Trait>` での初期化 |

### 変換パターン

#### AuthState

```rust
// Before
pub struct AuthState<C: CoreServiceClient, A: AuthServiceClient, S: SessionManager> {
   pub core_service_client: C,
   pub auth_service_client: A,
   pub session_manager:     S,
}

// After
pub struct AuthState {
   pub core_service_client: Arc<dyn CoreServiceClient>,
   pub auth_service_client: Arc<dyn AuthServiceClient>,
   pub session_manager:     Arc<dyn SessionManager>,
}
```

#### WorkflowState

```rust
// Before
pub struct WorkflowState<C: CoreServiceClient, S: SessionManager> {
   pub core_service_client: C,
   pub session_manager:     S,
}

// After
pub struct WorkflowState {
   pub core_service_client: Arc<dyn CoreServiceClient>,
   pub session_manager:     Arc<dyn SessionManager>,
}
```

#### CsrfState + Middleware

```rust
// Before
pub struct CsrfState<S: SessionManager + Clone> {
   pub session_manager: S,
}
pub async fn csrf_middleware<S: SessionManager + Clone + 'static>(
   State(state): State<CsrfState<S>>, ...

// After
#[derive(Clone)]
pub struct CsrfState {
   pub session_manager: Arc<dyn SessionManager>,
}
pub async fn csrf_middleware(
   State(state): State<CsrfState>, ...
```

`Arc<dyn SessionManager>` は `Clone` を実装しているため、`CsrfState` に `#[derive(Clone)]` を付与すれば問題ない。

#### main.rs

```rust
// Before
let session_manager = RedisSessionManager::new(&config.redis_url).await...;
let core_service_client = CoreServiceClientImpl::new(&config.core_url);
let auth_service_client = AuthServiceClientImpl::new(&config.auth_url);
...
.route("/api/v1/auth/login", post(login::<CoreServiceClientImpl, AuthServiceClientImpl, RedisSessionManager>))

// After
let session_manager: Arc<dyn SessionManager> = Arc::new(RedisSessionManager::new(&config.redis_url).await...);
let core_service_client: Arc<dyn CoreServiceClient> = Arc::new(CoreServiceClientImpl::new(&config.core_url));
let auth_service_client: Arc<dyn AuthServiceClient> = Arc::new(AuthServiceClientImpl::new(&config.auth_url));
...
.route("/api/v1/auth/login", post(login))
```

#### テスト

```rust
// Before
let state = Arc::new(AuthState {
   core_service_client: StubCoreServiceClient { ... },
   auth_service_client: StubAuthServiceClient { ... },
   session_manager: StubSessionManager { ... },
});
post(login::<StubCoreServiceClient, StubAuthServiceClient, StubSessionManager>)

// After
let state = Arc::new(AuthState {
   core_service_client: Arc::new(StubCoreServiceClient { ... }),
   auth_service_client: Arc::new(StubAuthServiceClient { ... }),
   session_manager: Arc::new(StubSessionManager { ... }),
});
post(login)
```

## 対象外

- フロントエンド（Elm）: 変更なし
- OpenAPI 仕様: 変更なし（API の外部インタフェースは不変）
- DB スキーマ: 変更なし
- リポジトリトレイト定義自体: `Send + Sync` は既に付与済み

## 検証

各 Phase 完了後:
1. `just check` — リント + テスト
2. Phase 3 完了後: `just check-all` — リント + テスト + API テスト

最終確認:
- core-service, auth-service, bff の main.rs に turbofish が残っていないこと
- Handler 関数にジェネリクスパラメータが残っていないこと
- 全テストがパスすること

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → 全サービスの構造確認 | core-service, auth-service, bff の UseCase/Handler/main.rs のジェネリクスパターンを探索。テストパターンも確認 | auth-service は既に `AuthUseCase` トレイトをハンドラレベルで使用しており、core-service/bff とは異なるアプローチが必要。BFF の `CsrfState` も `Clone` 制約があり個別対応が必要 |
| 2回目 | Auth Service のアプローチ選択 | (A) AuthUseCase トレイト維持 + `Arc<dyn AuthUseCase>` vs (B) トレイト削除 + 具象 impl 直接保持 | (A) を採用。理由: ハンドラテスト（StubAuthUseCase）の変更が最小限。ハンドラと UseCase 実装の疎結合を維持 |
| 3回目 | `resolve_user_names` の引数型 | `&impl UserRepository` → `&dyn UserRepository` の妥当性を検証。`impl Trait` に `dyn UserRepository` を渡す場合の `Sized` 制約も検討 | `&dyn UserRepository` に変更が最もシンプル。呼び出し側は `self.user_repo.as_ref()` で `Arc` から `&dyn` を取得 |
| 4回目 | CsrfState の Clone 制約 | `dyn Trait` は Clone できないが、`Arc<dyn Trait>` は Clone 可能。`from_fn_with_state` が Clone を要求 | `CsrfState { session_manager: Arc<dyn SessionManager> }` + `#[derive(Clone)]` で対応。`Arc::clone` は安価 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全サービス・全レイヤーが計画に含まれている | OK | core-service（4 UseCase + 4 Handler + main.rs）、auth-service（1 UseCase + 1 Handler + main.rs）、bff（5 Handler + middleware + main.rs）をすべて列挙 |
| 2 | 曖昧さ排除 | 各ファイルの変換パターンが明確 | OK | Before/After のコードスニペットで変換パターンを具体化。テスト側の変換も明示 |
| 3 | 設計判断の完結性 | auth-service のアプローチ、CsrfState の Clone 対応等、全判断に理由あり | OK | Auth Service: AuthUseCase トレイト維持の理由を記載。CsrfState: Arc による Clone 対応を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 「対象外」セクションでフロントエンド、OpenAPI、DB スキーマ、トレイト定義自体を明示 |
| 5 | 技術的前提 | `#[async_trait]` の存在、`Send + Sync` バウンド | OK | 「事前確認」セクションで `#[async_trait]` 確認を必須ステップとして記載。`Send + Sync` は探索時に全トレイトで確認済み |
| 6 | 既存ドキュメント整合 | Issue #333 の完了基準と計画が一致 | OK | Issue の全チェックボックスに対応する変更が計画に含まれている |
