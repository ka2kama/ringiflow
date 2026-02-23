# #817 Workflow/New.elm ADT ステートマシンリファクタリング

## Context

ADR-054（ADT ベースステートマシンパターンの標準化）に基づき、Workflow/New.elm のフラットな Model（13 フィールド）を ADT で構造化する。Epic #822 の 2 番目の Story。#818（Task/Detail.elm）で確立されたパターンを踏襲する。

現在の問題: `definitions = Loading` 時や `selectedDefinitionId = Nothing` 時にフォームフィールド（`title`, `formValues`, `approvers` 等）が型レベルで存在し、不正な状態が表現可能。

## 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Page/Workflow/New.elm` | Model ADT 化、update/view 分割 |
| `frontend/tests/Page/Workflow/NewTest.elm` | テストヘルパー・アサーション更新 |
| `frontend/review/src/ReviewConfig.elm` | elm-review 除外削除（行 25, 35） |

対象外: Main.elm（全 API シグネチャ不変）、E2E テスト（変更不要）

## 型設計

```elm
type alias Model =
    { shared : Shared
    , users : RemoteData ApiError (List UserItem)  -- 独立した並行 fetch
    , state : PageState
    }

type PageState
    = Loading
    | Failed ApiError
    | Loaded LoadedState

type alias LoadedState =
    { definitions : List WorkflowDefinition  -- RemoteData → 展開済み
    , formState : FormState
    }

type FormState
    = SelectingDefinition                    -- 定義未選択
    | Editing EditingState                   -- フォーム入力中

type alias EditingState =
    { selectedDefinition : WorkflowDefinition  -- Maybe 排除
    , title : String
    , formValues : Dict String String
    , validationErrors : Dict String String
    , approvers : Dict String ApproverSelector.State
    , savedWorkflow : Maybe WorkflowInstance    -- 保存完了の表現
    , saveMessage : Maybe SaveMessage
    , submitting : Bool
    , isDirty_ : Bool
    }
```

設計判断:

| # | 判断 | 理由 |
|---|------|------|
| 1 | `users` を外側 Model に配置 | `definitions` と独立して並行 fetch。ApproverSelector が内部で RemoteData をハンドル |
| 2 | 二段階: PageState + FormState | Loading/Loaded に加え、Loaded 内の「定義未選択/編集中」も型で分離 |
| 3 | `selectedDefinition : WorkflowDefinition` | Maybe 排除。繰り返しの定義検索が不要に（`getSelectedDefinition` 削除可能） |
| 4 | `saveMessage` を EditingState に配置 | SaveDraft/Submit は Editing 状態でのみ到達。定義未選択エラーは型レベルで消える |
| 5 | `NotAsked` バリアント除去 | init で即座に fetch 開始するため dead code（#818 と同様） |
| 6 | 「保存完了」は `savedWorkflow = Just ...` | 現在の UI では保存後もフォーム表示不変。別バリアントにするメリットなし |

## update 分割設計

```
update（外側）
├── GotDefinitions → Loading → Loaded/Failed
├── GotUsers → users フィールド更新
└── _ → Loaded 時 updateLoaded に委譲

updateLoaded
├── SelectDefinition → SelectingDefinition → Editing（新 EditingState 構築）
└── _ → Editing 時 updateEditing に委譲

updateEditing
├── UpdateTitle, UpdateField（フォーム入力 + markDirty）
├── UpdateApproverSearch, SelectApprover, ClearApprover, ApproverKeyDown, CloseApproverDropdown
├── SaveDraft, GotSaveResult
├── Submit, GotSaveAndSubmitResult, GotSubmitResult
└── ClearMessage
```

注意: `SelectDefinition` は二回目以降（定義変更）も `updateLoaded` で処理。新しい `EditingState` を構築してフォームリセット。dirty 状態は前の EditingState から引き継ぎ判定。

## view 分割設計

