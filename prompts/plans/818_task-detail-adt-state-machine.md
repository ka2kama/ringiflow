# 実装計画: #818 Task/Detail.elm の Model を ADT ベースステートマシンにリファクタリング

## Context

#797（調査 Issue）で Task/Detail.elm が ADT ベースステートマシン適用の高優先度候補として特定された。現在のフラットな Model では `task = Loading`/`Failure` 時にも承認/却下操作フィールド（`comment`, `isSubmitting`, `pendingAction`）が存在し、不正な操作が型レベルで許可されてしまう。ADR-054 で標準化されたパターン A を適用し、Loading/Failed 状態で操作フィールドが型レベルで存在しない構造にする。

#796（Designer.elm）で同パターンのリファクタリングが成功しており、確立されたパターンに従う。

## 対象・対象外

- 対象: `frontend/src/Page/Task/Detail.elm`（667 行）
- 対象外: `frontend/src/Main.elm`（公開インターフェース `init/update/view/subscriptions/updateShared/Model/Msg` は不変）、新機能の追加、E2E テストの変更

## 設計判断

### 判断 1: 全操作フィールドを LoadedState に配置

`comment`, `isSubmitting`, `pendingAction`, `errorMessage`, `successMessage` の 5 フィールドすべてを LoadedState に配置する。

根拠:
- `comment`/`isSubmitting`/`pendingAction`: 承認操作に直結。Loaded 時のみ意味がある
- `errorMessage`/`successMessage`: 承認操作の結果メッセージ。Loaded 状態でのみ設定・表示される
- `Refresh` は Loaded → Loading への遷移であり、メッセージの破棄は自然（現在のコードでも `Refresh` 時に明示的にクリアしている）
- 承認成功後の再取得（`GotTaskDetail`）は Loading に遷移せず Loaded のまま `taskDetail` を更新するため、メッセージは保持される

### 判断 2: `GotTaskDetail` は外側 update で処理、状態に応じて分岐

- Loading/Failed → Loaded: 新しい `LoadedState` を初期値で構築
- Loaded → Loaded: 既存の `LoadedState` の `taskDetail` のみ更新（メッセージ等を保持）

根拠: 承認成功後の再取得で `successMessage` を保持するため。Designer.elm の `handleGotDefinition` と同様の遷移ハンドラパターン。

### 判断 3: `Refresh` は外側 update で処理

`Refresh` は Loaded → Loading への状態遷移を伴う。外側 `update` で `model.state = Loading` に設定し、LoadedState を破棄する。

根拠: 状態遷移は外側で管理するという Designer.elm と同じ責務分離パターン。

### 判断 4: `approveStep`/`rejectStep`/`requestChangesStep` のシグネチャ簡素化

現在は `Model -> WorkflowStep -> Cmd Msg` で `model.task` をパターンマッチしている。リファクタリング後は `Shared -> LoadedState -> WorkflowStep -> Cmd Msg` となり、Loaded 状態が確定しているためパターンマッチが不要になる。

### 判断 5: NotAsked バリアントの除去

現在の `RemoteData` の `NotAsked` は `init` で使用されておらず、`viewContent` での処理も dead code。`PageState` では Loading/Failed/Loaded の 3 バリアントのみとし、`RemoteData` の import を削除する。

## ターゲット構造

```elm
-- 外側 Model: 共通フィールド + 状態 ADT
type alias Model =
    { shared : Shared
    , workflowDisplayNumber : Int
    , stepDisplayNumber : Int
    , state : PageState
    }

type PageState
    = Loading
    | Failed ApiError
    | Loaded LoadedState

-- Loaded 時のみ存在するフィールド（6 フィールド）
type alias LoadedState =
    { taskDetail : TaskDetail
    , comment : String
    , isSubmitting : Bool
    , pendingAction : Maybe PendingAction
    , errorMessage : Maybe String
    , successMessage : Maybe String
    }
```

## Phase 1（単一 Phase: 構造リファクタリング）

#### 確認事項

- 型: `TaskDetail` の構造 → `frontend/src/Data/Task.elm`
- 型: `WorkflowStep` の構造（`step.status`, `step.version`, `step.displayNumber`） → `frontend/src/Data/WorkflowInstance.elm`
- パターン: `Main.elm` が `TaskDetail.Model` を opaque に使用 → Grep 確認済み。内部構造の変更は Main.elm に影響しない
- パターン: Designer.elm の update 分割パターン → `prompts/plans/796_designer-adt-state-machine.md` で確認済み
- ライブラリ: `RemoteData` の import を削除、`PageState` で置換

