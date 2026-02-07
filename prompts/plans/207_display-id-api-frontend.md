# Phase A-3: 表示用 ID API + フロントエンド (#207)

## 概要

Issue #207 の完了基準を達成するため、Core Service / BFF の DTO に `display_id` フィールドを追加し、OpenAPI 仕様を更新し、Elm フロントエンドにデコーダーと UI 表示を実装する。

## スコープ

### 対象

- Core Service DTO（`WorkflowInstanceDto`, `WorkflowSummaryDto`）に `display_id` フィールド追加
- BFF DTO（クライアント・ハンドラ両方）に `display_id` フィールド追加
- OpenAPI 仕様書の更新
- Hurl API テストの更新
- Elm `WorkflowInstance` 型 + デコーダーに `displayId` 追加（TDD）
- Elm `Task.WorkflowSummary` 型 + デコーダーに `displayId` 追加（TDD）
- Elm ワークフロー一覧・詳細・タスク一覧の UI 表示

### 対象外

- WorkflowStep の `display_id`（Phase B のスコープ）
- `display_id` による検索 API（設計書で明示的に対象外）

### Phase A-1/A-2 で達成済みの項目

| 基準 | 状況 |
|------|------|
| DB スキーマ（`display_number` カラム、`display_id_counters` テーブル） | ✅ Phase A-1 |
| ドメイン型（`DisplayNumber`, `DisplayId`, `DisplayIdEntityType`） | ✅ Phase A-2 |
| 採番サービス（`DisplayIdCounterRepository`） | ✅ Phase A-2 |
| ワークフロー作成時に自動採番 | ✅ Phase A-2 |
| リポジトリ save/load | ✅ Phase A-1 |

## 設計判断

### 1. display_id の構築場所

`display_id` 文字列（例: `"WF-42"`）は **Core Service の DTO 変換時**に構築する。

理由:
- ドメインモデルは `display_number: DisplayNumber` を保持
- `DisplayId::new(prefix, number).to_string()` で文字列化
- プレフィックスは `display_prefix::WORKFLOW_INSTANCE`（`"WF"`）定数から取得
- BFF はすでに文字列を受け取るだけ（String 型のパススルー）

### 2. タスク一覧のフィールド名

設計書では `workflow_display_id`（タスク一覧のトップレベル）だが、現在の実装はワークフロー情報をネストされた `workflow` オブジェクト内に持つ構造。

**`workflow.display_id` として追加する**（ネストされた `WorkflowSummaryDto` に `display_id` を追加）。

理由:
- 既存パターンとの一貫性（`workflow.id`, `workflow.title` と同列）
- フロントエンドでは `task.workflow.displayId` でアクセス
- 設計書の意図（ワークフローの表示用 ID をタスク一覧に含める）は満たす

### 3. Elm Pipeline Decoder のフィールド順序

Elm の Pipeline Decoder では、`Decode.succeed Constructor |> required ... |> required ...` のパイプライン順序が型エイリアスのフィールド順序と**一致しなければならない**。

`displayId` は `id` の直後に配置する（UUID → 表示用 ID → その他フィールドの自然な順序）。

## 実装手順

### Step 0: ブランチ + Draft PR

### Step 1: Core Service DTO に `display_id` 追加

**1a. `WorkflowInstanceDto`** (`backend/apps/core-service/src/handler/workflow.rs`)

```rust
pub struct WorkflowInstanceDto {
    pub id: String,
    pub display_id: String,  // 追加
    pub title: String,
    // ...
}
```

`from_instance()` と `from_workflow_with_steps()` の両方で構築:
```rust
use ringiflow_domain::value_objects::{DisplayId, display_prefix};

display_id: DisplayId::new(
    display_prefix::WORKFLOW_INSTANCE,
    instance.display_number(),
).to_string(),
```

**1b. `WorkflowSummaryDto`** (`backend/apps/core-service/src/handler/task.rs`)

```rust
pub struct WorkflowSummaryDto {
    pub id: String,
    pub display_id: String,  // 追加
    pub title: String,
    // ...
}
```

`from_instance()` で同様に構築。

### Step 2: BFF DTO 更新

**2a. BFF クライアント DTO** (`backend/apps/bff/src/client/core_service.rs`)