```
view
└── viewBody（PageState パターンマッチ）
    ├── Loading → LoadingSpinner.view
    ├── Failed err → ErrorState.viewSimple (ErrorMessage.toUserMessage ...)
    └── Loaded → viewLoaded（FormState パターンマッチ）
        ├── SelectingDefinition → viewDefinitionSelector ... Nothing
        └── Editing → viewSaveMessage + viewDefinitionSelector + viewFormInputs
```

`viewError` で `ApiError` を使用 → `ErrorMessage.toUserMessage { entityName = "ワークフロー定義" }` で具体的エラーメッセージ。elm-review `NoUnused.CustomTypeConstructorArgs` の除外が不要に。

## Phase 1: Model ADT 化 + update/view 分割 + テスト更新

### 確認事項

- 型: `WorkflowDefinition` → `frontend/src/Data/WorkflowDefinition.elm:61-71`（9 フィールドの record alias）
- 型: `ApprovalStepInfo` → `{ id : String, name : String }`（`approvalStepInfos` が `definition` JSON をデコード）
- 型: `ApproverSelector.State`, `ApproverSelector.init` → `frontend/src/Component/ApproverSelector.elm`
- パターン: Task/Detail.elm の update/view 分割 → `frontend/src/Page/Task/Detail.elm:173-199, 405-447`
- パターン: DesignerTest.elm の `expectLoaded` ヘルパー → `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm:170-177`
- パターン: DesignerTest.elm の `testDefinition` 構築 → 同ファイル:948-959（承認ステップ付き JSON が必要）
- ライブラリ: `ErrorState.viewSimple : String -> Html msg` → `frontend/src/Component/ErrorState.elm:72-78`
- ライブラリ: `ErrorMessage.toUserMessage` → `frontend/src/Api/ErrorMessage.elm:27-55`
- elm-review: `ReviewConfig.elm:25,35` の `ignoreErrorsForFiles` 除外 → リファクタリング後に除外削除

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ページ初期表示 → ローディング → 定義一覧表示 | 正常系 | ユニット |
| 2 | 定義取得失敗 → エラー表示 | 準正常系 | ユニット |
| 3 | 定義を選択 → フォーム表示 | 正常系 | ユニット |
| 4 | フォーム入力 → 下書き保存 → 成功メッセージ | 正常系 | ユニット |
| 5 | タイトル空で下書き保存 → バリデーションエラー | 準正常系 | ユニット |
| 6 | フォーム入力 → 申請（承認者含む） | 正常系 | ユニット |
| 7 | 承認者未選択で申請 → バリデーションエラー | 準正常系 | ユニット |
| 8 | 承認者キーボード操作 | 正常系 | ユニット |
| 9 | dirty 状態管理 | 正常系 | ユニット |
| 10 | E2E: 申請フォーム → 申請 → 一覧反映 | 正常系 | E2E（既存、変更不要） |

### テストリスト

ユニットテスト:

新規（状態遷移テスト）:
- [ ] GotDefinitions Ok で state が Loaded.SelectingDefinition になる
- [ ] GotDefinitions Err で state が Failed になる
- [ ] SelectDefinition で formState が Editing になり approvers が初期化される

既存テスト移行:
- [ ] SaveDraft: タイトル空でバリデーションエラー
- [ ] SaveDraft: 入力済みで submitting = True
- [ ] Submit: 承認者未選択でバリデーションエラー
- [ ] Submit: タイトル空 + 承認者未選択で複数エラー
- [ ] ApproverKeyDown: ArrowDown/ArrowUp/Enter/Escape
- [ ] isDirty: 初期状態 False、UpdateTitle で True、GotSaveResult Ok で False

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト: 既存 `tests/e2e/tests/workflow.spec.ts` が変更なしでパス

### テスト設計

メッセージ経由のモデル構築（行動的アプローチ）:

