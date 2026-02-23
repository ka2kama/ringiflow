# 実装計画: #796 Designer.elm の Model を ADT ベース状態マシンにリファクタリング

## Context

#793 で発見された「Loading 中に `requestCanvasBounds` が発行され DOM 未存在で失敗する」バグの根本対策。
ADR-054 で標準化した ADT ベース状態マシンパターンの初適用として、Designer.elm のフラットな Model（21 フィールド）を状態ごとに分離する。

目的: Loading 状態でキャンバス関連フィールドが**型レベルで存在しない**構造にし、不正な状態を表現不可能にする。

## 対象・対象外

- 対象: `Designer.elm`（1613 行）、`DesignerTest.elm`（973 行）
- 対象外: `Main.elm`（Designer の公開インターフェース `init/update/view/subscriptions/isDirty/updateShared` は不変）、新機能の追加

## 設計判断

### 判断 1: `WorkflowDefinition` を CanvasState に格納しない

`GotDefinition` ハンドラで `def` から `steps`, `transitions`, `name`, `description`, `version` を抽出後、`def` 自体は参照されない。冗長なフィールドを持たせない。

### 判断 2: `updateLoaded` が `Shared` と `definitionId` をパラメータとして受け取る

API 呼び出しに必要な `shared` / `definitionId` は外側 Model のフィールド。CanvasState が API 接続情報を持たない明確な責務分離。

### 判断 3: テスト用 `expectLoaded` ヘルパーの導入

テストでの直接フィールドアクセス（`newModel.selectedStepId` 等）が 40 箇所以上。`expectLoaded` ヘルパーで可読性を維持。

### 判断 4: Port サブスクリプションの配置

`Ports.receiveCanvasBounds GotCanvasBounds` は全状態で購読を維持。`GotDefinition` 成功時に `requestCanvasBounds` を発行し、レスポンスは Loaded 遷移後に到着するが、購読は遷移前から必要。

### 判断 5: DirtyState の extensible record 互換性

`DirtyState.markDirty/clearDirty` は `{ a | isDirty_ : Bool }` 型制約。CanvasState に `isDirty_` を持たせることで互換性を維持。

## ターゲット構造

```elm
-- 外側 Model: 共通フィールド + 状態 ADT
type alias Model =
    { shared : Shared
    , definitionId : String
    , state : PageState
    }

type PageState
    = Loading
    | Failed ApiError
    | Loaded CanvasState

-- Loaded 時のみ存在するフィールド（20 フィールド）
type alias CanvasState =
    { steps : Dict String StepNode
    , transitions : List Transition
    , selectedStepId : Maybe String
    , selectedTransitionIndex : Maybe Int
    , dragging : Maybe DraggingState
    , canvasBounds : Maybe Bounds
    , nextStepNumber : Int
    , propertyName : String
    , propertyEndStatus : String
    , name : String
    , description : String
    , version : Int
    , isSaving : Bool
    , successMessage : Maybe String
    , errorMessage : Maybe String
    , isDirty_ : Bool
    , validationResult : Maybe ValidationResult
    , isValidating : Bool
    , isPublishing : Bool
    , pendingPublish : Bool
    }
```

## Phase 1（単一 Phase: 構造リファクタリング）

#### 確認事項

- 型: `DirtyState.markDirty/clearDirty` のシグネチャが `{ a | isDirty_ : Bool }` → `DirtyState.elm` 行 37, 44, 57 で確認済み
- パターン: `Main.elm` が `Designer.Model` を opaque に使用 → 内部構造の変更は Main.elm に影響しない
- パターン: view サブ関数すべてが `shared` / `definitionId` / `loadState` を参照しない → `CanvasState ->` に変更可能
- ライブラリ: `RemoteData` の import を Designer.elm / DesignerTest.elm 両方から削除

#### 実装手順

**ステップ 1: 型定義の変更（Designer.elm）**
1. `PageState` カスタム型、`CanvasState` type alias を定義
2. `Model` を 3 フィールド（`shared`, `definitionId`, `state`）に変更
3. `exposing` に `CanvasState`, `PageState(..)` を追加
4. `import RemoteData` を削除

**ステップ 2: `init` 関数**
```elm
init shared definitionId =
    ( { shared = shared, definitionId = definitionId, state = Loading }
    , WorkflowDefinitionApi.getDefinition { ... }
    )
```

**ステップ 3: `isDirty` 関数**
```elm
isDirty model =
    case model.state of
        Loaded canvas -> DirtyState.isDirty canvas
        _ -> False
```

**ステップ 4: `update` 関数の分割**

```elm
update msg model =
    case msg of
        GotDefinition result ->
            handleGotDefinition result model
        _ ->
            case model.state of
                Loaded canvas ->
                    let ( newCanvas, cmd ) = updateLoaded msg model.shared model.definitionId canvas
                    in ( { model | state = Loaded newCanvas }, cmd )
                _ ->
                    ( model, Cmd.none )
```

