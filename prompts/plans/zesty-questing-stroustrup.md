# Issue #687: リポジトリ層にトランザクションコンテキストを導入

## Context

ワークフローユースケース（approve, reject, request_changes, submit, resubmit）は複数リポジトリの書き込みメソッドを呼び出すが、各呼び出しは独立した DB 接続で実行される。中間状態での障害時にデータ不整合が発生する。

`.claude/rules/repository.md` にはトランザクション必須と明記されていたが、全5ユースケースで守られなかった。**ルールの存在だけでは品質は保証されない**ため、型レベルでの構造的強制を導入する。

Epic #685 の Story #687 として、トランザクションコンテキストの基盤を整備する。

## 設計判断: 構造的強制アプローチ

書き込みメソッドの署名自体に `TxContext` を必須引数として追加する。`_tx` 変種メソッドの追加（旧計画）ではなく、既存の書き込みメソッドを直接変更する。

| アプローチ | 構造的強制 | trait object 互換 | 採否 |
|-----------|----------|-----------------|------|
| A: Transaction 直接 | ○ | ○ | △ sqlx 直接露出 |
| B: Executor ジェネリクス | × (Pool も渡せる) | × (Arc\<dyn\> 非互換) | 却下 |
| C: UnitOfWork | ○ | ○ | △ 抽象化層が増える |
| **A+C ハイブリッド（構造的強制版）** | **○** | **○** | **採用** |

採用アプローチ: `TxContext` 型で `Transaction` をラップ（C 的）し、書き込みメソッドの必須引数とする（A 的）。`_tx` 変種は作らず、書き込みメソッドそのものが `TxContext` を要求する。

### スコープ

対象:
- `TxContext` 構造体の定義（`infra/src/db.rs`）
- `TransactionManager` trait + 実装（`infra/src/db.rs`, `infra/src/mock.rs`）
- `WorkflowStepRepository`: `insert`, `update_with_version_check` に `TxContext` 追加
- `WorkflowInstanceRepository`: `insert`, `update_with_version_check` に `TxContext` 追加
- `WorkflowInstanceRepository::update_with_version_check` に `tenant_id` 条件追加
- テスト用拡張 trait（`test-utils` feature）
- 全呼び出し元の更新（6 use case + テストセットアップ + 統合テスト）
- ADR-051 作成

対象外:
- `WorkflowCommentRepository` の TxContext 対応（独立した書き込み操作、Epic #685 のスコープ外）
- `DisplayIdCounterRepository` のトランザクション参加（#689 で対応）
- ユースケース間でのトランザクション共有（#688, #689 で対応。#687 では per-call TxContext）
- その他のリポジトリ（UserRepository, RoleRepository 等）の TxContext 対応
- 読み取りメソッドのトランザクション対応

## Phase 構成

### Phase 1: TxContext + TransactionManager 定義 + ADR-051

`TxContext` 型と `TransactionManager` trait を追加し、ADR-051 を作成する。既存コードへの破壊的変更なし。

#### 確認事項
- [ ] 型: `sqlx::Transaction<'static, Postgres>` → `db.rs` L55 の既存 import
- [ ] パターン: `TenantConnection` のラッパーパターン → `db.rs` L137-180
- [ ] ライブラリ: `pool.begin()` の戻り値型 → Grep 既存使用 `display_id_counter_repository.rs`
- [ ] パターン: `test-utils` feature ゲート → `mock.rs` L1-10, `lib.rs` L56-57, `Cargo.toml` L25

#### 実装内容