#### 実装手順

**ステップ 1: 型定義の変更**

```elm
type alias Model =
    { shared : Shared
    , workflowDisplayNumber : Int
    , stepDisplayNumber : Int
    , state : PageState
    }

type PageState
    = Loading
    | Failed ApiError
    | Loaded LoadedState

type alias LoadedState =
    { taskDetail : TaskDetail
    , comment : String
    , isSubmitting : Bool
    , pendingAction : Maybe PendingAction
    , errorMessage : Maybe String
    , successMessage : Maybe String
    }
```

- `import RemoteData exposing (RemoteData(..))` を削除

**ステップ 2: `init` 関数**

```elm
init shared workflowDisplayNumber stepDisplayNumber =
    ( { shared = shared
      , workflowDisplayNumber = workflowDisplayNumber
      , stepDisplayNumber = stepDisplayNumber
      , state = Loading
      }
    , TaskApi.getTaskByDisplayNumbers { ... }
    )
```

**ステップ 3: `update` 関数の分割**

外側 `update` で状態遷移メッセージを処理し、それ以外を `updateLoaded` に委譲:

```elm
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotTaskDetail result ->
            handleGotTaskDetail result model

        Refresh ->
            ( { model | state = Loading }
            , TaskApi.getTaskByDisplayNumbers
                { config = Shared.toRequestConfig model.shared
                , workflowDisplayNumber = model.workflowDisplayNumber
                , stepDisplayNumber = model.stepDisplayNumber
                , toMsg = GotTaskDetail
                }
            )

        _ ->
            case model.state of
                Loaded loaded ->
                    let
                        ( newLoaded, cmd ) =
                            updateLoaded msg model.shared model.workflowDisplayNumber model.stepDisplayNumber loaded
                    in
                    ( { model | state = Loaded newLoaded }, cmd )

                _ ->
                    ( model, Cmd.none )
```

`handleGotTaskDetail` — 状態に応じて遷移または更新:

```elm
handleGotTaskDetail : Result ApiError TaskDetail -> Model -> ( Model, Cmd Msg )
handleGotTaskDetail result model =
    case result of
        Ok taskDetail ->
            case model.state of
                Loaded loaded ->
                    ( { model | state = Loaded { loaded | taskDetail = taskDetail } }
                    , Cmd.none
                    )

                _ ->
                    ( { model | state = Loaded (initLoaded taskDetail) }
                    , Cmd.none
                    )

        Err err ->
            ( { model | state = Failed err }
            , Cmd.none
            )
```

`initLoaded` ヘルパー:

```elm
initLoaded : TaskDetail -> LoadedState
initLoaded taskDetail =
    { taskDetail = taskDetail
    , comment = ""
    , isSubmitting = False
    , pendingAction = Nothing
    , errorMessage = Nothing
    , successMessage = Nothing
    }
```

`updateLoaded` — Loaded 状態での全メッセージ:

```elm
updateLoaded : Msg -> Shared -> Int -> Int -> LoadedState -> ( LoadedState, Cmd Msg )
updateLoaded msg shared workflowDisplayNumber stepDisplayNumber loaded =
    case msg of
        UpdateComment comment ->
            ( { loaded | comment = comment }, Cmd.none )

        ClickApprove step ->
            ( { loaded | pendingAction = Just (ConfirmApprove step) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        -- ... ClickReject, ClickRequestChanges 同様

        ConfirmAction ->
            case loaded.pendingAction of
                Just (ConfirmApprove step) ->
                    ( { loaded | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , approveStep shared loaded step
                    )
                -- ... ConfirmReject, ConfirmRequestChanges 同様
                Nothing ->
                    ( loaded, Cmd.none )

        CancelAction ->
            ( { loaded | pendingAction = Nothing }, Cmd.none )

        GotApproveResult result ->
            handleApprovalResult "承認しました" result shared workflowDisplayNumber stepDisplayNumber loaded

        -- ... GotRejectResult, GotRequestChangesResult 同様

        DismissMessage ->
            ( { loaded | errorMessage = Nothing, successMessage = Nothing }, Cmd.none )

        _ ->
            ( loaded, Cmd.none )
```