```elm
-- ヘルパー
sendMsg : Msg -> Model -> Model
sendMsg msg model = New.update msg model |> Tuple.first

-- テスト用定義（承認ステップ付き definition JSON）
testDefinition : WorkflowDefinition
testDefinition =
    { id = "def-001", name = "テスト定義", description = Nothing, version = 1
    , definition = Encode.object
        [ ( "steps", Encode.list identity
            [ Encode.object
                [ ( "id", Encode.string "step-1" )
                , ( "name", Encode.string "承認" )
                , ( "type", Encode.string "approval" )
                ]
            ] )
        ]
    , status = "published", createdBy = "u-001"
    , createdAt = "2026-01-01T00:00:00Z", updatedAt = "2026-01-01T00:00:00Z"
    }

-- ロード完了モデル
loadedModel = initialModel |> sendMsg (GotDefinitions (Ok [ testDefinition ]))

-- 編集中モデル（承認ステップ付き）
editingModel = loadedModel |> sendMsg (SelectDefinition "def-001")

-- アサーションヘルパー
expectEditing : (EditingState -> Expectation) -> Model -> Expectation
```

Module exposing に `PageState(..)`, `FormState(..)`, `EditingState`, `LoadedState` を追加（テスト用）。

### 実装手順

1. 型定義変更（Model, PageState, LoadedState, FormState, EditingState, initEditing）
2. init 更新（`state = Loading`, `users = Loading`）
3. update 分割（update → updateLoaded → updateEditing）
4. ヘルパー関数更新（markDirty/clearDirty を EditingState 対応、validateForm/buildApprovers を EditingState 引数に、`getSelectedDefinition` 削除）
5. isDirty/updateShared を新構造に対応
6. view 分割（viewBody → viewLoaded → viewEditing）、viewError で ApiError 使用
7. テスト更新（ヘルパー追加 + 既存テスト移行 + 新規状態遷移テスト）
8. ReviewConfig.elm 除外削除（行 25, 35）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `saveMessage` の配置場所が不明確 | 状態依存フィールド | EditingState に配置。SaveDraft/Submit は Editing でのみ到達 |
| 2回目 | `viewError` が ApiError を未使用 | 既存手段の見落とし | `ErrorState.viewSimple` + `ErrorMessage.toUserMessage` を使用 |
| 3回目 | `users` の GotUsers が Loading 状態で到着する場合 | 不完全なパス | `users` を外側 Model に配置。state に依存せず更新可能 |
| 4回目 | テストの直接フィールドアクセスが ADT 化後に壊れる | テスト層網羅漏れ | メッセージ経由構築 + `expectEditing` ヘルパーに移行 |
| 5回目 | 二回目の SelectDefinition（定義変更）の dirty 引き継ぎ | 不完全なパス | 前の EditingState の isDirty_ を確認し、Port 重複送信を防止 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 全13フィールド、全15 Msg バリアント、全 view 関数、既存テスト16ケース + 新規3ケースを計画に含む |
| 2 | 曖昧さ排除 | OK | saveMessage 配置、ClearMessage 処理、isDirty 実装、viewError 実装が確定 |
| 3 | 設計判断の完結性 | OK | 6 つの設計判断に理由を記載。users 配置、二段階 ADT、Maybe 排除等 |
| 4 | スコープ境界 | OK | 対象: New.elm, NewTest.elm, ReviewConfig.elm。対象外: Main.elm, E2E テスト |
| 5 | 技術的前提 | OK | WorkflowDefinition 型（9フィールド record）、approvalStepInfos（JSON デコード）、ErrorState.viewSimple（string → Html）を確認済み |
| 6 | 既存ドキュメント整合 | OK | ADR-054 パターン A 準拠、#818 パターン踏襲、DesignerTest ヘルパーパターン参照 |

## 検証

```bash
# Step 1: Elm コンパイル
cd frontend && npx elm make src/Main.elm --output=/dev/null

# Step 2: elm-test
just test-elm

# Step 3: elm-review
just lint-elm

# Step 4: 全テスト
just check-all
```

E2E テスト（`tests/e2e/tests/workflow.spec.ts`）が変更なしでパスすることを確認。