```rust
// infra/src/db.rs に追加

/// トランザクションコンテキスト
///
/// 書き込みリポジトリメソッドの必須引数。
/// トランザクションなしの書き込みをコンパイルエラーにする（構造的強制）。
pub struct TxContext(TxContextInner);

enum TxContextInner {
    Pg(sqlx::Transaction<'static, Postgres>),
    #[cfg(any(test, feature = "test-utils"))]
    Mock,
}

impl TxContext {
    /// Postgres トランザクションを開始する
    pub(crate) async fn begin_pg(pool: &PgPool) -> Result<Self, InfraError> {
        Ok(Self(TxContextInner::Pg(pool.begin().await?)))
    }

    /// テスト用のモック TxContext を作成する
    #[cfg(any(test, feature = "test-utils"))]
    pub fn mock() -> Self {
        Self(TxContextInner::Mock)
    }

    /// トランザクションをコミットする
    pub async fn commit(self) -> Result<(), InfraError> {
        match self.0 {
            TxContextInner::Pg(tx) => { tx.commit().await?; Ok(()) }
            #[cfg(any(test, feature = "test-utils"))]
            TxContextInner::Mock => Ok(()),
        }
    }

    /// トランザクション内の DB コネクションを取得する（crate 内部用）
    pub(crate) fn conn(&mut self) -> &mut PgConnection {
        match &mut self.0 {
            TxContextInner::Pg(tx) => &mut **tx,
            #[cfg(any(test, feature = "test-utils"))]
            TxContextInner::Mock => panic!(
                "BUG: conn() called on Mock TxContext. Mock repos should not call conn()."
            ),
        }
    }
}
```

設計判断:
- `begin_pg` は `pub(crate)`: `PgTransactionManager` のみが使用。ユースケース層は `TransactionManager` trait 経由
- `conn()` は `pub(crate)`: Postgres リポジトリ実装のみが使用
- `mock()` は `pub`: テストコードから直接呼び出し
- Mock バリアントの `conn()` は panic: Mock リポジトリは `conn()` を呼ばないため、呼ばれたらバグ

```rust
// infra/src/db.rs に追加

/// トランザクション管理 trait
///
/// ユースケース層が TxContext を作成するための抽象化。
/// ユースケース層は PgPool に直接依存しない。
#[async_trait::async_trait]
pub trait TransactionManager: Send + Sync {
    async fn begin(&self) -> Result<TxContext, InfraError>;
}

/// Postgres 用 TransactionManager 実装
pub struct PgTransactionManager {
    pool: PgPool,
}

impl PgTransactionManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TransactionManager for PgTransactionManager {
    async fn begin(&self) -> Result<TxContext, InfraError> {
        TxContext::begin_pg(&self.pool).await
    }
}
```

```rust
// infra/src/mock.rs に追加

/// テスト用 MockTransactionManager
pub struct MockTransactionManager;

#[async_trait]
impl TransactionManager for MockTransactionManager {
    async fn begin(&self) -> Result<TxContext, InfraError> {
        Ok(TxContext::mock())
    }
}
```

lib.rs の re-export に `TransactionManager`, `PgTransactionManager` を追加。

#### テストリスト

ユニットテスト:
- [ ] `TxContext` が `Send` であることの型テスト
- [ ] `PgTransactionManager` が `Send + Sync` であることの型テスト

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

統合テスト:
- [ ] `begin` でトランザクションを開始できる
- [ ] `commit` でトランザクションをコミットできる
- [ ] ドロップ時にロールバックされる（commit せずにスコープを抜ける）

### Phase 2: tenant_id 修正

`WorkflowInstanceRepository::update_with_version_check` に `tenant_id: &TenantId` 引数を追加し、WHERE 句に `AND tenant_id = $N` を追加する。

#### 確認事項
- [ ] 型: `WorkflowInstanceRepository` trait の現在のシグネチャ → `workflow_instance_repository.rs` L65-69
- [ ] パターン: `WorkflowStepRepository::update_with_version_check` の tenant_id 使用 → `workflow_step_repository.rs` L40-45, L187-229
- [ ] 呼び出し元: approve.rs L134, reject.rs L121, request_changes.rs L121, submit.rs L138, resubmit.rs L150

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `infra/src/repository/workflow_instance_repository.rs` | trait: `tenant_id: &TenantId` 追加、impl: WHERE に `AND tenant_id = $N` 追加 |
| `infra/src/mock.rs` | Mock impl: `_tenant_id: &TenantId` 追加 |
| `core-service/src/usecase/workflow/command/decision/approve.rs` | L134 に `&tenant_id` 追加 |
| `core-service/src/usecase/workflow/command/decision/reject.rs` | L121 に `&tenant_id` 追加 |
| `core-service/src/usecase/workflow/command/decision/request_changes.rs` | L121 に `&tenant_id` 追加 |
| `core-service/src/usecase/workflow/command/lifecycle/submit.rs` | L138 に `&tenant_id` 追加 |
| `core-service/src/usecase/workflow/command/lifecycle/resubmit.rs` | L150 に `&tenant_id` 追加 |
| `infra/tests/workflow_instance_repository_test.rs` | テスト呼び出しに `&tenant_id` 追加 |

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