- `handleGotDefinition`: CanvasState を初期値で構築し `Loaded` に遷移 + `requestCanvasBounds` を Cmd で返す
- `updateLoaded`: 全 Msg ハンドラ（GotDefinition 除く）。`model` → `canvas`、`model.shared` → `shared`、`model.definitionId` → `definitionId`

**ステップ 5: ヘルパー関数のシグネチャ変更**
- `syncPropertyFields : String -> CanvasState -> CanvasState`
- `deleteSelectedStep : CanvasState -> ( CanvasState, Cmd Msg )`

**ステップ 6: `subscriptions` 関数**
- `Ports.receiveCanvasBounds` は全状態で購読（判断 4）
- `onMouseMove/onMouseUp/onKeyDown` は `Loaded` 時のみ

**ステップ 7: `view` 関数**
- `view` で `model.state` をパターンマッチ（`Loading`/`Failed`/`Loaded`）
- `viewLoaded : CanvasState -> Html Msg` を新設
- 全 14 view サブ関数: `Model ->` → `CanvasState ->`、内部 `model` → `canvas`

**ステップ 8: テストファイル（DesignerTest.elm）**

8a. import 変更:
```elm
import Page.WorkflowDefinition.Designer as Designer exposing (CanvasState, Model, Msg(..), PageState(..))
-- RemoteData の import を削除
```

8b. ヘルパー関数の再構築:
```elm
expectLoaded : (CanvasState -> Expect.Expectation) -> Model -> Expect.Expectation

defaultCanvas : CanvasState       -- 全フィールドデフォルト値
canvasWithBounds : CanvasState    -- defaultCanvas + canvasBounds = Just ...
canvasWithOneStep : CanvasState   -- canvasWithBounds + 承認ステップ
canvasWithEndStep : CanvasState   -- canvasWithBounds + 終了ステップ
loadedCanvas : CanvasState        -- GotDefinition 経由で構築された canvas

baseModel : Model                 -- { shared, definitionId, state = Loaded defaultCanvas }
modelWithBounds : Model           -- baseModel + Loaded canvasWithBounds
modelWithOneStep : Model          -- baseModel + Loaded canvasWithOneStep
modelWithEndStep : Model          -- baseModel + Loaded canvasWithEndStep
```

8c. テスト内 record update:
```elm
-- Before: { modelWithOneStep | selectedStepId = Just "approval_1" }
-- After:  { baseModel | state = Loaded { canvasWithOneStep | selectedStepId = Just "approval_1" } }
```

8d. アサーション:
```elm
-- Before: newModel.selectedStepId |> Expect.equal (Just "approval_1")
-- After:  newModel |> expectLoaded (\c -> c.selectedStepId |> Expect.equal (Just "approval_1"))
```

8e. API テスト:
```elm
-- Before: case loadedModel.loadState of Success _ -> ...
-- After:  case loadedModel.state of Loaded _ -> ...
```

#### 操作パス

操作パス: 該当なし（ドメインロジックのみ。純粋な構造リファクタリングであり、ユーザー操作パスに変更なし）

#### テストリスト

ユニットテスト:
- [ ] 既存 973 行の全テストが構造変更後もパスすること（テスト件数の増減なし）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `subscriptions` で `onKeyDown` が全状態で購読されているが Loading/Failed では不要 | 状態網羅漏れ | onKeyDown を Loaded ブランチ内に移動 |
| 2回目 | テストの `modelWithBounds` 等が `initModel`（Loading 状態）からの record update で構築されているが、リファクタリング後は Loading に canvas フィールドが存在しない | 未定義 | Canvas レベルヘルパー（`defaultCanvas`/`canvasWithBounds`）を導入 |
| 3回目 | `loadedModel` から canvas フィールドを更新するテストが複数存在 | 不完全なパス | `loadedCanvas` ヘルパーを追加 |
| 4回目 | `GotCanvasBounds` が Loading 中に到着した場合 | 競合・エッジケース | `updateLoaded` に到達しないため安全に無視される。追加対応不要 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Designer.elm 全関数 + テスト全パターンを網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各ステップで具体的コードスニペット提示 |
| 3 | 設計判断の完結性 | 全差異に判断記載 | OK | 5 つの設計判断を記載 |
| 4 | スコープ境界 | 対象・対象外が明記 | OK | Designer.elm + DesignerTest.elm / Main.elm は対象外 |
| 5 | 技術的前提 | 前提が考慮されている | OK | DirtyState extensible record 互換性確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-054 パターン A 準拠 |

## 検証

```bash
just check-all  # lint + test + API test + E2E test
```

すべてのテストがパスすることで、リファクタリングが既存の振る舞いを破壊していないことを確認する。
