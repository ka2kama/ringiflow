# Issue #37: タスク一覧・詳細画面 — 実装計画

## 概要

「タスク」= WorkflowStep のユーザー向けビュー（自分にアサインされたアクティブな承認ステップ）。
既存のドメインモデル・リポジトリを活用し、API + フロントエンドレイヤーを追加する。

## 前提作業: Issue 更新

Issue #37 の完了基準を E2E 視点を含むよう更新:

```
- [ ] GET /api/v1/tasks/my で自分のタスク一覧を取得できる
- [ ] GET /api/v1/tasks/{id} でタスク詳細を取得できる
- [ ] フロントエンドでタスク一覧画面が動作する
- [ ] フロントエンドでタスク詳細画面から承認/却下できる
- [ ] ユーザーがタスク一覧にアクセスし、自分の承認待ちタスクを確認できる（E2E）
- [ ] ユーザーがタスク一覧からタスクを選択し、詳細画面で承認操作を完了できる（E2E）
```

## Phase 分割

### Phase 1: リポジトリ層 — `find_by_ids` 追加

`WorkflowInstanceRepository` に `find_by_ids` メソッドを追加。
タスク一覧で workflow title を取得するために必要。

**変更ファイル:**
- `backend/crates/infra/src/repository/workflow_instance_repository.rs` — トレイト + PostgreSQL 実装に追加
- `backend/crates/infra/tests/workflow_instance_repository_test.rs` — 統合テスト追加

**テストリスト:**
- [ ] 空の Vec を渡すと空の Vec が返る
- [ ] 存在する ID を渡すとインスタンスが返る
- [ ] 存在しない ID を含んでも、存在するもののみ返る
- [ ] テナント ID でフィルタされる（他テナントのインスタンスは返らない）

**SQL:** `WHERE id = ANY($1) AND tenant_id = $2`

**`just sqlx-prepare` の実行が必要**

---

### Phase 2: Core Service — タスクユースケース + ハンドラ

タスク一覧・詳細の内部 API を実装。

**新規ファイル:**
- `backend/apps/core-service/src/usecase/task.rs` — `TaskUseCaseImpl<I, S>` (instance_repo + step_repo)
- `backend/apps/core-service/src/handler/task.rs` — `TaskState<I, S>` + ハンドラ

**変更ファイル:**
- `backend/apps/core-service/src/usecase.rs` — `pub mod task;` + re-export
- `backend/apps/core-service/src/handler.rs` — `pub mod task;` + re-export
- `backend/apps/core-service/src/main.rs` — ルート追加 + TaskState 初期化

**ルート:**
- `GET /internal/tasks/my` — `Query<UserQuery>` で tenant_id + user_id
- `GET /internal/tasks/{id}` — `Path<Uuid>` + `Query<UserQuery>`

**State 設計:**
Core Service では `TaskState<I, S>` を新規作成（`WorkflowState` とは別）。
既存パターン: `UserState` と `WorkflowState` が分離済み。

```rust
pub struct TaskState<I, S> {
    pub usecase: TaskUseCaseImpl<I, S>,
}
```

**ユースケースロジック:**

`list_my_tasks`:
1. `step_repo.find_by_assigned_to(tenant_id, user_id)` でステップ取得
2. Active のみフィルタ
3. `instance_repo.find_by_ids(instance_ids, tenant_id)` でインスタンス一括取得
4. ステップ + インスタンスを結合して返す

`get_task`:
1. `step_repo.find_by_id(step_id, tenant_id)` でステップ取得
2. 権限チェック: `step.assigned_to == Some(user_id)`
3. `instance_repo.find_by_id(instance_id, tenant_id)` でインスタンス取得
4. `step_repo.find_by_instance(instance_id, tenant_id)` で全ステップ取得
5. ステップ + ワークフロー（全ステップ含む）を返す

**テストリスト:**
- [ ] list_my_tasks: Active なステップのみ返る
- [ ] list_my_tasks: workflow title がタスクに含まれる
- [ ] list_my_tasks: 他ユーザーのタスクは返らない
- [ ] list_my_tasks: タスクがない場合は空 Vec
- [ ] get_task: 正常系 — step + workflow が返る
- [ ] get_task: step が見つからない場合は NotFound
- [ ] get_task: 他ユーザーのタスクは Forbidden

---

### Phase 3: BFF — タスクプロキシエンドポイント

フロントエンド向けの `/api/v1/tasks/*` を実装。

**新規ファイル:**
- `backend/apps/bff/src/handler/task.rs` — BFF タスクハンドラ

**変更ファイル:**
- `backend/apps/bff/src/client/core_service.rs` — `CoreServiceClient` トレイト + 実装にメソッド追加、レスポンス DTO 追加
- `backend/apps/bff/src/handler.rs` — `pub mod task;` + re-export
- `backend/apps/bff/src/main.rs` — ルート追加

**BFF State:** 既存の `WorkflowState<C, S>` を再利用（同じ依存: core_service_client + session_manager）。

**ルート:**
- `GET /api/v1/tasks/my` → Core Service `/internal/tasks/my`
- `GET /api/v1/tasks/{id}` → Core Service `/internal/tasks/{id}`

**CoreServiceClient 追加メソッド:**
- `list_my_tasks(tenant_id, user_id) -> Result<TaskListResponse, CoreServiceError>`
- `get_task(tenant_id, user_id, task_id) -> Result<TaskDetailResponse, CoreServiceError>`

---

### Phase 4: OpenAPI 仕様書更新

**変更ファイル:**
- `openapi/openapi.yaml`