統合テスト:
- [ ] バージョン一致で更新できる（既存テストの修正）
- [ ] バージョン不一致で conflict エラーを返す（既存テストの修正）
- [ ] 別テナントのインスタンスは更新できない（新規テスト）

### Phase 3: 構造的強制 — 書き込みメソッドに TxContext 必須化

4つの書き込みメソッドに `tx: &mut TxContext` を第2引数として追加する。テスト用拡張 trait を提供し、テストセットアップの冗長化を抑制する。

#### 確認事項
- [ ] 型: Phase 1 で定義した TxContext の `conn()` 戻り値型 → Phase 1 の実装結果
- [ ] パターン: `sqlx::query!().execute(&self.pool)` → `sqlx::query!().execute(conn)` への変更 → `workflow_step_repository.rs` L181, L218, `workflow_instance_repository.rs` L255, L291
- [ ] 呼び出し元: `instance_repo.insert()` 全箇所（約 50 箇所）、`step_repo.insert()` 全箇所（約 30 箇所）

#### 変更メソッドシグネチャ

```rust
// WorkflowStepRepository
async fn insert(
    &self,
    tx: &mut TxContext,  // 追加
    step: &WorkflowStep,
    tenant_id: &TenantId,
) -> Result<(), InfraError>;

async fn update_with_version_check(
    &self,
    tx: &mut TxContext,  // 追加
    step: &WorkflowStep,
    expected_version: Version,
    tenant_id: &TenantId,
) -> Result<(), InfraError>;

// WorkflowInstanceRepository
async fn insert(
    &self,
    tx: &mut TxContext,  // 追加
    instance: &WorkflowInstance,
) -> Result<(), InfraError>;

async fn update_with_version_check(
    &self,
    tx: &mut TxContext,  // 追加
    instance: &WorkflowInstance,
    expected_version: Version,
    tenant_id: &TenantId,  // Phase 2 で追加済み
) -> Result<(), InfraError>;
```

#### Postgres impl の変更パターン

```rust
// Before:
async fn insert(&self, step: &WorkflowStep, tenant_id: &TenantId) -> Result<(), InfraError> {
    sqlx::query!(...).execute(&self.pool).await?;
    Ok(())
}

// After:
async fn insert(&self, tx: &mut TxContext, step: &WorkflowStep, tenant_id: &TenantId) -> Result<(), InfraError> {
    sqlx::query!(...).execute(tx.conn()).await?;
    Ok(())
}
```

pool から TxContext への変更のみ。DRY 用 private 関数の抽出は不要（pool ベースの書き込みパスが消滅するため）。

#### Mock impl の変更パターン

```rust
// Before:
async fn insert(&self, step: &WorkflowStep, _tenant_id: &TenantId) -> Result<(), InfraError> {
    // ...
}

// After:
async fn insert(&self, _tx: &mut TxContext, step: &WorkflowStep, _tenant_id: &TenantId) -> Result<(), InfraError> {
    // TxContext は無視、ロジックは同一
}
```

#### テスト用拡張 trait

テストセットアップコードの冗長化を抑制するため、`test-utils` feature ゲート付きの拡張 trait を提供する。

```rust
// infra/src/repository/workflow_instance_repository.rs の末尾

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
pub trait WorkflowInstanceRepositoryTestExt {
    /// テスト用: mock TxContext で insert する
    async fn insert_for_test(&self, instance: &WorkflowInstance) -> Result<(), InfraError>;
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl<T: WorkflowInstanceRepository + ?Sized> WorkflowInstanceRepositoryTestExt for T {
    async fn insert_for_test(&self, instance: &WorkflowInstance) -> Result<(), InfraError> {
        let mut tx = TxContext::mock();
        self.insert(&mut tx, instance).await
    }
}
```

