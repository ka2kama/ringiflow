# #816 Workflow/Detail.elm の Model を ADT ベースステートマシンにリファクタリングする

## Context

ADR-054（ADT ベースステートマシンパターンの標準化）に基づき、Workflow/Detail.elm の 18 フィールドのフラット Model を Loading/Failed/Loaded の ADT に分離する。Loading 中に承認/コメント/再提出の操作フィールドが型レベルで存在しないようにすることが目的。

先行実装 #818（Task/Detail.elm）、#817（Workflow/New.elm）のパターンが確立されており、同一パターンを踏襲する。

## 対象ファイル

- `frontend/src/Page/Workflow/Detail.elm` — リファクタリング本体（唯一の変更対象）

## 参照ファイル

- `frontend/src/Page/Task/Detail.elm` — 確立済みパターン（`initLoaded`, `handleGotTaskDetail`, `updateLoaded`）
- `frontend/src/Page/Workflow/New.elm` — `PageState` 型宣言のスタイル参照

## 設計判断

### コメント fetch 順序の変更

現行: `init` で `getWorkflow` + `listComments` を並列発行
変更後: `init` では `getWorkflow` のみ発行。`handleGotWorkflow Ok` で `getDefinition` + `listComments` を並列発行

理由: Loading 状態に LoadedState が存在しないため、`GotComments` が Loading 中に届いた場合の格納先がない。workflow 取得後に並列発行することで、コードの簡潔さと Task/Detail のパターンとの一貫性を確保する。コメントはページ下部の付随データであり UX 影響は最小。

## Phase 1: ADT リファクタリング（単一フェーズ）

### 確認事項

- 型: `WorkflowInstance`, `WorkflowDefinition`, `WorkflowComment` → `Data/WorkflowInstance.elm`, `Data/WorkflowDefinition.elm`, `Data/WorkflowComment.elm`
- パターン: `handleGotTaskDetail`, `initLoaded`, `updateLoaded` → `Page/Task/Detail.elm`
- API: `WorkflowApi.getWorkflow`, `WorkflowApi.listComments`, `WorkflowDefinitionApi.getDefinition` → `Api/Workflow.elm`, `Api/WorkflowDefinition.elm`

### 1. 型定義の変更

```elm
type alias Model =
    { shared : Shared
    , workflowDisplayNumber : Int
    , state : PageState
    }

type PageState
    = Loading
    | Failed ApiError
    | Loaded LoadedState

type alias LoadedState =
    { workflow : WorkflowInstance                              -- RemoteData ではなく直値
    , definition : RemoteData ApiError WorkflowDefinition      -- Loaded 後も非同期ロード中
    -- 承認/却下/差し戻し
    , comment : String
    , isSubmitting : Bool
    , pendingAction : Maybe PendingAction
    , errorMessage : Maybe String
    , successMessage : Maybe String
    -- コメントスレッド
    , comments : RemoteData ApiError (List WorkflowComment)    -- Loaded 後も非同期ロード中
    , newCommentBody : String
    , isPostingComment : Bool
    -- 再提出
    , isEditing : Bool
    , editFormData : Dict String String
    , editApprovers : Dict String ApproverSelector.State
    , users : RemoteData ApiError (List UserItem)
    , resubmitValidationErrors : Dict String String
    , isResubmitting : Bool
    }
```

### 2. 新規ヘルパー関数

```elm
initLoaded : WorkflowInstance -> LoadedState
-- workflow から LoadedState を構築。definition = Loading, comments = Loading で初期化

handleGotWorkflow : Result ApiError WorkflowInstance -> Model -> ( Model, Cmd Msg )
-- Loading/Failed → initLoaded + Cmd.batch [getDefinition, listComments]
-- Loaded → { loaded | workflow = workflow } (承認後の部分更新用)

updateLoaded : Msg -> Shared -> Int -> LoadedState -> ( LoadedState, Cmd Msg )
-- Loaded 状態専用の update。GotWorkflow/Refresh 以外のすべての Msg を処理
```

### 3. update の構造

```elm
update msg model =
    case msg of
        GotWorkflow result -> handleGotWorkflow result model
        Refresh -> ( { model | state = Loading }, getWorkflow のみ )
        _ ->
            case model.state of
                Loaded loaded ->
                    let (newLoaded, cmd) = updateLoaded msg model.shared model.workflowDisplayNumber loaded
                    in ( { model | state = Loaded newLoaded }, cmd )
                _ -> ( model, Cmd.none )
```

`updateLoaded` に委譲される Msg: `GotDefinition`, `GotComments`, `UpdateComment`, `ClickApprove/Reject/RequestChanges`, `ConfirmAction`, `CancelAction`, `GotApproveResult/RejectResult/RequestChangesResult`, `UpdateNewComment`, `SubmitComment`, `GotPostCommentResult`, `StartEditing`, `CancelEditing`, `UpdateEditFormField`, `EditApprover*`, `SubmitResubmit`, `GotResubmitResult`, `GotUsers`, `DismissMessage`

### 4. init の変更

```elm
init shared workflowDisplayNumber =
    ( { shared = shared, workflowDisplayNumber = workflowDisplayNumber, state = Loading }
    , WorkflowApi.getWorkflow { ..., toMsg = GotWorkflow }
    )
-- listComments は handleGotWorkflow に移動
```

