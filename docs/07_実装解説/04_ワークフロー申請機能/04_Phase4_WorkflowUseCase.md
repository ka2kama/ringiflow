# Phase 4: WorkflowUseCase

## 概要

ワークフローの作成・申請に関するビジネスロジックを実装した。
下書きとしてのワークフロー作成と、申請時のステップ生成を提供する。

### 対応 Issue

[#35 ワークフロー申請機能](https://github.com/ka2kama/ringiflow/issues/35) - Phase 4

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [API設計 > POST /api/v1/workflows](../../03_詳細設計書/03_API設計.md) | ワークフロー作成 API |
| [API設計 > POST /api/v1/workflows/{id}/submit](../../03_詳細設計書/03_API設計.md) | ワークフロー申請 API |
| [実装ロードマップ > Phase 1: MVP](../../03_詳細設計書/00_実装ロードマップ.md#phase-1-mvp) | Phase 1 の位置づけ |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/apps/core-service/src/usecase.rs`](../../../backend/apps/core-service/src/usecase.rs) | ユースケーストレイト定義 |
| [`backend/apps/core-service/src/usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs) | ワークフローユースケース実装 |
| [`backend/apps/core-service/src/error.rs`](../../../backend/apps/core-service/src/error.rs) | エラー定義 |

---

## 実装内容

### ユースケーストレイト（[`usecase.rs`](../../../backend/apps/core-service/src/usecase.rs)）

```rust
#[async_trait]
pub trait WorkflowUseCase: Send + Sync {
    /// ワークフローインスタンスを作成する（下書き）
    async fn create_workflow(
        &self,
        input: CreateWorkflowInput,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowInstance, CoreError>;

    /// ワークフローを申請する
    async fn submit_workflow(
        &self,
        input: SubmitWorkflowInput,
        instance_id: WorkflowInstanceId,
        tenant_id: TenantId,
    ) -> Result<WorkflowInstance, CoreError>;
}
```

### WorkflowUseCaseImpl（[`usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs)）

#### 1. ワークフロー作成（`create_workflow`）

**処理フロー:**

```rust
pub async fn create_workflow(
    &self,
    input: CreateWorkflowInput,
    tenant_id: TenantId,
    user_id: UserId,
) -> Result<WorkflowInstance, CoreError> {
    // 1. ワークフロー定義を取得
    let definition = self.definition_repo.find_by_id(&input.definition_id, &tenant_id).await?
        .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

    // 2. 公開済みであるか確認
    if definition.status() != WorkflowDefinitionStatus::Published {
        return Err(CoreError::BadRequest("公開されていないワークフロー定義です".to_string()));
    }

    // 3. WorkflowInstance を draft として作成
    let instance = WorkflowInstance::new(
        tenant_id,
        input.definition_id,
        definition.version(),
        input.title,
        input.form_data,
        user_id,
    );

    // 4. リポジトリに保存
    self.instance_repo.save(&instance).await?;

    Ok(instance)
}
```

**入力:**

```rust
pub struct CreateWorkflowInput {
    pub definition_id: WorkflowDefinitionId,
    pub title: String,
    pub form_data: JsonValue,
}
```

**検証:**

- ワークフロー定義が存在するか
- 定義が公開済み (`published`) であるか

#### 2. ワークフロー申請（`submit_workflow`）

**処理フロー:**

```rust
pub async fn submit_workflow(
    &self,
    input: SubmitWorkflowInput,
    instance_id: WorkflowInstanceId,
    tenant_id: TenantId,
) -> Result<WorkflowInstance, CoreError> {
    // 1. ワークフローインスタンスを取得
    let instance = self.instance_repo.find_by_id(&instance_id, &tenant_id).await?
        .ok_or_else(|| CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string()))?;

    // 2. draft 状態であるか確認
    if instance.status() != WorkflowInstanceStatus::Draft {
        return Err(CoreError::BadRequest("下書き状態のワークフローのみ申請できます".to_string()));
    }

    // 3. ワークフロー定義を取得（将来の拡張のため）
    let _definition = self.definition_repo.find_by_id(instance.definition_id(), &tenant_id).await?
        .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

    // 4. ステップを作成 (MVP では1段階承認のみ)
    let step = WorkflowStep::new(
        instance_id.clone(),
        "approval".to_string(),
        "承認".to_string(),
        "approval".to_string(),
        Some(input.assigned_to),
    );

    // 5. ステップを active に設定
    let active_step = step.activated();

    // 6. ワークフローインスタンスを申請済みに遷移
    let submitted_instance = instance.submitted()?;

    // 7. current_step_id を設定して in_progress に遷移
    let in_progress_instance = submitted_instance.with_current_step("approval".to_string());

    // 8. インスタンスとステップを保存
    self.instance_repo.save(&in_progress_instance).await?;
    self.step_repo.save(&active_step).await?;

    Ok(in_progress_instance)
}
```

**入力:**

```rust
pub struct SubmitWorkflowInput {
    pub assigned_to: UserId,
}
```

**検証:**

- ワークフローインスタンスが存在するか
- インスタンスが下書き (`draft`) 状態であるか
- ワークフロー定義が存在するか

**状態遷移:**

```
draft → pending (submitted) → in_progress (with_current_step)
```

### CoreError（[`error.rs`](../../../backend/apps/core-service/src/error.rs)）

```rust
#[derive(Debug, Error)]
pub enum CoreError {
    /// リソースが見つからない (404)
    NotFound(String),

    /// 不正なリクエスト (400)
    BadRequest(String),

    /// データベースエラー (500)
    Database(#[from] InfraError),

    /// 内部エラー (500)
    Internal(String),
}
```

RFC 7807 Problem Details 形式のエラーレスポンスに変換される。

---

## テスト

### ユニットテスト（[`usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs)）

| テスト | 目的 |
|-------|------|
| `test_create_workflow_正常系` | ワークフロー作成の正常系 |
| `test_create_workflow_定義が見つからない` | 定義不在時のエラーハンドリング |
| `test_submit_workflow_正常系` | ワークフロー申請の正常系 |

### 実行方法

```bash
cd backend && cargo test -p ringiflow-core-service usecase::workflow::tests
```

---

## 関連ドキュメント

- [API設計](../../03_詳細設計書/03_API設計.md)
- [実装ロードマップ](../../03_詳細設計書/00_実装ロードマップ.md)
- [Phase 1-3 実装解説](./01_Phase1_WorkflowDefinitionRepository.md)

---

## 設計解説

### 1. トレイトベースの設計によるテスタビリティ

**場所:** [`usecase.rs:32-73`](../../../backend/apps/core-service/src/usecase.rs#L32-L73)

**コード例:**

```rust
#[async_trait]
pub trait WorkflowUseCase: Send + Sync {
    async fn create_workflow(...) -> Result<WorkflowInstance, CoreError>;
    async fn submit_workflow(...) -> Result<WorkflowInstance, CoreError>;
}

impl<D, I, S> WorkflowUseCase for WorkflowUseCaseImpl<D, I, S>
where
    D: WorkflowDefinitionRepository + Send + Sync,
    I: WorkflowInstanceRepository + Send + Sync,
    S: WorkflowStepRepository + Send + Sync,
{
    // 実装
}
```

**なぜこの設計か:**

auth-service の `AuthUseCase` トレイトと同じパターンを採用。

設計の意図:

1. **テスタビリティの向上**
   - トレイトによりモックの注入が容易
   - ユニットテストで Mock リポジトリを使用可能
   - ハンドラ層のテストでもモックを活用

2. **依存性の逆転**
   - ユースケース層がトレイトに依存
   - リポジトリの具体的な実装（PostgreSQL など）はインフラ層が提供
   - レイヤー間の結合度を低減

3. **将来の拡張性**
   - 別の実装（キャッシュ付き、読み取り専用など）への切り替えが容易
   - トレイト境界を保ちながら実装を追加可能

**代替案:**

1. トレイトなしで直接実装を使用
   - ❌ テストでモックが困難
   - ❌ レイヤー間の結合が強くなる

2. トレイトオブジェクト (`dyn WorkflowUseCase`) を使用
   - ❌ ジェネリクスより実行時オーバーヘッドが大きい
   - ❌ コンパイル時の型チェックが弱い

3. 現在の方式（ジェネリクス + トレイト境界）
   - ✅ ゼロコスト抽象化
   - ✅ コンパイル時の型安全性
   - ✅ テストでの柔軟性

### 2. 状態遷移の明示的な管理

**場所:** [`usecase/workflow.rs:186-191`](../../../backend/apps/core-service/src/usecase/workflow.rs#L186-L191)

**コード例:**

```rust
// ワークフローインスタンスを申請済みに遷移
let submitted_instance = instance
    .submitted()
    .map_err(|e| CoreError::BadRequest(e.to_string()))?;

// current_step_id を設定して in_progress に遷移
let in_progress_instance = submitted_instance.with_current_step("approval".to_string());
```

**なぜこの設計か:**

ワークフローの状態遷移をドメインモデルのメソッドで明示的に管理。

設計のポイント:

1. **型による状態保証**
   - 不変なエンティティを採用
   - 状態遷移は新しいインスタンスを返す
   - 不正な状態遷移はコンパイル時 or 実行時にエラー

2. **ビジネスルールの局所化**
   - 「draft のみ申請可能」というルールは `WorkflowInstance::submitted()` 内で検証
   - ユースケース層はドメインのメソッドを呼ぶだけ
   - ルールの重複がない

3. **明示的な遷移ステップ**
   - `draft → pending (submitted)` → `in_progress (with_current_step)`
   - 2段階に分けることで、将来の拡張（申請後の承認待ち処理など）に対応可能

**代替案:**

1. ユースケース層で直接ステータスを変更
   - ❌ ビジネスルールがユースケース層に散在
   - ❌ 同じ検証コードが複数箇所に重複

2. ステータスを直接設定する setter を用意
   - ❌ 不正な状態遷移を防げない
   - ❌ 型による保証がない

3. 現在の方式（不変エンティティ + 明示的な遷移メソッド）
   - ✅ 型安全性
   - ✅ ビジネスルールの局所化
   - ✅ 履歴が残る（状態変更が明示的）

### 3. MVP スコープでの簡略化と将来の拡張性

**場所:** [`usecase/workflow.rs:165-180`](../../../backend/apps/core-service/src/usecase/workflow.rs#L165-L180)

**コード例:**

```rust
// 3. ワークフロー定義を取得（ステップ定義の取得のため）
let _definition = self
    .definition_repo
    .find_by_id(instance.definition_id(), &tenant_id)
    .await?
    .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

// 4. ステップを作成 (MVP では1段階承認のみ)
let step = WorkflowStep::new(
    instance_id.clone(),
    "approval".to_string(),
    "承認".to_string(),
    "approval".to_string(),
    Some(input.assigned_to),
);
```

**なぜこの設計か:**

MVP では固定の1段階承認のみを実装し、将来の拡張に備えて設計を残す。

設計の意図:

1. **MVP スコープの明確化**
   - コメントで「MVP では1段階承認のみ」と明示
   - 固定値 (`"approval"`) をハードコード
   - 複雑なフロー定義の解析は Phase 2 以降

2. **将来の拡張ポイント**
   - `_definition` を取得（現在は未使用だが、将来のステップ定義解析に備える）
   - ステップの `step_id`, `step_name`, `step_type` を明示的に設定
   - 定義の JSON からステップを生成するロジックは後で追加可能

3. **段階的な実装**
   - Phase 4: 固定1段階承認
   - Phase 5: 複数ステップの順次承認
   - Phase 6: 並列承認・条件分岐

**代替案:**

1. MVP から複雑なフロー定義を実装
   - ❌ Phase 4 のスコープが肥大化
   - ❌ テストが複雑になる

2. 将来の拡張を考慮せず、完全にハードコード
   - ❌ Phase 5 で大幅な書き直しが必要
   - ❌ 設計の連続性がない

3. 現在の方式（MVP 実装 + 拡張ポイント明示）
   - ✅ Phase 4 は最小限の実装
   - ✅ 将来の拡張が容易
   - ✅ コメントで意図が明確

### 4. Mock を使ったユニットテスト

**場所:** [`usecase/workflow.rs:224-412`](../../../backend/apps/core-service/src/usecase/workflow.rs#L224-L412)

**コード例:**

```rust
#[derive(Clone)]
struct MockWorkflowDefinitionRepository {
    definitions: Arc<Mutex<Vec<WorkflowDefinition>>>,
}

impl MockWorkflowDefinitionRepository {
    fn new() -> Self {
        Self {
            definitions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add_definition(&self, def: WorkflowDefinition) {
        self.definitions.lock().unwrap().push(def);
    }
}

#[async_trait]
impl WorkflowDefinitionRepository for MockWorkflowDefinitionRepository {
    // ...
}
```

**なぜこの設計か:**

リポジトリの Mock 実装により、DB なしでユースケースをテスト可能。

設計のポイント:

1. **メモリ内リポジトリ**
   - `Arc<Mutex<Vec<T>>>` でスレッドセーフなインメモリストレージ
   - 非同期テストでも正常に動作
   - テストの実行速度が速い（DB 接続不要）

2. **トレイトによる抽象化の恩恵**
   - ユースケースは `WorkflowDefinitionRepository` トレイトに依存
   - 本番は `PostgresWorkflowDefinitionRepository`
   - テストは `MockWorkflowDefinitionRepository`

3. **テストの独立性**
   - 各テストケースで新しい Mock インスタンスを作成
   - テスト間でデータが共有されない
   - 並列実行が可能

**代替案:**

1. 統合テストのみ（DB 接続が必要）
   - ❌ テストの実行速度が遅い
   - ❌ 環境構築が複雑

2. テストなし
   - ❌ バグの早期発見が困難
   - ❌ リファクタリング時の安全性が低い

3. 現在の方式（Mock を使ったユニットテスト）
   - ✅ 高速なテスト実行
   - ✅ 環境に依存しない
   - ✅ ビジネスロジックのテストに集中

---

## 次のステップ

Phase 5 では、Phase 4 で未実装の部分を拡張する:

- **複数ステップの順次承認**: ワークフロー定義から複数のステップを生成
- **差し戻し機能**: 前ステップへの差し戻し
- **承認/却下ユースケース**: ステップの完了処理

Phase 4 で確立したパターン（トレイトベース設計、状態遷移の明示化）を継続して適用する。

---

## 変更履歴

| 日付 | 変更内容 | 担当 |
|------|---------|------|
| 2026-01-26 | Phase 4 実装解説を追加 | - |