同様に `WorkflowStepRepositoryTestExt` も定義（`insert_for_test` + `update_for_test`）。

テスト側の変更パターン:
```rust
// Before:
instance_repo.insert(&instance).await.unwrap();
step_repo.insert(&step, &tenant_id).await.unwrap();

// After:
use ringiflow_infra::repository::{WorkflowInstanceRepositoryTestExt, WorkflowStepRepositoryTestExt};
instance_repo.insert_for_test(&instance).await.unwrap();
step_repo.insert_for_test(&step, &tenant_id).await.unwrap();
```

#### 変更ファイル一覧

**infra crate:**

| ファイル | 変更内容 |
|---------|---------|
| `infra/src/repository/workflow_step_repository.rs` | trait: TxContext 追加、impl: `&self.pool` → `tx.conn()`、拡張 trait 追加 |
| `infra/src/repository/workflow_instance_repository.rs` | trait: TxContext 追加、impl: `&self.pool` → `tx.conn()`、拡張 trait 追加 |
| `infra/src/mock.rs` | Mock impl: `_tx: &mut TxContext` パラメータ追加 |
| `infra/tests/workflow_step_repository_test.rs` | 全 write 呼び出しに TxContext 追加 |
| `infra/tests/workflow_instance_repository_test.rs` | 全 write 呼び出しに TxContext 追加 |
| `infra/tests/workflow_comment_repository_test.rs` | テストセットアップの insert に拡張 trait 使用 |

**core-service crate:**

| ファイル | 変更内容 |
|---------|---------|
| `core-service/src/usecase/workflow.rs` | `WorkflowUseCaseImpl` に `tx_manager: Arc<dyn TransactionManager>` 追加 |
| `core-service/src/usecase/workflow/command/lifecycle/create.rs` | per-call TxContext で insert |
| `core-service/src/usecase/workflow/command/lifecycle/submit.rs` | per-call TxContext で update + insert |
| `core-service/src/usecase/workflow/command/lifecycle/resubmit.rs` | per-call TxContext で update + insert |
| `core-service/src/usecase/workflow/command/decision/approve.rs` | per-call TxContext で update |
| `core-service/src/usecase/workflow/command/decision/reject.rs` | per-call TxContext で update |
| `core-service/src/usecase/workflow/command/decision/request_changes.rs` | per-call TxContext で update |
| `core-service/src/usecase/workflow/command/comment.rs` | テストセットアップに拡張 trait 使用 |
| `core-service/src/usecase/dashboard.rs` | テストセットアップに拡張 trait 使用 |
| `core-service/src/usecase/task.rs` | テストセットアップに拡張 trait 使用 |
| `core-service/src/usecase/workflow/query.rs` | テストセットアップに拡張 trait 使用 |
| `core-service/src/test_utils/workflow_test_builder.rs` | MockTransactionManager 追加 |

**apps（DI 設定）:**

| ファイル | 変更内容 |
|---------|---------|
| `core-service/src/main.rs` (L214) | `PgTransactionManager` のインスタンス生成・`WorkflowUseCaseImpl::new()` に注入 |

**テスト内 `WorkflowUseCaseImpl::new()` 直接呼び出し（約 34 箇所）:**

`WorkflowTestBuilder` を使わず直接コンストラクトしているテスト関数すべてに `Arc::new(MockTransactionManager)` を追加する。対象ファイル: `create.rs`(2), `submit.rs`(4), `resubmit.rs`(5), `approve.rs`(6), `reject.rs`(6), `request_changes.rs`(5), `comment.rs`(4), `query.rs`(2)。

#### use case 内の per-call TxContext パターン

```rust
// #687 での暫定パターン（各 write call で独立した TxContext）
// #688/#689 でトランザクション共有に統合する

// Before:
self.instance_repo
    .update_with_version_check(&instance, expected_version)
    .await?;

// After:
let mut tx = self.tx_manager.begin().await
    .map_err(|e| CoreError::Internal(format!("トランザクション開始に失敗: {}", e)))?;
self.instance_repo
    .update_with_version_check(&mut tx, &instance, expected_version, &tenant_id)
    .await?;
tx.commit().await
    .map_err(|e| CoreError::Internal(format!("トランザクションコミットに失敗: {}", e)))?;
```

