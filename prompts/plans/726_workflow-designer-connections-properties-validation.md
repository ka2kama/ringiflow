# #726 ワークフローデザイナー 接続線・プロパティ・バリデーション

## コンテキスト

#725（キャンバス + ステップ配置）の上に、フロー定義を完成させるために必要な機能を追加する。バックエンド API は全エンドポイント実装済みのため、フロントエンドのみの変更。

スコープ: WFD-003（接続線）、WFD-004（プロパティパネル）、WFD-005（バリデーション）、定義の保存

対象外: フォームエディタ（WFD-006/007）、ワークフローインスタンスとの連携

## 設計判断

| 判断 | 決定内容 | 理由 |
|------|---------|------|
| モジュール構成 | Designer.elm 単一ファイルを維持 | データ型は Data/DesignerCanvas.elm に分離済み。TEA ページモジュールとして自然な範囲 |
| デザイナーモード | Editing のみ（Creating 廃止） | List ページの既存作成ダイアログでDraft定義を作成 → Edit ルートで開く方式。名前/説明入力の重複を避ける |
| 接続ポート表示 | 常時表示 | 発見可能性（Discoverability）を優先 |
| 接続 trigger 自動設定 | Approval ステップで approve/reject なしの場合に自動選択 | UX 改善。手動操作を最小化 |
| プロパティ適用タイミング | onInput でリアルタイム更新 | 送信ボタン不要、キャンバスに即座反映 |
| ValidationResult の配置 | Data/WorkflowDefinition.elm | API レスポンス型としての位置づけ |
| position なし後方互換 | 縦一列等間隔の自動配置 | 既存 seed データとの互換性確保 |

## Phase 構成

```
Phase 1: データ型・エンコーダー・デコーダー (基盤)
  ↓
Phase 2: 接続線 (SVG + D&D)
  ↓
Phase 3: プロパティパネル (ステップ編集)
  ↓
Phase 4: API 統合 (Route + Load + Save)
  ↓
Phase 5: バリデーション + 公開
```

---

### Phase 1: データ型・エンコーダー・デコーダー

変更ファイル:
- `frontend/src/Data/DesignerCanvas.elm` — 型拡張、エンコーダー/デコーダー追加
- `frontend/src/Data/WorkflowDefinition.elm` — encodeUpdateRequest、ValidationResult 追加
- `frontend/src/Api/WorkflowDefinition.elm` — updateDefinition、validateDefinition 追加
- `frontend/tests/Data/DesignerCanvasTest.elm`

#### 型変更

`Data/DesignerCanvas.elm`:

```elm
-- StepNode 拡張
type alias StepNode =
    { id : String
    , stepType : StepType
    , name : String
    , position : Position
    , assignee : Maybe Assignee
    , endStatus : Maybe String  -- "approved" | "rejected"
    }

type alias Assignee =
    { type_ : String }  -- Phase 2-4 は "user" のみ

-- Transition 追加
type alias Transition =
    { from : String
    , to : String
    , trigger : Maybe String  -- "approve" | "reject"
    }

-- DraggingState 拡張
type DraggingState
    = DraggingExistingStep String Position
    | DraggingNewStep StepType Position
    | DraggingConnection String Position  -- 接続元 stepId, マウス位置
```

エンコーダー/デコーダー:

```elm
encodeDefinition : Dict String StepNode -> List Transition -> Encode.Value
loadStepsFromDefinition : Decode.Value -> Result Decode.Error (Dict String StepNode)
loadTransitionsFromDefinition : Decode.Value -> Result Decode.Error (List Transition)
```

`Data/WorkflowDefinition.elm`:

```elm
encodeUpdateRequest : { name : String, description : String, definition : Encode.Value, version : Int } -> Encode.Value

type alias ValidationError = { code : String, message : String, stepId : Maybe String }
type alias ValidationResult = { valid : Bool, errors : List ValidationError }
validationResultDecoder : Decode.Decoder ValidationResult
```

