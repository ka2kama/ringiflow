# Plan: `new` / `from_db` にパラメータ構造体を導入

## 背景

Issue #222 で `id` と `now` をドメインモデルに外部注入した結果、引数が増加し `#[allow(clippy::too_many_arguments)]` が必要になった。パラメータ構造体を導入して引数を1つにまとめる。

### `FromRow` を採用しない理由

当初 `from_db` には `sqlx::FromRow` derive を検討したが、以下の理由でパラメータ構造体に変更:

- domain クレートは sqlx に依存していない（`apps → domain → shared`, `apps → infra → shared`）
- `FromRow` を domain 型に derive すると、ドメイン層が特定の DB ライブラリに結合する
- カスタム型（ID 型、Version、WorkflowName 等）は `sqlx::Type` / `sqlx::Decode` を実装しておらず、追加すると domain の sqlx 依存が深まる
- パラメータ構造体で `too_many_arguments` の解消と引数順序の安全性は同様に達成できる

## 変更方針

### 命名規則

| 用途 | 構造体名 | 例 |
|------|---------|-----|
| エンティティ新規作成 | `New{Entity}` | `NewWorkflowInstance` |
| DB からの復元 | `{Entity}Record` | `WorkflowInstanceRecord` |

### 構造体の設計

- `New*`: `id`, ビジネスデータ, `now` を**すべて含む**（ユーザー選択済み）
- `*Record`: エンティティの全フィールドを含む（DB の完全な状態を表現）
- フィールドは `pub` にする（構造体自体がデータの入れ物であり、不変条件は `new` / `from_db` 内で保証）

### `new` のシグネチャ変更

```rust
// Before
impl WorkflowInstance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(id: ..., tenant_id: ..., ..., now: ...) -> Self { ... }
}

// After
pub struct NewWorkflowInstance {
    pub id: WorkflowInstanceId,
    pub tenant_id: TenantId,
    pub definition_id: WorkflowDefinitionId,
    pub definition_version: Version,
    pub display_number: DisplayNumber,
    pub title: String,
    pub form_data: JsonValue,
    pub initiated_by: UserId,
    pub now: DateTime<Utc>,
}

impl WorkflowInstance {
    pub fn new(params: NewWorkflowInstance) -> Self { ... }
}
```

### `from_db` のシグネチャ変更

```rust
// Before
impl WorkflowInstance {
    #[allow(clippy::too_many_arguments)]
    pub fn from_db(id: ..., ..., updated_at: ...) -> Self { ... }
}

// After
pub struct WorkflowInstanceRecord {
    pub id: WorkflowInstanceId,
    pub tenant_id: TenantId,
    // ... 全フィールド
}

impl WorkflowInstance {
    pub fn from_db(record: WorkflowInstanceRecord) -> Self { ... }
}
```

## 対象スコープ

### 対象

- WorkflowDefinition (`new`: 7 args → 1, `from_db`: 10 args → 1)
- WorkflowInstance (`new`: 9 args → 1, `from_db`: 15 args → 1)
- WorkflowStep (`new`: 7 args → 1, `from_db`: 15 args → 1)

### 対象外

- `User::from_db`（8 args）— Issue #222 のスコープ外。別 Issue で対応
- `from_db` の `sqlx::FromRow` 化 — 上述の理由で見送り

## 変更対象ファイル

### domain 層（構造体定義 + メソッド変更）

- `backend/crates/domain/src/workflow.rs`
  - `NewWorkflowDefinition` 構造体追加（`new` の前）
  - `WorkflowDefinitionRecord` 構造体追加（`from_db` の前）
  - `WorkflowDefinition::new` — パラメータ構造体を受け取るように変更
  - `WorkflowDefinition::from_db` — パラメータ構造体を受け取るように変更、`#[allow]` 削除
  - 同様に `WorkflowInstance`, `WorkflowStep` も変更

### usecase 層（`new` の呼び出し元）

- `backend/apps/core-service/src/usecase/workflow.rs`
  - L176: `WorkflowInstance::new(...)` → `NewWorkflowInstance { ... }` を構築して渡す
  - L251: `WorkflowStep::new(...)` → `NewWorkflowStep { ... }` を構築して渡す
  - テスト内の多数の呼び出し箇所も同様

- `backend/apps/core-service/src/usecase/task.rs`
  - テスト内の `WorkflowInstance::new`, `WorkflowStep::new` 呼び出し箇所

### infra 層（`from_db` の呼び出し元）

- `backend/crates/infra/src/repository/workflow_definition_repository.rs`
  - L107, L159: `WorkflowDefinition::from_db(...)` → `WorkflowDefinitionRecord { ... }` を構築して渡す

- `backend/crates/infra/src/repository/workflow_instance_repository.rs`
  - L247, L295, L348, L407: `WorkflowInstance::from_db(...)` → `WorkflowInstanceRecord { ... }` を構築して渡す

- `backend/crates/infra/src/repository/workflow_step_repository.rs`
  - L168, L220, L272: `WorkflowStep::from_db(...)` → `WorkflowStepRecord { ... }` を構築して渡す

### infra テスト（`new` の呼び出し元）

- `backend/crates/infra/tests/workflow_definition_repository_test.rs`
- `backend/crates/infra/tests/workflow_instance_repository_test.rs`
- `backend/crates/infra/tests/workflow_step_repository_test.rs`

## 実装順序

1. **domain 層**: パラメータ構造体を定義し、`new` / `from_db` のシグネチャを変更
2. **infra リポジトリ**: `from_db` の呼び出し元を更新
3. **usecase 層**: `new` の呼び出し元を更新
4. **infra テスト**: `new` の呼び出し元を更新
5. **検証**: `just check` でコンパイル + テスト通過を確認

## 検証

```bash
just check      # lint + test
just check-all  # lint + test + API test
```

## 自己検証（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | `new` の全呼び出し元（usecase 2ファイル、infra tests 3ファイル）と `from_db` の全呼び出し元（infra repository 3ファイル）を調査済み。User::from_db は対象外として明記 |
| 2 | 曖昧さ排除 | OK | 構造体名（New*, *Record）、フィールドの公開範囲（pub）、配置場所（workflow.rs 内）を確定。「必要に応じて」等の不確定表現なし |
| 3 | 設計判断の完結性 | OK | FromRow 不採用の理由（ドメイン層の sqlx 依存回避）を根拠付きで記載。パラメータ構造体のフィールド構成（全部入り）はユーザー選択済み |
| 4 | スコープ境界 | OK | 対象（3エンティティの new + from_db）と対象外（User、FromRow 化）を明記 |
| 5 | 技術的前提 | OK | sqlx::FromRow の制約（domain が sqlx に依存する必要性）、clippy::too_many_arguments の閾値（7引数以上）を確認済み |
| 6 | 既存ドキュメント整合 | OK | 依存方向（apps → domain → shared）は CLAUDE.md のアーキテクチャ記載と整合。domain が infra 技術に依存しない原則を遵守 |