注: #688/#689 で複数の write call が同一 TxContext を共有するよう統合される。

#### テストリスト

ユニットテスト:
- [ ] Send + Sync チェック（TxContext 引数を含むトレイトオブジェクトが `Arc<dyn Repo>` に格納可能）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

統合テスト:
- [ ] `insert`（Step）でトランザクション内に挿入し commit で反映される
- [ ] `insert`（Instance）でトランザクション内に挿入し commit で反映される
- [ ] `update_with_version_check`（Step）でトランザクション内で更新し commit で反映される
- [ ] `update_with_version_check`（Instance）でトランザクション内で更新し commit で反映される
- [ ] トランザクション内で Step と Instance を同時に更新し、commit で両方が反映される
- [ ] トランザクション内で Step と Instance を同時に更新し、rollback（drop）で両方が取り消される
- [ ] TxContext 付き `update_with_version_check` でもバージョン不一致で conflict エラーを返す

## 検証

```bash
# ユニットテスト + 統合テスト
just check-all

# 統合テストのみ（開発中）
just test-rust-integration

# sqlx オフラインキャッシュの更新（クエリの引数変更のため必要）
just sqlx-prepare
```

### 構造的強制の確認

Phase 3 完了後、以下を確認する:
1. 書き込みメソッドから `TxContext` 引数を削除 → コンパイルエラーになる
2. ユースケースで `TxContext` なしに書き込みメソッドを呼ぶ → コンパイルエラーになる

### ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Approach B（Executor ジェネリクス）は `Arc<dyn Repository>` と非互換 + Pool も渡せるため構造的強制の要件を満たさない | アーキテクチャ不整合 | Approach B を却下 |
| 2回目 | Mock リポジトリで TxContext を受け取る必要があるが、Mock は PgPool を持たない | 不完全なパス | TxContext を enum にし、Mock バリアントを `#[cfg(any(test, feature = "test-utils"))]` で追加。Mock の `conn()` は panic |
| 3回目 | ユースケース層が TxContext を作成するには PgPool が必要だが、直接依存は避けたい | 既存手段の見落とし | `TransactionManager` trait を導入。ユースケース層は trait 経由で TxContext を取得 |
| 4回目 | テストセットアップコードが大量にあり（insert 約 50 箇所、step.insert 約 30 箇所）、全箇所で `let mut tx = TxContext::mock();` が必要になる | 品質の向上: シンプルさ | テスト用拡張 trait（`insert_for_test` 等）を `test-utils` feature で提供 |
| 5回目 | pool ベースの書き込みパスが消滅するため、DRY 用 private 関数（`do_insert` 等）は不要 | シンプルさ | Postgres impl は `&self.pool` → `tx.conn()` の直接置換のみ。private 関数抽出は不要 |
| 6回目 | `begin_pg` を `pub` にすると ユースケース層から直接呼べてしまう | 既存手段の見落とし | `begin_pg` を `pub(crate)` にし、外部からは `TransactionManager` 経由のみ |
| 7回目 | Phase 2 の tenant_id 修正と Phase 3 の TxContext 追加で同一メソッドの署名が2回変わる | シンプルさ | 関心の分離を優先し Phase を分ける。機械的変更なので重複コストは小さい |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Issue の完了基準 6 項目すべてに対応: ADR → Phase 1, 構造的強制 → Phase 3, コンパイルエラー確認 → 検証, tenant_id → Phase 2, 既存テスト → 各 Phase, check-all → 検証 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全メソッドシグネチャ、変更ファイル一覧、変更パターンを具体的に記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 3 アプローチの評価、TxContext 内部構造（enum + cfg）、conn() / begin_pg() の可視性、拡張 trait の設計理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象 7 項目と対象外 5 項目を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | sqlx 0.8 の execute が `&mut PgConnection` を受け取ること、`#[cfg]` の test-utils パターン、Arc\<dyn Repo\> との互換性を確認済み。RLS 用 `app.tenant_id` は現行コードで write 時に設定していない（bind パラメータで代用）ため TxContext でも同様 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | repository.md のトランザクション管理ルール、ADR-044 の RLS 二重防御方針と整合 |