**ステップ 4: API 呼び出し関数のシグネチャ変更**

```elm
approveStep : Shared -> LoadedState -> WorkflowStep -> Cmd Msg
approveStep shared loaded step =
    WorkflowApi.approveStep
        { config = Shared.toRequestConfig shared
        , workflowDisplayNumber = loaded.taskDetail.workflow.displayNumber
        , stepDisplayNumber = step.displayNumber
        , body =
            { version = step.version
            , comment = nonEmptyComment loaded.comment
            }
        , toMsg = GotApproveResult
        }
```

- `rejectStep`, `requestChangesStep` も同様
- `model.task` のパターンマッチが不要になる（Loaded 確定のため）

`handleApprovalResult` のシグネチャ変更:

```elm
handleApprovalResult : String -> Result ApiError WorkflowInstance -> Shared -> Int -> Int -> LoadedState -> ( LoadedState, Cmd Msg )
```

**ステップ 5: `view` 関数の変更**

```elm
view : Model -> Html Msg
view model =
    div []
        [ viewHeader
        , viewBody model
        ]

viewBody : Model -> Html Msg
viewBody model =
    case model.state of
        Loading ->
            LoadingSpinner.view

        Failed _ ->
            viewError

        Loaded loaded ->
            viewLoaded (Shared.zone model.shared) loaded

viewLoaded : Time.Zone -> LoadedState -> Html Msg
viewLoaded zone loaded =
    div []
        [ MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = loaded.successMessage
            , errorMessage = loaded.errorMessage
            }
        , viewTaskDetail zone loaded
        , viewConfirmDialog loaded.pendingAction
        ]
```

view サブ関数のシグネチャ変更:
- `viewTaskDetail : Time.Zone -> LoadedState -> Html Msg`（`TaskDetail -> Model ->` から変更）
- `viewApprovalSection : WorkflowStep -> LoadedState -> Html Msg`（`Model` → `LoadedState`）
- `viewApprovalButtons : WorkflowStep -> Bool -> Html Msg`（変更なし）
- `viewCommentInput : String -> Html Msg`（変更なし）
- その他の view 関数（`viewWorkflowTitle`, `viewWorkflowStatus`, `viewSteps`, `viewBasicInfo`, `viewFormData`, `viewRawFormData`, `viewStepStatusBadge`, `viewStep`）はシグネチャ変更なし

#### 操作パス

操作パス: 該当なし（純粋な構造リファクタリングであり、ユーザー操作パスに変更なし）

#### テストリスト

ユニットテスト（該当なし — 既存のユニットテストなし）

ハンドラテスト（該当なし）
API テスト（該当なし）

E2E テスト:
- [ ] 既存 E2E テスト（approval.spec.ts, rejection.spec.ts, request-changes.spec.ts）が全パス

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `GotTaskDetail` が初回ロードと承認後の再取得の 2 つの文脈で呼ばれる | 不完全なパス | `handleGotTaskDetail` で `model.state` をパターンマッチし、Loaded なら taskDetail のみ更新（メッセージ保持）、それ以外なら新規 LoadedState を構築 |
| 2回目 | `view` で `MessageAlert` と `viewConfirmDialog` が Loaded 時のみ意味がある | 状態網羅漏れ | `viewBody` で状態パターンマッチし、Loaded 時のみ `viewLoaded` でメッセージとダイアログをレンダリング |
| 3回目 | `NotAsked` バリアントが dead code | 既存手段の見落とし | `PageState` で 3 バリアントのみ定義し、`RemoteData` import を削除 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Detail.elm 全関数（init, update, view, subscriptions, updateShared + ヘルパー関数 8 個）を網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各ステップで具体的コードスニペット提示、判断 5 項目すべて根拠付き |
| 3 | 設計判断の完結性 | 全差異に判断記載 | OK | 5 つの設計判断を記載 |
| 4 | スコープ境界 | 対象・対象外が明記 | OK | Detail.elm のみ / Main.elm・E2E テスト変更は対象外 |
| 5 | 技術的前提 | 前提が考慮されている | OK | RemoteData 削除、Main.elm への影響なし、E2E テストの変更不要を確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-054 パターン A 準拠、#796 と同じ構造パターン |

## 検証

```bash
just check-all  # lint + test + API test + E2E test
```

すべてのテストがパスすることで、リファクタリングが既存の振る舞いを破壊していないことを確認する。