追加するエンドポイント + スキーマ:
- `GET /api/v1/tasks/my` — TaskListResponse
- `GET /api/v1/tasks/{id}` — TaskDetailResponse
- `TaskItem` スキーマ（step + WorkflowSummary）
- `TaskDetail` スキーマ（step + WorkflowInstance）

---

### Phase 5: フロントエンド — データ型 + API クライアント + ルーティング

**新規ファイル:**
- `frontend/src/Data/Task.elm` — Task, WorkflowSummary, TaskDetail 型 + Decoder
- `frontend/src/Api/Task.elm` — listMyTasks, getTask

**変更ファイル:**
- `frontend/src/Route.elm` — `Tasks`, `TaskDetail String` バリアント追加
- `frontend/src/Main.elm` — Page 型 + init/update/view に Task ページ追加

**Route 追加:**
```
/tasks      → Tasks
/tasks/{id} → TaskDetail id
```

---

### Phase 6: フロントエンド — タスク一覧ページ (SCR-007)

**新規ファイル:**
- `frontend/src/Page/Task/List.elm`

**UI 構成:**
- ヘッダー: 「タスク一覧」
- テーブル: タスク名(step_name), 申請タイトル(workflow.title), ステータス, 期限, 開始日
- 各行 → `/tasks/{id}` にリンク
- 空状態: 「承認待ちのタスクはありません」

**テストリスト:**
- [ ] 初期化時に API が呼ばれる
- [ ] 成功時にタスク一覧が表示される
- [ ] タスクがない場合に空メッセージ
- [ ] エラー時にエラーメッセージ + 再読み込みボタン

---

### Phase 7: フロントエンド — タスク詳細ページ (SCR-008) + ナビゲーション

**新規ファイル:**
- `frontend/src/Page/Task/Detail.elm`

**UI 構成:**
- 戻るリンク: 「← タスク一覧に戻る」
- ワークフロー情報: タイトル, 申請者, フォームデータ
- 進捗表示: 全ステップの状態
- 承認/却下ボタン（Active ステップの場合のみ）
- コメント入力欄

**承認/却下:** 既存の `Api.Workflow.approveStep` / `rejectStep` を直接再利用。

**ナビゲーション更新:**
- `Main.elm` のヘッダーまたはホームページにタスク一覧へのリンクを追加

**テストリスト:**
- [ ] 初期化時に API が呼ばれる
- [ ] タスク詳細が表示される
- [ ] 承認ボタンクリックで承認 API が呼ばれる
- [ ] 却下ボタンクリックで却下 API が呼ばれる
- [ ] 409 Conflict でエラーメッセージ
- [ ] Active でないステップには承認/却下ボタン非表示

---

## API レスポンス型

### タスク一覧: `GET /api/v1/tasks/my`

```json
{
  "data": [
    {
      "id": "step-uuid",
      "step_name": "部長承認",
      "status": "active",
      "version": 1,
      "assigned_to": "user-uuid",
      "due_date": null,
      "started_at": "2026-01-29T10:00:00Z",
      "created_at": "2026-01-29T10:00:00Z",
      "workflow": {
        "id": "instance-uuid",
        "title": "経費精算申請",
        "status": "in_progress",
        "initiated_by": "applicant-uuid",
        "submitted_at": "2026-01-29T09:30:00Z"
      }
    }
  ]
}
```

### タスク詳細: `GET /api/v1/tasks/{id}`

```json
{
  "data": {
    "step": { "...全 WorkflowStep フィールド..." },
    "workflow": { "...全 WorkflowInstance フィールド + steps..." }
  }
}
```

## 変更ファイル一覧

### 新規 (7 ファイル)

| Phase | ファイル |
|-------|---------|
| 2 | `backend/apps/core-service/src/usecase/task.rs` |
| 2 | `backend/apps/core-service/src/handler/task.rs` |
| 3 | `backend/apps/bff/src/handler/task.rs` |
| 5 | `frontend/src/Data/Task.elm` |
| 5 | `frontend/src/Api/Task.elm` |
| 6 | `frontend/src/Page/Task/List.elm` |
| 7 | `frontend/src/Page/Task/Detail.elm` |

### 変更 (10 ファイル)

| Phase | ファイル | 変更概要 |
|-------|---------|---------|
| 1 | `backend/crates/infra/src/repository/workflow_instance_repository.rs` | `find_by_ids` 追加 |
| 1 | `backend/crates/infra/tests/workflow_instance_repository_test.rs` | 統合テスト |
| 2 | `backend/apps/core-service/src/usecase.rs` | `pub mod task` + re-export |
| 2 | `backend/apps/core-service/src/handler.rs` | `pub mod task` + re-export |
| 2 | `backend/apps/core-service/src/main.rs` | ルート + TaskState 初期化 |
| 3 | `backend/apps/bff/src/client/core_service.rs` | トレイト + 実装 + DTO |
| 3 | `backend/apps/bff/src/handler.rs` | `pub mod task` + re-export |
| 3 | `backend/apps/bff/src/main.rs` | ルート追加 |
| 4 | `openapi/openapi.yaml` | エンドポイント + スキーマ追加 |
| 5 | `frontend/src/Route.elm` | Tasks, TaskDetail 追加 |
| 5-7 | `frontend/src/Main.elm` | Page 統合 + ナビゲーション |

## 検証方法

各 Phase 完了時:
1. `just check-all` で lint + テスト通過を確認
2. Phase 1: `just test-rust-integration` で DB 統合テスト
3. Phase 3 完了後: `just dev-all` で開発サーバー起動、curl で API 確認
4. Phase 7 完了後: ブラウザでタスク一覧→詳細→承認の E2E フロー確認