`Api/WorkflowDefinition.elm`:

```elm
updateDefinition : { config : RequestConfig, id : String, body : Encode.Value, toMsg : Result ApiError WorkflowDefinition -> msg } -> Cmd msg
validateDefinition : { config : RequestConfig, body : Encode.Value, toMsg : Result ApiError ValidationResult -> msg } -> Cmd msg
```

#### 確認事項
- [x] 型: StepNode の現在のフィールド → `Data/DesignerCanvas.elm` L49-54: `{ id, stepType, name, position }`
- [x] 型: DraggingState の現在のバリアント → `Data/DesignerCanvas.elm` L69-71: `DraggingExistingStep String Position | DraggingNewStep StepType Position`
- [x] パターン: Decoder.Pipeline 使用パターン → `Data/WorkflowDefinition.elm` L245-254: `Decode.succeed T |> required "field" Decode.type`
- [x] パターン: エンコーダーパターン → `Data/WorkflowDefinition.elm` L159-165: `Encode.object [ ( "key", Encode.string val ) ]`
- [x] パターン: `Api.put` シグネチャ → `Api.elm` L166-183: `{ config, url, body, decoder, toMsg }`
- [x] パターン: `Api.post` シグネチャ → `Api.elm` L141-148: `{ config, url, body, decoder, toMsg }` (put と同形)

#### テストリスト

ユニットテスト:
- [x] `encodeDefinition` が steps と transitions から正しい JSON を生成する
- [x] `loadStepsFromDefinition` が position あり JSON から StepNode Dict を生成する
- [x] `loadStepsFromDefinition` が position なし JSON から自動配置で StepNode Dict を生成する
- [x] `loadTransitionsFromDefinition` が transitions を正しくデコードする
- [x] `encodeUpdateRequest` が name/description/definition/version を正しくエンコードする
- [x] `validationResultDecoder` が valid: true, errors: [] をデコードする
- [x] `validationResultDecoder` が valid: false, errors: [{code, message, step_id}] をデコードする

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 2: 接続線

変更ファイル:
- `frontend/src/Data/DesignerCanvas.elm` — 接続ポート座標計算関数
- `frontend/src/Page/WorkflowDefinition/Designer.elm` — transitions フィールド、接続 D&D、SVG レイヤー
- `frontend/tests/Data/DesignerCanvasTest.elm`
- `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm`

#### Model 変更

```elm
type alias Model =
    { ...
    , transitions : List Transition
    , selectedTransitionIndex : Maybe Int
    }
```

#### Msg 追加

```elm
| ConnectionPortMouseDown String Float Float  -- stepId, clientX, clientY
| TransitionClicked Int
```

#### SVG 描画

