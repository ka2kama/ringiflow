# Phase A-3: API + フロントエンド

## 概要

表示用 ID（`WF-42` 形式）を API レスポンスとフロントエンドに反映した。Core Service DTO → BFF DTO → OpenAPI 仕様 → Hurl テスト → Elm デコーダー → Elm UI の 6 レイヤーにわたるフルスタック変更。

### 対応 Issue

[#207 表示用 ID API + フロントエンド](https://github.com/ka2kama/ringiflow/issues/207)
PR: [#220](https://github.com/ka2kama/ringiflow/pull/220)

### 設計書との対応

- [表示用 ID 設計 > API 仕様変更](../../03_詳細設計書/12_表示用ID設計.md#api-仕様変更)
- [表示用 ID 設計 > フロントエンド対応](../../03_詳細設計書/12_表示用ID設計.md#フロントエンド対応)

## 実装したコンポーネント

### バックエンド

| ファイル | 責務 |
|---------|------|
| [`backend/apps/core-service/src/handler/workflow.rs`](../../../backend/apps/core-service/src/handler/workflow.rs) | `WorkflowInstanceDto` に `display_id` + 構築ロジック |
| [`backend/apps/core-service/src/handler/task.rs`](../../../backend/apps/core-service/src/handler/task.rs) | `WorkflowSummaryDto` に `display_id` + 構築ロジック |
| [`backend/apps/bff/src/client/core_service.rs`](../../../backend/apps/bff/src/client/core_service.rs) | クライアント DTO に `display_id` パススルー |
| [`backend/apps/bff/src/handler/workflow.rs`](../../../backend/apps/bff/src/handler/workflow.rs) | ハンドラ DTO に `display_id` + `From` impl |
| [`backend/apps/bff/src/handler/task.rs`](../../../backend/apps/bff/src/handler/task.rs) | ハンドラ DTO に `display_id` + `From` impl |

### API 仕様・テスト

| ファイル | 責務 |
|---------|------|
| [`openapi/openapi.yaml`](../../../openapi/openapi.yaml) | `WorkflowInstance`, `TaskWorkflowSummary` スキーマに `display_id` |
| [`tests/api/hurl/workflow/create_workflow.hurl`](../../../tests/api/hurl/workflow/create_workflow.hurl) | `display_id` exists アサーション |
| [`tests/api/hurl/workflow/submit_workflow.hurl`](../../../tests/api/hurl/workflow/submit_workflow.hurl) | `display_id` exists アサーション |

### フロントエンド

| ファイル | 責務 |
|---------|------|
| [`frontend/src/Data/WorkflowInstance.elm`](../../../frontend/src/Data/WorkflowInstance.elm) | `WorkflowInstance` 型 + デコーダーに `displayId` |
| [`frontend/src/Data/Task.elm`](../../../frontend/src/Data/Task.elm) | `WorkflowSummary` 型 + デコーダーに `displayId` |
| [`frontend/tests/Data/WorkflowInstanceTest.elm`](../../../frontend/tests/Data/WorkflowInstanceTest.elm) | JSON フィクスチャ + アサーション更新 |
| [`frontend/src/Page/Workflow/List.elm`](../../../frontend/src/Page/Workflow/List.elm) | テーブルに ID 列追加 |
| [`frontend/src/Page/Workflow/Detail.elm`](../../../frontend/src/Page/Workflow/Detail.elm) | タイトル横に表示用 ID |
| [`frontend/src/Page/Task/List.elm`](../../../frontend/src/Page/Task/List.elm) | ワークフロータイトルに表示用 ID 併記 |

## 実装内容

### Core Service DTO での display_id 構築

```rust
// backend/apps/core-service/src/handler/workflow.rs:232-233
display_id: DisplayId::new(display_prefix::WORKFLOW_INSTANCE, instance.display_number())
    .to_string(),
```

ドメインモデルの `display_number()`（`DisplayNumber` 型）を取得し、`DisplayId::new()` で値オブジェクトを生成、`to_string()` で `"WF-42"` 形式の文字列に変換する。

この構築ロジックは `from_instance()`（一覧用）と `from_workflow_with_steps()`（詳細用）の両方に実装。タスク一覧の `WorkflowSummaryDto` にも同様の構築ロジックを追加。

### BFF のパススルー

BFF のクライアント DTO（`Deserialize`）とハンドラ DTO（`Serialize`）に `pub display_id: String` を追加し、`From` impl で `display_id: dto.display_id` とパススルーする。BFF はプレゼンテーション変換を行わない。

### Elm デコーダー（TDD）

```elm
-- frontend/src/Data/WorkflowInstance.elm:107
type alias WorkflowInstance =
    { id : WorkflowInstanceId
    , displayId : String        -- 追加
    , title : String
    -- ...

-- デコーダー（Pipeline Decoder）
decoder =
    Decode.succeed WorkflowInstance
        |> required "id" Decode.string
        |> required "display_id" Decode.string  -- 追加
        |> required "title" Decode.string
```

TDD で実装: テストフィクスチャに `"display_id": "WF-1"` を追加してテスト失敗（Red）→ 型 + デコーダーに `displayId` を追加してテスト成功（Green）。

### UI 表示

ワークフロー一覧: ID 列を独立したカラムとして追加。

```elm
-- frontend/src/Page/Workflow/List.elm:288
td [ class "px-4 py-3 text-sm text-secondary-500" ] [ text workflow.displayId ]
```

ワークフロー詳細: タイトル横に薄い色で表示用 ID を表示。

```elm
-- frontend/src/Page/Workflow/Detail.elm:388-392
viewTitle workflow =
    h1 [ class "text-2xl font-bold text-secondary-900" ]
        [ span [ class "text-secondary-400 mr-2" ] [ text workflow.displayId ]
        , text workflow.title
        ]
```

タスク一覧: ワークフロータイトルの前に表示用 ID を併記。

```elm
-- frontend/src/Page/Task/List.elm:198
td [ class "px-4 py-3" ] [ text (task.workflow.displayId ++ " " ++ task.workflow.title) ]
```

## テスト

| テスト | 場所 | 内容 |
|--------|------|------|
| 全フィールドをデコード | `frontend/tests/Data/WorkflowInstanceTest.elm` | `displayId` フィールドの追加後もデコーダーが正常動作 |
| Optional フィールド null | 同上 | `display_id` 必須フィールドの存在下で Optional フィールドが null でもデコード可能 |
| ステータスデコード | 同上 | 各ステータスのデコードテストに `display_id` を追加 |
| Hurl API テスト | `tests/api/hurl/workflow/create_workflow.hurl` | ワークフロー作成 API に `display_id` が存在 |
| Hurl API テスト | `tests/api/hurl/workflow/submit_workflow.hurl` | ワークフロー申請 API に `display_id` が存在 |

## 関連ドキュメント

- [Phase A-1: DB スキーマ変更](./01_PhaseA1_DBスキーマ変更.md)
- [Phase A-2: 採番サービス](./02_PhaseA2_採番サービス.md)
- [表示用 ID 設計](../../03_詳細設計書/12_表示用ID設計.md)
- [OpenAPI 仕様書](../../../openapi/openapi.yaml)

---

## 設計解説

### 1. display_id の構築場所を Core Service の DTO 変換層に配置

場所: [`backend/apps/core-service/src/handler/workflow.rs:232-233`](../../../backend/apps/core-service/src/handler/workflow.rs)

```rust
display_id: DisplayId::new(display_prefix::WORKFLOW_INSTANCE, instance.display_number())
    .to_string(),
```

なぜこの設計か: ドメインモデル（`WorkflowInstance`）は `display_number: DisplayNumber`（連番）を保持し、表示形式（`"WF-42"`）はプレゼンテーションの関心事として DTO 変換時に生成する。BFF は `String` としてパススルーするだけで、変換ロジックを持たない。

代替案:
- ドメインモデルに `display_id()` メソッドを追加: プレフィックスがドメイン層の関心になってしまう。プレフィックスの表示ルール変更がドメインモデルに波及する
- BFF で構築: Core Service が返す JSON にプレフィックスと番号を別々に含め、BFF で結合する。レイヤー間の責務分担が不明確になる
- DB に `display_id` カラムを持つ: 冗長だが、検索時に便利。現時点では検索要件がないため Phase B 以降で検討

### 2. タスク一覧の display_id をネスト構造に配置

場所: [`backend/apps/core-service/src/handler/task.rs:60`](../../../backend/apps/core-service/src/handler/task.rs)

```rust
pub struct WorkflowSummaryDto {
    pub id:           String,
    pub display_id:   String,  // ネスト内に配置
    pub title:        String,
    // ...
}
```

なぜこの設計か: 設計書では `workflow_display_id`（トップレベル）を定義していたが、既存実装はワークフロー情報を `workflow` オブジェクトにネストしている。`workflow.display_id` として追加することで、`workflow.id`, `workflow.title` と同列の構造を維持。フロントエンドでは `task.workflow.displayId` でアクセスする。

代替案:
- 設計書通りトップレベルに配置: 既存のネスト構造と不整合が生じ、API の一貫性が低下する
- 両方に配置: 冗長。二重管理のリスク

### 3. Elm Pipeline Decoder のフィールド順序

場所: [`frontend/src/Data/WorkflowInstance.elm:107, 354-357`](../../../frontend/src/Data/WorkflowInstance.elm)

```elm
type alias WorkflowInstance =
    { id : WorkflowInstanceId
    , displayId : String        -- id の直後
    , title : String

decoder =
    Decode.succeed WorkflowInstance
        |> required "id" Decode.string
        |> required "display_id" Decode.string  -- id の直後
        |> required "title" Decode.string
```

なぜこの設計か: Elm の `Decode.succeed Constructor |> required ...` パイプラインでは、パイプラインの順序が型エイリアスのフィールド定義順と一致しなければならない。これは `Decode.succeed` がコンストラクタ関数（位置引数）を受け取るためで、型エイリアスのフィールド順 = コンストラクタの引数順となる。

`displayId` は意味的に `id` と同じ識別子カテゴリなので、`id` の直後に配置した。

注意点: 順序を間違えてもコンパイルエラーにならず、型が一致する場合は実行時にフィールドの値が入れ替わる Silent Bug になりうる。TDD でテストを先に書くことで、この種のバグを防いでいる。
