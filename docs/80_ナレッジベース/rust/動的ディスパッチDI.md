# 動的ディスパッチ DI（Arc<dyn Trait>）

## 概要

Rust で依存性注入（DI）を行う際のパターンの一つ。`Arc<dyn Trait>` を使って実行時にトレイトオブジェクトを注入する。

静的ディスパッチ（ジェネリクス）と比較して、型パラメータの伝播（ジェネリクス汚染）を防ぎ、アプリケーションコードをシンプルに保つ利点がある。

## 静的 vs 動的ディスパッチ

| 観点 | 静的（ジェネリクス） | 動的（`Arc<dyn Trait>`） |
|------|---------------------|-------------------------|
| パフォーマンス | 単相化（monomorphization）で最適 | vtable 経由の間接呼び出し |
| 型パラメータの伝播 | 使用箇所すべてに伝播 | なし |
| コンパイル時間 | 単相化でバイナリ膨張の可能性 | トレイトオブジェクトで1コピー |
| テスト | 型パラメータで Mock を注入 | `Arc::new(mock)` で注入 |
| 制約 | トレイト境界のみ | オブジェクト安全性が必要 |

### パフォーマンスの影響

vtable 経由の間接呼び出しによるオーバーヘッドはナノ秒単位。Web アプリケーションのリポジトリ層（DB I/O がミリ秒単位）では無視できる差異。

ライブラリやホットパス（毎秒数百万回呼ばれるコード）では静的ディスパッチが適切だが、アプリケーション層の DI には動的ディスパッチが実用的。

## 使い方

### トレイト定義

`#[async_trait]` と `Send + Sync` バウンドが必要:

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &UserId) -> anyhow::Result<Option<User>>;
}
```

`Send + Sync` はマルチスレッド環境（tokio ランタイム）で `Arc` を安全に共有するために必要。

### UseCase 層

```rust
pub struct WorkflowUseCaseImpl {
    definition_repo: Arc<dyn WorkflowDefinitionRepository>,
    instance_repo:   Arc<dyn WorkflowInstanceRepository>,
    user_repo:       Arc<dyn UserRepository>,
}

impl WorkflowUseCaseImpl {
    pub fn new(
        definition_repo: Arc<dyn WorkflowDefinitionRepository>,
        instance_repo:   Arc<dyn WorkflowInstanceRepository>,
        user_repo:       Arc<dyn UserRepository>,
    ) -> Self {
        Self { definition_repo, instance_repo, user_repo }
    }
}
```

### Handler 層（axum）

```rust
pub struct WorkflowState {
    pub usecase: WorkflowUseCaseImpl,
}

pub async fn create_workflow(
    State(state): State<Arc<WorkflowState>>,
    Json(body): Json<CreateWorkflowRequest>,
) -> Response {
    // ...
}
```

Handler にジェネリクスがないため、ルート登録が簡潔になる:

```rust
// Before: turbofish が必要
.route("/workflows", post(create_workflow::<PostgresDef, PostgresInst, PostgresStep, PostgresUser, PostgresCounter>))

// After: turbofish 不要
.route("/workflows", post(create_workflow))
```

### main.rs での初期化

```rust
let user_repo: Arc<dyn UserRepository> = Arc::new(PostgresUserRepository::new(pool.clone()));
let instance_repo: Arc<dyn WorkflowInstanceRepository> = Arc::new(PostgresWorkflowInstanceRepository::new(pool.clone()));

// Arc::clone() で複数の State 間で共有
let workflow_state = Arc::new(WorkflowState {
    usecase: WorkflowUseCaseImpl::new(
        Arc::new(PostgresWorkflowDefinitionRepository::new(pool.clone())),
        instance_repo.clone(),  // 共有
        user_repo.clone(),      // 共有
    ),
});
```

### テストでの Mock 注入

```rust
let sut = WorkflowUseCaseImpl::new(
    Arc::new(mock_definition_repo),
    Arc::new(mock_instance_repo),
    Arc::new(mock_user_repo),
);
```

## 注意点

### `Arc<dyn Trait>` は `Trait` を実装しない

`Arc<dyn UserRepository>` から `&dyn UserRepository` を取得するには `Arc::as_ref()` を使う:

```rust
// NG: Arc<dyn Trait> を &dyn Trait として直接渡せない
fn process(repo: &dyn UserRepository) { ... }
process(&arc_repo);  // コンパイルエラー

// OK: as_ref() で &dyn Trait を取得
process(arc_repo.as_ref());
```

これは `Arc<T>` が `Deref<Target = T>` を実装しているが、`T = dyn Trait` の場合、`&dyn Trait` への自動変換が `&dyn Trait` を期待する関数引数で常に効くわけではないため。明示的に `as_ref()` を使うのが確実。

### オブジェクト安全性

`dyn Trait` にするには、トレイトがオブジェクト安全（object-safe）である必要がある:

- ジェネリックメソッドがないこと
- `Self` を戻り値に使わないこと（`-> Self` は不可）
- `Sized` を要求しないこと

`#[async_trait]` はこれらの制約を自動的に処理する。

### `Clone` との組み合わせ

`dyn Trait` 自体は `Clone` できないが、`Arc<dyn Trait>` は `Clone` を実装する（参照カウントの増加のみ）。axum の `from_fn_with_state` が `Clone` を要求する場合も問題なく使える:

```rust
#[derive(Clone)]
pub struct CsrfState {
    pub session_manager: Arc<dyn SessionManager>,  // Arc は Clone
}
```

## プロジェクトでの使用箇所

| サービス | 対象 | `Arc<dyn Trait>` の使用箇所 |
|---------|------|---------------------------|
| Core Service | UseCase 層 | 5つのリポジトリトレイト |
| Core Service | Handler 層（UserState） | `UserRepository`, `TenantRepository` |
| Auth Service | UseCase 層 | `CredentialsRepository`, `PasswordChecker` |
| Auth Service | Handler 層 | `AuthUseCase` |
| BFF | Handler 層 | `CoreServiceClient`, `AuthServiceClient`, `SessionManager` |
| BFF | Middleware | `SessionManager`（CsrfState） |

導入: Issue #333、PR #335

## 関連リソース

- [Rust リファレンス: Trait objects](https://doc.rust-lang.org/reference/types/trait-object.html)
- [async-trait crate](https://docs.rs/async-trait/)
- ADR-007: DI にジェネリクスを採用（本変更で動的ディスパッチに移行）
