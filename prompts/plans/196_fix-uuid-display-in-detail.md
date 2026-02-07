# Issue #196: 申請詳細画面でユーザーIDがUUID表示されている問題を修正する

## 概要

API レスポンスの `initiated_by` と `assigned_to` を UUID 文字列からユーザー参照オブジェクト `{ id, name }` に変更し、フロントエンドでユーザー名を表示する。

## 設計判断: ユーザー名解決の実装場所

**Core Service ハンドラ層で名前解決する（CQRS の Query 側の責務）**

理由:
- ユーザー名は「表示用の補足情報」であり、ドメインのビジネスロジック（状態遷移、権限検証）には不要
- ユースケース層に `UserRepository` を追加すると型パラメータ `<D, I, S>` → `<D, I, S, U>` に増え、全ハンドラ・ルーター・テストに波及する
- ヘルパー関数に抽出して肥大化を防ぐ

## Phase 分割

### Phase 1: UserRepository に `find_by_ids` を追加

**ファイル:**
- `backend/crates/infra/src/repository/user_repository.rs` - トレイト + 実装
- `backend/crates/infra/tests/user_repository_test.rs` - 統合テスト

**テストリスト:**
- [ ] 複数IDでユーザーを一括取得できる
- [ ] 存在しないIDが含まれても取得できるものだけ返す
- [ ] 空のID配列を渡すと空Vecを返す

**`just sqlx-prepare` の実行が必要**

### Phase 2: Core Service DTO 変更 + ユーザー名解決

**ファイル:**
- `backend/apps/core-service/src/handler/workflow.rs` - `UserRefDto` 導入、DTO 変更、ヘルパー追加
- `backend/apps/core-service/src/handler/task.rs` - DTO 変更
- `backend/apps/core-service/src/main.rs` - `WorkflowState` / `TaskState` に `UserRepository` 追加

**変更内容:**

1. `UserRefDto { id: String, name: String }` を導入
2. `WorkflowState<D, I, S>` → `WorkflowState<D, I, S, U>` に拡張
3. `TaskState<I, S>` → `TaskState<I, S, U>` に拡張
4. DTO のフィールド変更:
   - `WorkflowInstanceDto.initiated_by`: `String` → `UserRefDto`
   - `WorkflowStepDto.assigned_to`: `Option<String>` → `Option<UserRefDto>`
   - `WorkflowSummaryDto.initiated_by`: `String` → `UserRefDto`
   - `TaskItemDto.assigned_to`: `Option<String>` → `Option<UserRefDto>`
5. `resolve_user_names` ヘルパー関数を追加
6. `From` トレイト → 明示的な変換関数に変更（ユーザー名マップを引数に取るため）
7. 存在しないユーザーは `UserRefDto { id, name: "（不明なユーザー）" }` にフォールバック

### Phase 3: BFF レスポンス型の更新

**ファイル:**
- `backend/apps/bff/src/client/core_service.rs` - デシリアライズ型更新
- `backend/apps/bff/src/handler/workflow.rs` - `WorkflowData` 更新
- `backend/apps/bff/src/handler/task.rs` - タスク関連型更新

**変更内容:**

1. `UserRefDto { id: String, name: String }` (Deserialize) を追加
2. `UserRefData { id: String, name: String }` (Serialize) を追加
3. 各レスポンス型で `String` → `UserRefDto` / `UserRefData` に変更

### Phase 4: Elm フロントエンド更新

**ファイル:**
- `frontend/src/Data/UserRef.elm` - 新規作成（型 + デコーダー）
- `frontend/src/Data/WorkflowInstance.elm` - 型 + デコーダー変更
- `frontend/src/Data/Task.elm` - 型 + デコーダー変更
- `frontend/src/Page/Workflow/Detail.elm` - 表示更新
- `frontend/src/Page/Task/Detail.elm` - 表示更新

**変更内容:**

1. `UserRef` 型とデコーダーを作成
2. `WorkflowInstance.initiatedBy`: `String` → `UserRef`
3. `WorkflowStep.assignedTo`: `Maybe String` → `Maybe UserRef`
4. `WorkflowSummary.initiatedBy`: `String` → `UserRef`
5. `TaskItem.assignedTo`: `Maybe String` → `Maybe UserRef`
6. ビュー: `workflow.initiatedBy` → `workflow.initiatedBy.name`
7. ビュー: `assignee` → `assignee.name`
8. ID 比較ロジック: `step.assignedTo == Just userId` → `Maybe.map .id step.assignedTo == Just userId`

### Phase 5: OpenAPI 仕様書更新 + 最終確認

**ファイル:**
- `openapi/openapi.yaml`

**変更内容:**
1. `UserRef` スキーマを追加
2. `WorkflowInstance.initiated_by` / `WorkflowStep.assigned_to` 等のスキーマを変更

## 検証方法

1. `just check-all` が通ること
2. 手動テスト: ログイン → ワークフロー詳細画面 → ユーザー名表示を確認
3. タスク一覧/詳細画面でもユーザー名が表示されることを確認
