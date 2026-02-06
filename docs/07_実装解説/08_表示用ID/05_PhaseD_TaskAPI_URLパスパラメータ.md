# Phase D: Task API の URL パスパラメータに display_number を使用

## 概要

タスク詳細取得 API の URL パスパラメータを UUID から階層的な display_number 形式に変更した。

```
Before: GET /api/v1/tasks/{uuid}
After:  GET /api/v1/workflows/{workflow_display_number}/tasks/{step_display_number}
```

WorkflowStep の display_number はワークフロー内でのみ一意であるため、階層的 URL 設計を採用した。

### 対応 Issue

[#247 タスク API の URL パスパラメータに表示用番号を使用する](https://github.com/ka2kama/ringiflow/issues/247)

## 設計書との対応

- [表示用 ID 設計](../../../03_詳細設計書/12_表示用ID設計.md)

## 実装したコンポーネント

### Phase 1: Core Service

| ファイル | 責務 |
|----------|------|
| [`usecase/task.rs`](../../../backend/apps/core-service/src/usecase/task.rs) | `get_task_by_display_numbers` ユースケース追加 |
| [`handler/task.rs`](../../../backend/apps/core-service/src/handler/task.rs) | 新規エンドポイント追加、DTO に display_number 追加 |

新規エンドポイント:

```
GET /internal/workflows/by-display-number/{dn}/tasks/by-display-number/{step_dn}
```

### Phase 2: BFF

| ファイル | 責務 |
|----------|------|
| [`client/core_service.rs`](../../../backend/apps/bff/src/client/core_service.rs) | `get_task_by_display_numbers` クライアントメソッド追加 |
| [`handler/task.rs`](../../../backend/apps/bff/src/handler/task.rs) | レスポンス DTO に display_number 追加 |
| [`handler/workflow.rs`](../../../backend/apps/bff/src/handler/workflow.rs) | タスク詳細取得ハンドラをワークフローモジュールに追加 |

公開エンドポイント:

```
GET /api/v1/workflows/{display_number}/tasks/{step_display_number}
```

### Phase 3: フロントエンド

| ファイル | 責務 |
|----------|------|
| [`Route.elm`](../../../frontend/src/Route.elm) | `TaskDetail String` → `TaskDetail Int Int` |
| [`Api/Task.elm`](../../../frontend/src/Api/Task.elm) | `getTaskByDisplayNumbers` に変更、URL を階層的に |
| [`Data/Task.elm`](../../../frontend/src/Data/Task.elm) | TaskItem, WorkflowSummary に display_number 追加 |
| [`Page/Task/Detail.elm`](../../../frontend/src/Page/Task/Detail.elm) | モデルを `taskId` → `workflowDisplayNumber, stepDisplayNumber` に変更 |
| [`Page/Task/List.elm`](../../../frontend/src/Page/Task/List.elm) | リンク生成を display_number ベースに変更 |

### Phase 4: OpenAPI + API テスト

| ファイル | 責務 |
|----------|------|
| [`openapi.yaml`](../../../openapi/openapi.yaml) | エンドポイント定義変更、display_number スキーマ追加 |
| [`get_task_by_display_numbers.hurl`](../../../tests/api/hurl/task/get_task_by_display_numbers.hurl) | 新規 E2E テスト |
| [`list_my_tasks.hurl`](../../../tests/api/hurl/task/list_my_tasks.hurl) | display_number アサーション追加 |

## 実装内容

### Core Service: get_task_by_display_numbers ユースケース

```rust
pub async fn get_task_by_display_numbers(
    &self,
    workflow_display_number: DisplayNumber,
    step_display_number: DisplayNumber,
    tenant_id: &TenantId,
    user_id: &UserId,
) -> Result<TaskDetail, TaskError> {
    // ワークフローを display_number で取得
    let workflow = self.workflow_repo
        .find_by_display_number(workflow_display_number, tenant_id)
        .await?
        .ok_or(TaskError::NotFound)?;

    // 該当するステップを検索
    let step = workflow.steps().iter()
        .find(|s| s.display_number() == step_display_number)
        .ok_or(TaskError::NotFound)?;

    // 権限チェック（担当者のみ取得可能）
    if step.assigned_to().map(|a| a.id()) != Some(user_id) {
        return Err(TaskError::Forbidden);
    }

    Ok(TaskDetail { step, workflow })
}
```

### フロントエンド: Route の型変更

```elm
-- Before
type Route
    = TaskDetail String
    ...

-- After
type Route
    = TaskDetail Int Int  -- workflowDisplayNumber, stepDisplayNumber
    ...

parser : Parser (Route -> a) a
parser =
    oneOf
        [ map TaskDetail (s "workflows" </> int </> s "tasks" </> int)
        ...
        ]
```

### フロントエンド: タスク一覧からのリンク生成

```elm
-- Page/Task/List.elm
taskRow : Task.TaskItem -> Html Msg
taskRow task =
    a [ Route.href (Route.TaskDetail task.workflow.displayNumber task.displayNumber) ]
        [ ... ]
```

## テスト

### API テスト

```bash
just check-all
```

| ファイル | テスト内容 |
|----------|-----------|
| `get_task_by_display_numbers.hurl` | 正常系: タスク詳細取得、異常系: 権限なし (403)、存在しない (404) |
| `list_my_tasks.hurl` | タスク一覧に display_number が含まれることを検証 |

## 関連ドキュメント

- [表示用 ID 設計](../../../docs/03_詳細設計書/12_表示用ID設計.md)
- [Phase C: URL パスパラメータ](./04_PhaseC_URLパスパラメータ.md)

---

## 設計解説

### 1. 階層的 URL 設計

**場所**: BFF エンドポイント、フロントエンド Route

**なぜこの設計か**:

WorkflowStep の display_number は**ワークフロー内でのみ一意**であり、グローバル一意ではない。

| ID の種類 | スコープ | URL パターン |
|-----------|---------|--------------|
| UUID | グローバル一意 | `/tasks/{uuid}` |
| display_number | 親スコープ内で一意 | `/workflows/{wf_dn}/tasks/{step_dn}` |

**参考事例**:

GitHub の API も同様のパターンを採用している:

```
# Issue コメント（コメント ID はリポジトリ内で一意）
GET /repos/{owner}/{repo}/issues/{issue_number}/comments/{comment_id}

# PR レビューコメント
GET /repos/{owner}/{repo}/pulls/{pull_number}/comments/{comment_id}
```

**採用理由**:

1. **曖昧性の排除**: `GET /tasks/1` だけでは、どのワークフローのステップ 1 か不明
2. **RESTful**: リソースの親子関係が URL で表現される
3. **一貫性**: Workflow API（Phase C）と同じパターン

### 2. タスク一覧レスポンスへの display_number 追加

**場所**: Core Service DTO、BFF レスポンス、フロントエンド Model

**コード例**:

```json
{
  "data": [{
    "id": "01924f3e-...",
    "display_number": 1,
    "step_name": "承認",
    "workflow": {
      "id": "01924f3e-...",
      "display_id": "WF-42",
      "display_number": 42,
      "title": "経費申請"
    }
  }]
}
```

**なぜこの設計か**:

タスク一覧からタスク詳細へのリンクを生成するために、両方の display_number が必要:

```elm
Route.TaskDetail task.workflow.displayNumber task.displayNumber
-- → /workflows/42/tasks/1
```

displayId（"WF-42"）をパースして取得する方法もあるが、Phase C と同様に API が直接値を提供する設計を採用した。

### 3. Elm のダブル Int パターンマッチ

**場所**: `Route.elm`, `Main.elm`

**コード例**:

```elm
-- Route.elm
type Route
    = TaskDetail Int Int  -- 2つの Int を持つバリアント

-- Main.elm (パターンマッチ)
case route of
    Route.TaskDetail workflowDn stepDn ->
        -- 両方の値を使用
```

**なぜこの設計か**:

Elm の型システムにより、以下が保証される:

1. **コンパイル時チェック**: パターンマッチで `TaskDetail _` のように 1 つだけ書くとエラー
2. **変更追跡**: `TaskDetail String` → `TaskDetail Int Int` の変更で、全ての関連箇所をコンパイラが検出
3. **型安全**: 整数以外の値を渡すことが不可能

これは「型で表現できるものは型で表現する」というプロジェクトの設計原則に合致している。