`WorkflowInstanceDto` (L144) に追加:
```rust
pub display_id: String,  // id の直後
```

`TaskWorkflowSummaryDto` (L179) に追加:
```rust
pub display_id: String,  // id の直後
```

**2b. BFF ハンドラ DTO** (`backend/apps/bff/src/handler/workflow.rs`)

`WorkflowData` (L179) に追加:
```rust
pub display_id: String,  // id の直後
```

`From<WorkflowInstanceDto> for WorkflowData` (L205) に追加:
```rust
display_id: dto.display_id,
```

**2c. BFF タスクハンドラ DTO** (`backend/apps/bff/src/handler/task.rs`)

`TaskWorkflowSummaryData` (L41) に追加:
```rust
pub display_id: String,  // id の直後
```

`From<TaskWorkflowSummaryDto>` (L49) に追加:
```rust
display_id: dto.display_id,
```

### Step 3: OpenAPI 仕様書更新

`openapi/openapi.yaml`:

**3a. `WorkflowInstance` スキーマ** (L922):
- `required` 配列に `display_id` 追加
- `properties` に追加（`id` の直後）:
```yaml
display_id:
  type: string
  description: 表示用 ID（例: WF-42）
  example: "WF-42"
```

**3b. `TaskWorkflowSummary` スキーマ** (L1155):
- `required` 配列に `display_id` 追加
- `properties` に追加（`id` の直後）:
```yaml
display_id:
  type: string
  description: ワークフローの表示用 ID（例: WF-42）
  example: "WF-42"
```

### Step 4: Hurl API テスト更新

**4a. `tests/api/hurl/workflow/create_workflow.hurl`** (L53-66):
```
jsonpath "$.data.display_id" exists
```

**4b. `tests/api/hurl/workflow/submit_workflow.hurl`** (L67-75):
```
jsonpath "$.data.display_id" exists
```

### Step 5: バックエンド全体チェック

```bash
cd backend && cargo sqlx prepare --workspace -- --all-targets
just check-all
```

### Step 6: Elm データ層（TDD）

**6a. `WorkflowInstance` 型** (`frontend/src/Data/WorkflowInstance.elm`)

型エイリアスにフィールド追加（L106、`id` の直後）:
```elm
type alias WorkflowInstance =
    { id : WorkflowInstanceId
    , displayId : String        -- 追加
    , title : String
    -- ...
    }
```

デコーダー更新（L354、`"id"` の直後）:
```elm
decoder =
    Decode.succeed WorkflowInstance
        |> required "id" Decode.string
        |> required "display_id" Decode.string  -- 追加
        |> required "title" Decode.string
        -- ...
```

**6b. `Task.WorkflowSummary` 型** (`frontend/src/Data/Task.elm`)

型エイリアスにフィールド追加（L41、`id` の直後）:
```elm
type alias WorkflowSummary =
    { id : String
    , displayId : String        -- 追加
    , title : String
    -- ...
    }
```

デコーダー更新（L88、`"id"` の直後）:
```elm
workflowSummaryDecoder =
    Decode.succeed WorkflowSummary
        |> required "id" Decode.string
        |> required "display_id" Decode.string  -- 追加
        |> required "title" Decode.string
        -- ...
```

**6c. テスト更新** (`frontend/tests/Data/WorkflowInstanceTest.elm`)

全 JSON フィクスチャに `"display_id": "WF-1"` を追加（`"id"` の直後）。

対象テスト:
- `decoderTests` 全フィールドデコード (L208-221)
- Optional フィールド null (L244-255)
- Optional フィールド省略 (L274-284)
- ステータスデコード (L301-338)
- リストデコーダー (L388-405)

期待値の検証にも `displayId` のアサーション追加。

テストリスト:

| # | テスト | 期待結果 |
|---|--------|---------|
| 1 | 既存テスト全パス | `display_id` 追加後もデコーダーが正常動作 |
| 2 | `displayId` フィールド値検証 | デコード結果に `displayId = "WF-1"` |

### Step 7: Elm ビュー層

**7a. ワークフロー一覧** (`frontend/src/Page/Workflow/List.elm`)

テーブルヘッダーに「ID」列を追加し、各行で `workflow.displayId` を表示。

