# ユーザー名解決の実装（Phase 1-5）

## 概要

API レスポンスの `initiated_by` / `assigned_to` を UUID 文字列から `UserRef { id, name }` オブジェクトに変更し、フロントエンドでユーザー名を表示する。

対応 Issue: [#196](https://github.com/ka2kama/ringiflow/issues/196)

## 設計書との対応

- [基本設計書: ワークフロー管理](../../02_基本設計書/) — ワークフローインスタンスのデータ構造

## 実装したコンポーネント

### Phase 1: UserRepository

| ファイル | 責務 |
|---------|------|
| [`user_repository.rs`](../../../backend/crates/infra/src/repository/user_repository.rs) | `find_by_ids` トレイトメソッド + PostgreSQL 実装 |
| [`user_repository_test.rs`](../../../backend/crates/infra/tests/user_repository_test.rs) | 統合テスト 3 件 |

### Phase 2: Core Service DTO + ユーザー名解決

| ファイル | 責務 |
|---------|------|
| [`handler/workflow.rs`](../../../backend/apps/core-service/src/handler/workflow.rs) | `UserRefDto`, `resolve_user_names`, DTO 変換関数 |
| [`handler/task.rs`](../../../backend/apps/core-service/src/handler/task.rs) | タスク関連 DTO 変換関数 |
| [`main.rs`](../../../backend/apps/core-service/src/main.rs) | State への `UserRepository` 追加 |

### Phase 3: BFF レスポンス型

| ファイル | 責務 |
|---------|------|
| [`client/core_service.rs`](../../../backend/apps/bff/src/client/core_service.rs) | `UserRefDto`（Deserialize） |
| [`handler/workflow.rs`](../../../backend/apps/bff/src/handler/workflow.rs) | `UserRefData`（Serialize） |
| [`handler/task.rs`](../../../backend/apps/bff/src/handler/task.rs) | タスク関連レスポンス型 |

### Phase 4: Elm フロントエンド

| ファイル | 責務 |
|---------|------|
| [`Data/UserRef.elm`](../../../frontend/src/Data/UserRef.elm) | `UserRef` 型 + デコーダー（新規） |
| [`Data/WorkflowInstance.elm`](../../../frontend/src/Data/WorkflowInstance.elm) | 型・デコーダー変更 |
| [`Data/Task.elm`](../../../frontend/src/Data/Task.elm) | 型・デコーダー変更 |
| [`Page/Workflow/Detail.elm`](../../../frontend/src/Page/Workflow/Detail.elm) | ビュー変更 |
| [`Page/Task/Detail.elm`](../../../frontend/src/Page/Task/Detail.elm) | ビュー変更 |

### Phase 5: OpenAPI + テスト

| ファイル | 責務 |
|---------|------|
| [`openapi.yaml`](../../../openapi/openapi.yaml) | `UserRef` スキーマ追加、フィールド参照更新 |
| [`create_workflow.hurl`](../../../tests/api/hurl/workflow/create_workflow.hurl) | アサーション修正 |
| [`submit_workflow.hurl`](../../../tests/api/hurl/workflow/submit_workflow.hurl) | アサーション修正 |

## 実装内容

### UserRefDto パターン

```rust
#[derive(Debug, Serialize)]
pub(crate) struct UserRefDto {
    pub id: String,
    pub name: String,
}
```

UUID 文字列の代わりに `{ id, name }` ペアを返すことで、フロントエンドがユーザー名を直接表示できる。

### ユーザー名一括解決

```rust
pub(crate) async fn resolve_user_names(
    user_repository: &impl UserRepository,
    user_ids: &[UserId],
) -> HashMap<UserId, String> {
    // HashSet で重複排除 → find_by_ids で一括取得 → HashMap に変換
}
```

ワークフロー内の全ユーザー ID（`initiated_by` + 各ステップの `assigned_to`）を収集し、1回のクエリで一括取得する。存在しないユーザーは `（不明なユーザー）` にフォールバック。

### State の型パラメータ拡張

```rust
// Before
pub struct WorkflowState<D, I, S> { ... }
// After
pub struct WorkflowState<D, I, S, U> {
    pub usecase: WorkflowUseCaseImpl<D, I, S>,
    pub user_repository: U,
}
```

## テスト

テストケース:
- `test_複数idでユーザーを一括取得できる`
- `test_存在しないidが含まれても取得できるものだけ返す`
- `test_空のid配列を渡すと空vecを返す`

実行方法:
```bash
just test-rust-integration  # 統合テスト（DB 接続が必要）
```

## 設計解説

### 1. ハンドラ層でのユーザー名解決（CQRS Query 側の責務）

場所: [`handler/workflow.rs`](../../../backend/apps/core-service/src/handler/workflow.rs) の `resolve_user_names` 関数

なぜこの設計か:
- ユーザー名はプレゼンテーション層の関心事であり、ドメインロジックには不要
- ユースケース層に `UserRepository` を追加すると、`WorkflowUseCaseImpl<D, I, S>` → `<D, I, S, U>` となり影響範囲が大きい

代替案:
- ドメインサービスで解決する → ドメイン層がインフラ依存になるため却下
- BFF で解決する → Core Service に別 API コールが必要で N+1 問題が発生するため却下

### 2. HashSet による ID 重複排除

場所: [`handler/workflow.rs`](../../../backend/apps/core-service/src/handler/workflow.rs) の `collect_user_ids_from_workflow` 関数

なぜこの設計か:
- `UserId` は `Hash + Eq` を実装しているが `Ord` を実装していない
- `sort()` + `dedup()` パターンは `Ord` が必要なため使用不可
- `HashSet` はこの制約を回避しつつ重複排除を実現する

### 3. From トレイトから明示的な変換関数への移行

場所: [`handler/workflow.rs`](../../../backend/apps/core-service/src/handler/workflow.rs) の `WorkflowStepDto::from_step` 等

なぜこの設計か:
- `From` トレイトは `fn from(value: T) -> Self` というシグネチャが固定されており、追加の引数（ユーザー名マップ）を渡せない
- 明示的な関数にすることで、変換に必要なコンテキストを柔軟に渡せる

代替案:
- ユーザー名マップを構造体に含める → 変換のたびに構造体を作成する必要があり冗長
- `impl From<(DomainEntity, &HashMap)>` → タプルの `From` は可読性が低い

## 関連ドキュメント

- [セッションログ](../../../prompts/runs/2026-02/2026-02-01_0003_ユーザー名解決の実装.md)
- [改善記録: Hurl 仕様未確認のまま推奨](../../../prompts/improvements/2026-02/2026-02-01_0003_Hurl仕様未確認のまま機能を推奨.md)