マーカー定義（`Svg.defs` + `Svg.marker`）:
- `arrow-none`: secondary-400 (#94a3b8) — trigger なし
- `arrow-approve`: success-600 (#059669) — approve
- `arrow-reject`: error-600 (#dc2626) — reject

接続線（`Svg.path` ベジェ曲線）:
- from ステップの右端中央 → to ステップの左端中央
- approve: 実線 + 緑矢印、reject: 破線 + 赤矢印、none: 実線 + グレー矢印

接続ポート（`Svg.circle` r=5）:
- ステップ右端中央に出力ポート（常時表示）
- D&D でポートからドラッグ → 別ステップ上でドロップ → Transition 作成

SVG レイヤー順序:
```
defs → background → grid → transitions → connectionDragPreview → steps → dragPreview
```

#### 接続ドロップの判定

DraggingConnection 中の CanvasMouseUp 時:
1. `clientToCanvas` でキャンバス座標に変換
2. 全ステップの矩形と重なり判定（`stepContainsPoint`）
3. 接続元と異なるステップが見つかれば Transition 追加
4. 自己ループ防止: from == to は無視

#### 接続 trigger の自動設定

from が Approval の場合:
- 既存に approve なし → `Just "approve"`
- 既存に reject なし → `Just "reject"`
- 両方あり → `Nothing`

from が Start/End の場合: `Nothing`

#### Delete キーの拡張

ステップ削除時: 関連する transitions も同時に削除。
接続線削除: selectedTransitionIndex 設定済みで Delete → 該当 transition を削除。

#### 確認事項
- [x] ライブラリ: `Svg.defs`, `Svg.marker` → elm/svg 1.0.1 で確認済み（前セッション）
- [x] ライブラリ: `SvgAttr.markerEnd`, `SvgAttr.markerWidth`, `SvgAttr.refX`, `SvgAttr.refY`, `SvgAttr.orient` → 確認済み（前セッション）
- [x] パターン: `Html.Events.stopPropagationOn` → `Designer.elm` L499-503 で既存使用
- [x] パターン: `Svg.Events.onClick` → `Designer.elm` L401 で既存使用

#### テストリスト

ユニットテスト:
- [x] `stepOutputPortPosition` がステップ右端中央を返す
- [x] `stepInputPortPosition` がステップ左端中央を返す
- [x] `stepContainsPoint` が矩形内の座標で True を返す（境界値含む）
- [x] `stepContainsPoint` が矩形外の座標で False を返す
- [x] `autoTrigger`: Approval で approve なし → Just "approve"
- [x] `autoTrigger`: Approval で approve あり reject なし → Just "reject"
- [x] `autoTrigger`: Start/End → Nothing
- [x] update: `ConnectionPortMouseDown` で DraggingConnection に遷移する
- [x] update: `TransitionClicked` で selectedTransitionIndex が設定される
- [x] update: `KeyDown "Delete"` でステップ削除時に関連 transitions も削除される
- [x] update: `KeyDown "Delete"` で selectedTransitionIndex 時に該当 transition が削除される
- [x] update: `CanvasBackgroundClicked` で selectedTransitionIndex が Nothing になる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 3: プロパティパネル

変更ファイル:
- `frontend/src/Page/WorkflowDefinition/Designer.elm` — プロパティパネル追加
- `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm`

#### Model 変更

```elm
type alias Model =
    { ...
    , propertyName : String
    , propertyEndStatus : String
    , propertyValidationErrors : Dict String String
    }
```

#### Msg 追加

```elm
| UpdatePropertyName String
| UpdatePropertyEndStatus String
```

#### プロパティパネル

右サイドバー（w-64）。`selectedStepId` に連動して表示:
- 未選択時: 「ステップを選択してください」
- Start: ステップ名（`FormField.viewTextField`）
- Approval: ステップ名 + 承認者指定方式（`FormField.viewReadOnlyField` — Phase 2-4 は「申請時にユーザーを選択」固定）
- End: ステップ名 + 終了ステータス（`FormField.viewSelectField` — approved/rejected）

#### プロパティ同期

`StepClicked` / `StepMouseDown` 時: 選択ステップの name, endStatus を propertyName, propertyEndStatus に同期。
`UpdatePropertyName` / `UpdatePropertyEndStatus` 時: 即座に steps Dict を更新（リアルタイム反映）。

#### レイアウト変更

```elm
view = div [...]
    [ viewToolbar model
    , div [ class "flex flex-1 overflow-hidden" ]
        [ viewPalette
        , viewCanvasArea model
        , viewPropertyPanel model  -- 追加
        ]
    , viewStatusBar model
    ]
```

#### 確認事項
- [ ] パターン: `FormField.viewTextField` シグネチャ → `Component/FormField.elm` L54-62
- [ ] パターン: `FormField.viewSelectField` シグネチャ → `Component/FormField.elm` L103-111
- [ ] パターン: `FormField.viewReadOnlyField` シグネチャ → `Component/FormField.elm` L133-134

#### テストリスト

ユニットテスト:
- [ ] update: `StepClicked` 後に propertyName がステップの name に同期される
- [ ] update: `StepClicked` 後に propertyEndStatus が endStatus の値に同期される
- [ ] update: `UpdatePropertyName` でステップの name がリアルタイム更新される
- [ ] update: `UpdatePropertyEndStatus "approved"` で endStatus が Just "approved" になる
- [ ] update: `UpdatePropertyEndStatus ""` で endStatus が Nothing になる
- [ ] update: `CanvasBackgroundClicked` で propertyName がクリアされる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 4: API 統合（Route + Load + Save）

変更ファイル:
- `frontend/src/Route.elm` — Edit ルート追加
- `frontend/src/Main.elm` — Edit ルートの initPage
- `frontend/src/Page/WorkflowDefinition/Designer.elm` — Load/Save ロジック
- `frontend/src/Page/WorkflowDefinition/List.elm` — デザイナー編集リンク追加
- `frontend/tests/RouteTest.elm`
- `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm`

#### Route 変更

```elm
type Route = ... | WorkflowDefinitionDesignerEdit String  -- /workflow-definitions/{id}/edit
```

パーサー:
```elm
Parser.map WorkflowDefinitionDesignerEdit (s "workflow-definitions" </> string </> s "edit")
```

`isRouteActive`, `toString`, `pageTitle` に追加。

#### Designer.init 変更

```elm
init : Shared -> Nav.Key -> String -> ( Model, Cmd Msg )
init shared key definitionId =
    ( { ...
      , mode = Editing { id = definitionId, version = 0 }
      , loadState = Loading
      }
    , Cmd.batch
        [ Ports.requestCanvasBounds canvasElementId
        , WorkflowDefinitionApi.getDefinition { config, id = definitionId, toMsg = GotDefinition }
        ]
    )
```

#### Model 変更

```elm
type alias Model =
    { ...
    , key : Nav.Key
    , mode : DesignerMode  -- Editing { id, version }
    , loadState : RemoteData ApiError WorkflowDefinition
    , name : String
    , description : String
    , isSaving : Bool
    , successMessage : Maybe String
    , errorMessage : Maybe String
    , isDirty_ : Bool
    }
```

#### Msg 追加

```elm
| GotDefinition (Result ApiError WorkflowDefinition)
| SaveClicked
| GotSaveResult (Result ApiError WorkflowDefinition)
| UpdateDefinitionName String
| DismissMessage
```

#### Save ロジック

```elm
SaveClicked ->
    let definition = DesignerCanvas.encodeDefinition model.steps model.transitions
    in
    case model.mode of
        Editing { id, version } ->
            ( { model | isSaving = True }
            , WorkflowDefinitionApi.updateDefinition
                { config, id, body = encodeUpdateRequest { name, description, definition, version }, toMsg = GotSaveResult }
            )
```

#### List ページ変更

各定義行に「デザイナーで編集」リンクを追加（Draft 定義のみ）:
```elm
a [ href (Route.toString (Route.WorkflowDefinitionDesignerEdit def.id)) ]
    [ text "デザイナーで編集" ]
```

#### 確認事項
- [ ] パターン: Route の string パーサー → `Route.elm` L171 `Parser.map RoleEdit (s "roles" </> string </> s "edit")`
- [ ] パターン: Main.elm の initPage → `Main.elm` での `RoleEdit` のパターン
- [ ] パターン: RemoteData Loading/Success/Failure → `Page/Role/Edit.elm` L60,290-299
- [ ] パターン: Nav.Key の init 受け渡し → `Page/Role/Edit.elm` L55-56
- [ ] パターン: DirtyState の使い方 → `Form/DirtyState.elm`

#### テストリスト

ユニットテスト:
- [ ] Route: `/workflow-definitions/{id}/edit` が `WorkflowDefinitionDesignerEdit id` にマッチ
- [ ] Route: `toString (WorkflowDefinitionDesignerEdit "abc")` が正しいパスを返す
- [ ] update: `GotDefinition (Ok def)` でステップと transitions がロードされる
- [ ] update: `GotDefinition (Err err)` で loadState が Failure になる
- [ ] update: `SaveClicked` で isSaving が True になる
- [ ] update: `GotSaveResult (Ok def)` で successMessage + version 更新

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 5: バリデーション + 公開

変更ファイル:
- `frontend/src/Page/WorkflowDefinition/Designer.elm` — バリデーション表示、ツールバー、公開
- `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm`

#### Model 変更

```elm
type alias Model =
    { ...
    , validationResult : Maybe ValidationResult
    , isValidating : Bool
    , isPublishing : Bool
    }
```

#### Msg 追加

```elm
| ValidateClicked
| GotValidationResult (Result ApiError ValidationResult)
| PublishClicked
| ConfirmPublish
| CancelPublish
| GotPublishResult (Result ApiError WorkflowDefinition)
```

#### ツールバー更新

左: 定義名表示/入力、右: バリデーション・保存・公開ボタン（`Component.Button`）。

#### バリデーション表示

キャンバス下部のパネル:
- valid: 緑テキスト「フロー定義は有効です」
- invalid: エラー一覧（クリックで該当ステップ選択）

ステップハイライト: validationErrors に step_id が含まれるステップに赤枠。

#### 公開フロー

PublishClicked → ConfirmDialog 表示 → ConfirmPublish → 保存（dirty なら）→ バリデーション → 公開 API 呼び出し。

#### 確認事項
- [ ] パターン: `Button.view` のシグネチャ → `Component/Button.elm`
- [ ] パターン: `ConfirmDialog.view` → `Component/ConfirmDialog.elm` と `Ports.showModalDialog`
- [ ] 型: Phase 1 で実装した `ValidationResult` と `validationResultDecoder`

#### テストリスト

ユニットテスト:
- [ ] update: `ValidateClicked` で isValidating が True になる
- [ ] update: `GotValidationResult (Ok { valid: true })` で validationResult が設定される
- [ ] update: `GotValidationResult (Ok { valid: false })` でエラー情報が設定される
- [ ] update: `GotValidationResult (Err err)` で errorMessage が設定される
- [ ] update: `PublishClicked` で確認ダイアログ表示
- [ ] update: `GotPublishResult (Ok def)` で successMessage + ステータス更新

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（Phase 完了後に手動確認: デザイナーで定義を作成し保存できる）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | StepNode に assignee/endStatus がない | アーキテクチャ不整合 | Phase 1 をデータ型拡張専用 Phase として分離 |
| 2回目 | Creating モードと List ダイアログで名前入力が重複 | 既存手段の見落とし | Creating 廃止 → List で作成後 Edit ルートで開く方式に変更 |
| 3回目 | ステップ削除時に関連 transitions の削除が未考慮 | 不完全なパス | Phase 2 の KeyDown Delete で transitions も同時削除 |
| 4回目 | SVG defs/marker の Elm 表現が未確認 | 技術的前提 | elm/svg 1.0.1 で `Svg.defs`, `Svg.marker`, `SvgAttr.markerEnd` 等が利用可能であることを確認 |
| 5回目 | List ページにデザイナーへの導線がない | 不完全なパス | Phase 4 で List に「デザイナーで編集」リンクを追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の全機能が Phase に割り当て | OK | WFD-003→Phase2, WFD-004→Phase3, WFD-005→Phase5, Save/Load→Phase4, Publish→Phase5 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更ファイル・型・関数シグネチャを明示。encodeCreateRequest はフロー変更により影響なし |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | モジュール構成、Creating 廃止、接続ポート表示、trigger 自動設定、プロパティ即時更新の各判断を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | フォームエディタ(WFD-006/007)を対象外に明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮 | OK | Svg.defs/marker の利用可能性、List ページの Nav.Key 不要（リンク方式）を確認 |
| 6 | 既存ドキュメント整合 | 設計書・仕様書と矛盾なし | OK | 詳細設計書15_の型定義案、機能仕様書04_の WFD-003/004/005 仕様と整合 |