`viewWorkflowTable` (L269) のヘッダーに列追加:
```elm
th [] [ text "ID" ]
```

`viewWorkflowRow` (L284) にセル追加:
```elm
td [] [ text workflow.displayId ]
```

**7b. ワークフロー詳細** (`frontend/src/Page/Workflow/Detail.elm`)

`viewTitle` (L387) でタイトルの前に表示用 ID を表示:
```elm
viewTitle workflow =
    h1 [] [ text (workflow.displayId ++ " " ++ workflow.title) ]
```

または基本情報セクション `viewBasicInfo` (L403) に行を追加。

**7c. タスク一覧** (`frontend/src/Page/Task/List.elm`)

`viewTaskRow` (L191) でワークフロータイトルの横に `task.workflow.displayId` を表示:
```elm
td [] [ text (task.workflow.displayId ++ " " ++ task.workflow.title) ]
```

### Step 8: フロントエンドチェック

```bash
cd frontend && pnpm run test
cd frontend && pnpm run build
```

### Step 9: 全体チェック + Ready for Review

```bash
just check-all
```

Issue #207 チェックボックス更新 + `gh pr ready`

## 修正対象ファイル一覧

| ファイル | 種類 | 修正内容 |
|---------|------|---------|
| `backend/apps/core-service/src/handler/workflow.rs` | 修正 | `WorkflowInstanceDto` に `display_id` + 構築ロジック |
| `backend/apps/core-service/src/handler/task.rs` | 修正 | `WorkflowSummaryDto` に `display_id` + 構築ロジック |
| `backend/apps/bff/src/client/core_service.rs` | 修正 | `WorkflowInstanceDto`, `TaskWorkflowSummaryDto` に `display_id` |
| `backend/apps/bff/src/handler/workflow.rs` | 修正 | `WorkflowData` に `display_id` + From impl |
| `backend/apps/bff/src/handler/task.rs` | 修正 | `TaskWorkflowSummaryData` に `display_id` + From impl |
| `openapi/openapi.yaml` | 修正 | `WorkflowInstance`, `TaskWorkflowSummary` に `display_id` |
| `tests/api/hurl/workflow/create_workflow.hurl` | 修正 | `display_id` exists アサーション |
| `tests/api/hurl/workflow/submit_workflow.hurl` | 修正 | `display_id` exists アサーション |
| `frontend/src/Data/WorkflowInstance.elm` | 修正 | 型 + デコーダーに `displayId` |
| `frontend/src/Data/Task.elm` | 修正 | `WorkflowSummary` 型 + デコーダーに `displayId` |
| `frontend/tests/Data/WorkflowInstanceTest.elm` | 修正 | JSON フィクスチャ + アサーション更新 |
| `frontend/src/Page/Workflow/List.elm` | 修正 | テーブルに ID 列追加 |
| `frontend/src/Page/Workflow/Detail.elm` | 修正 | タイトルまたは基本情報に表示用 ID |
| `frontend/src/Page/Task/List.elm` | 修正 | ワークフロータイトルに表示用 ID 併記 |

## 完了基準マッピング

| # | 基準 | 達成手段 |
|---|------|---------|
| 1 | API レスポンスに `display_id` フィールド（ワークフロー一覧・詳細） | Step 1a + 2 |
| 2 | タスク一覧 API に `workflow_display_id` フィールド | Step 1b + 2（`workflow.display_id` として） |
| 3 | OpenAPI 仕様書が更新されている | Step 3 |
| 4 | フロントエンド: ワークフロー一覧に表示用 ID（E2E） | Step 6a + 7a |
| 5 | フロントエンド: ワークフロー詳細に表示用 ID（E2E） | Step 6a + 7b |
| 6 | フロントエンド: タスク一覧にワークフローの表示用 ID（E2E） | Step 6b + 7c |

## 検証方法

```bash
just check-all  # lint + unit + integration + API test
just dev-all    # E2E 手動確認
```

E2E 確認ポイント:
1. ワークフロー一覧画面に「WF-N」形式の ID が表示される
2. ワークフロー詳細画面のタイトル付近に表示用 ID が表示される
3. タスク一覧画面のワークフロー情報に表示用 ID が表示される