### 5. ヘルパー関数のシグネチャ変更

| 関数 | 変更前 | 変更後 |
|------|--------|--------|
| `handleApprovalResult` | `Model -> ( Model, Cmd Msg )` | `LoadedState -> ( LoadedState, Cmd Msg )` |
| `validateResubmit` | `Model -> Dict String String` | `LoadedState -> Dict String String` |
| `buildResubmitApprovers` | `Model -> List StepApproverRequest` | `LoadedState -> List StepApproverRequest` |

フィールドアクセスを `model.xxx` → `loaded.xxx` に変更。

### 6. view の構造変更

```elm
view model =
    div [] [ viewHeader, viewBody model ]

viewBody model =
    case model.state of
        Loading -> LoadingSpinner.view
        Failed _ -> viewError
        Loaded loaded -> viewLoaded model.shared loaded

viewLoaded shared loaded =
    div [] [ MessageAlert.view ..., viewWorkflowDetail shared loaded, viewConfirmDialog loaded.pendingAction ]
```

| view 関数 | 変更前の引数 | 変更後の引数 |
|-----------|------------|------------|
| `viewWorkflowDetail` | `Model -> WorkflowInstance -> Html Msg` | `Shared -> LoadedState -> Html Msg` |
| `viewResubmitSection` | `Model -> WorkflowInstance -> Html Msg` | `Shared -> LoadedState -> Html Msg` |
| `viewEditableFormData` | `Model -> Html Msg` | `LoadedState -> Html Msg` |
| `viewEditableApprovers` | `Model -> WorkflowDefinition -> Html Msg` | `LoadedState -> WorkflowDefinition -> Html Msg` |
| `viewEditableApproverStep` | `Model -> ApprovalStepInfo -> Html Msg` | `LoadedState -> ApprovalStepInfo -> Html Msg` |
| `viewEditActions` | `Model -> Html Msg` | `LoadedState -> Html Msg` |
| `viewCommentSection` | `Model -> Html Msg` | `LoadedState -> Html Msg` |

変更不要: `viewHeader`, `viewError`, `viewTitle`, `viewStatus`, `viewStepProgress`, `viewBasicInfo`, `viewFormData`, `viewApprovalSection`, `viewConfirmDialog`, `viewCommentList`, `viewCommentItem`, `viewCommentForm`, `viewEditableFormField`, その他の純粋な表示関数

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 詳細ページロード → workflow 取得成功 → Loaded 表示 | 正常系 | E2E |
| 2 | Loaded 状態で承認 → 確認ダイアログ → workflow 更新 | 正常系 | E2E (approval.spec.ts) |
| 3 | Loaded 状態で却下 → 確認ダイアログ → workflow 更新 | 正常系 | E2E (rejection.spec.ts) |
| 4 | Loaded 状態で差し戻し → 再申請 → 承認 | 正常系 | E2E (request-changes.spec.ts) |
| 5 | Loaded 状態でコメント投稿 | 正常系 | E2E (request-changes.spec.ts 内) |

### テストリスト

ユニットテスト: 該当なし（純粋リファクタリング、Elm ユニットテストなし）
ハンドラテスト: 該当なし（バックエンド変更なし）
API テスト: 該当なし（API インターフェース変更なし）
E2E テスト: 既存テストがそのまま検証基準
- `tests/e2e/tests/approval.spec.ts`
- `tests/e2e/tests/rejection.spec.ts`
- `tests/e2e/tests/request-changes.spec.ts`

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `listComments` の並列フェッチが Loading 中に届く問題 | 不完全なパス | `handleGotWorkflow` に移動（逐次→並列発行） |
| 2回目 | `handleApprovalResult` のシグネチャが Model を受け取る | アーキテクチャ不整合 | `LoadedState -> ( LoadedState, Cmd Msg )` に変更 |
| 3回目 | `validateResubmit`, `buildResubmitApprovers` が Model を参照 | 状態依存フィールド | シグネチャを LoadedState 受け取りに変更 |
| 4回目 | `GotResubmitResult Ok` が `listComments` を再発行する動作 | 不完全なパス | `updateLoaded` 内で発行（`workflowDisplayNumber` を引数で渡す） |
| 5回目 | `GotDefinition` が update トップレベルで処理されている | アーキテクチャ不整合 | `updateLoaded` に委譲（Loaded 状態にのみ存在するため） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 18フィールドすべての移動先が確定 | OK | shared/workflowDisplayNumber → Model、残り16 → LoadedState |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | コメント fetch 順序は確定、handleGotWorkflow の 2 ケース明記 |
| 3 | 設計判断の完結性 | 全差異に判断記載 | OK | 承認後の直接更新、GotResubmitResult の再フェッチ、Refresh 動作を明記 |
| 4 | スコープ境界 | 対象・対象外が明記 | OK | 対象: Detail.elm のみ。対象外: バックエンド・API・E2E テストコード |
| 5 | 技術的前提 | 前提が考慮されている | OK | Elm コンパイラの型チェック、Refresh 後の状態遷移パス |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-054 パターン A 準拠、Task/Detail・Workflow/New と同一パターン |

## 検証手順

```bash
just fmt          # フォーマット
just check-all    # lint + テスト + API テスト + E2E テスト
```
