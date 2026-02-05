# Phase 2: Workflow の非決定的値排除

## 概要

`WorkflowDefinition`, `WorkflowInstance`, `WorkflowStep` のコンストラクタ・状態遷移メソッドから `Utc::now()` / `Uuid::now_v7()` を排除し、呼び出し元から注入する形に変更した。

対応 Issue: [#222](https://github.com/ka2kama/ringiflow/issues/222)

## 設計書との対応

- [詳細設計書: ドメインモデル](../../../docs/03_詳細設計書/) — WorkflowDefinition, WorkflowInstance, WorkflowStep エンティティ

## 実装したコンポーネント

### ドメイン層

| ファイル | 責務 |
|---------|------|
| [`workflow.rs`](../../../backend/crates/domain/src/workflow.rs) | 3 エンティティのコンストラクタ + 状態遷移に `id`, `now` 注入 |

### ユースケース層（プロダクションコード）

| ファイル | 責務 |
|---------|------|
| [`usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs) | `Utc::now()` の取得 + ドメイン操作への注入 |

### テストコード

| ファイル | 責務 |
|---------|------|
| [`usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs) テスト | ワークフローユースケーステスト |
| [`usecase/task.rs`](../../../backend/apps/core-service/src/usecase/task.rs) テスト | タスクユースケーステスト |
| [`usecase/dashboard.rs`](../../../backend/apps/core-service/src/usecase/dashboard.rs) テスト | ダッシュボードユースケーステスト |
| [`workflow_instance_repository_test.rs`](../../../backend/crates/infra/tests/workflow_instance_repository_test.rs) | インスタンスリポジトリ統合テスト |
| [`workflow_step_repository_test.rs`](../../../backend/crates/infra/tests/workflow_step_repository_test.rs) | ステップリポジトリ統合テスト |

## 実装内容

### WorkflowDefinition

| メソッド | 追加パラメータ |
|---------|--------------|
| `new()` | `id: WorkflowDefinitionId`, `now: DateTime<Utc>` |
| `published()` | `now: DateTime<Utc>` |
| `archived()` | `now: DateTime<Utc>` |

### WorkflowInstance

| メソッド | 追加パラメータ |
|---------|--------------|
| `new()` | `id: WorkflowInstanceId`, `now: DateTime<Utc>` |
| `submitted()` | `now: DateTime<Utc>` |
| `approved()` | `now: DateTime<Utc>` |
| `rejected()` | `now: DateTime<Utc>` |
| `cancelled()` | `now: DateTime<Utc>` |
| `with_current_step()` | `now: DateTime<Utc>` |
| `complete_with_approval()` | `now: DateTime<Utc>` |
| `complete_with_rejection()` | `now: DateTime<Utc>` |

### WorkflowStep

| メソッド | 追加パラメータ |
|---------|--------------|
| `new()` | `id: WorkflowStepId`, `now: DateTime<Utc>` |
| `activated()` | `now: DateTime<Utc>` |
| `completed()` | `now: DateTime<Utc>` |
| `skipped()` | `now: DateTime<Utc>` |
| `approve()` | `now: DateTime<Utc>` |
| `reject()` | `now: DateTime<Utc>` |
| `is_overdue()` | `now: DateTime<Utc>` |

### ユースケース層のパターン

各ユースケースメソッドの冒頭で `let now = chrono::Utc::now();` を1回取得し、そのスコープ内の全ドメイン操作に渡す:

```rust
pub async fn submit_workflow(&self, id: &str) -> Result<WorkflowInstanceDto, AppError> {
    let now = chrono::Utc::now();
    // ...
    let step = WorkflowStep::new(
        WorkflowStepId::new(),
        // ... 他の引数 ...
        now,
    );
    let instance = instance.submitted(now)?;
    let instance = instance.with_current_step(step.id().to_string(), now);
    let step = step.activated(now);
    // ...
}
```

### 追加したテスト

WorkflowInstance:
- `test_新規作成時のcreated_atとupdated_atは注入された値と一致する`
- `test_submitted後のsubmitted_atは注入された値と一致する`

WorkflowStep:
- `test_新規作成時のcreated_atとupdated_atは注入された値と一致する`
- `test_activated後のstarted_atは注入された値と一致する`
- `test_is_overdue_期限切れの場合trueを返す`
- `test_is_overdue_期限内の場合falseを返す`

## テスト

```bash
cd backend && cargo test --package ringiflow-domain
cd backend && cargo test --package ringiflow-core-service
just test-rust-integration  # infra テスト
```

## 設計解説

### 1. ユースケース層での `now` 一括取得パターン

場所: [`usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs)

なぜこの設計か:
- 1つのユースケース操作内で複数のエンティティが生成・遷移する場合、同じタイムスタンプを使うことで一貫性を保つ
- 例: `submit_workflow` では `WorkflowStep::new()`, `instance.submitted()`, `instance.with_current_step()`, `step.activated()` がすべて同じ `now` を受け取る
- `Utc::now()` の呼び出しはユースケースメソッドの冒頭に集約され、ドメイン層には一切の副作用がない

代替案:
- メソッドごとに `Utc::now()` を呼ぶ: ミリ秒単位のずれが生じ、`created_at` と `updated_at` が異なる可能性がある
- トランザクション開始時の DB タイムスタンプ（`NOW()`）を使う: DB 層への依存が生じ、ドメインモデルの独立性が損なわれる

### 2. `is_overdue()` の `now` 引数化

場所: [`workflow.rs`](../../../backend/crates/domain/src/workflow.rs)（`WorkflowStep::is_overdue`）

```rust
pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
    if let Some(deadline) = self.deadline {
        self.status == StepStatus::Active && now > deadline
    } else {
        false
    }
}
```

なぜこの設計か:
- Functional Core の一貫性。ドメインモデル内の全メソッドが決定的（同じ入力 → 同じ出力）
- テストで「期限切れ」「期限内」の両方を確実に検証できる

代替案:
- `is_overdue()` だけ `Utc::now()` を内部で呼ぶ: 「読み取り専用だから許容」という判断もあり得るが、テスタビリティの観点と一貫性から `now` 引数を選択した

### 3. `#[allow(clippy::too_many_arguments)]` の適用

場所: [`workflow.rs`](../../../backend/crates/domain/src/workflow.rs)（`WorkflowInstance::new`）

`WorkflowInstance::new()` は `id` + `now` の追加で 9 引数になった。

なぜこの設計か:
- エンティティの全フィールドを明示的に受け取るコンストラクタでは、引数が多いのは構造的に避けられない
- `from_db()` も同じパターンで `#[allow]` を使用しており、一貫性がある

代替案:
- Builder パターン: `WorkflowInstance::builder().id(id).tenant_id(tid)...build()` — コンストラクタのためだけに Builder を導入するのは過度な複雑化。必須フィールドの欠落がコンパイル時に検出できない
- パラメータ構造体: `NewWorkflowInstanceParams { id, tenant_id, ... }` — フィールドが構造体に移動するだけで本質的な改善にならない

## 関連ドキュメント

- [Phase 1: User と Role](01_Phase1_UserとRole.md)
- [ナレッジベース: Functional Core, Imperative Shell](https://blog.ploeh.dk/2020/03/02/impureim-sandwich/)
