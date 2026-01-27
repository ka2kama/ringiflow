# Phase 10: 申請一覧・詳細ページ

## 概要

Elm で申請一覧ページ（`Page/Workflow/List.elm`）と詳細ページ（`Page/Workflow/Detail.elm`）を実装する。

### 対応 Issue

[#115 フロントエンド ワークフロー申請フォーム](https://github.com/ka2kama/ringiflow/issues/115)

## 実装内容

### 1. ルーティング拡張

```elm
type Route
    = Home
    | Workflows           -- 申請一覧（新規追加）
    | WorkflowNew
    | WorkflowDetail String  -- 申請詳細（新規追加）
    | NotFound
```

URL パース:

| URL パターン | Route |
|-------------|-------|
| `/workflows` | `Workflows` |
| `/workflows/new` | `WorkflowNew` |
| `/workflows/:id` | `WorkflowDetail id` |

注意: `/workflows/new` と `/workflows/:id` の順序が重要。`new` は `:id` より先にマッチさせる。

### 2. 一覧ページ（List.elm）

#### 機能

- 自分の申請一覧を取得・表示
- ステータスによるフィルタリング
- 詳細ページへの遷移リンク
- 新規申請ボタン

#### 状態管理

```elm
type alias Model =
    { session : Session
    , workflows : RemoteData (List WorkflowInstance)
    , statusFilter : Maybe Status
    }

type RemoteData a
    = Loading
    | Failure
    | Success a
```

設計ポイント:

- `NotAsked` を削除: `init` 時に必ず API を呼び出すため不要
- `Failure` は引数なし: 現在の UI ではエラー種別によらず同じメッセージを表示

#### フィルタリング

```elm
viewWorkflowList : Maybe Status -> List WorkflowInstance -> Html Msg
viewWorkflowList statusFilter workflows =
    let
        filteredWorkflows =
            case statusFilter of
                Nothing ->
                    workflows

                Just status ->
                    List.filter (\w -> w.status == status) workflows
    in
    ...
```

フィルタはクライアントサイドで実装（サーバーサイドフィルタは将来課題）。

#### ステータス表示ヘルパー

```elm
-- Data/WorkflowInstance.elm に追加
statusToJapanese : Status -> String
statusToJapanese status =
    case status of
        Draft -> "下書き"
        Pending -> "申請待ち"
        InProgress -> "承認中"
        Approved -> "承認済み"
        Rejected -> "却下"
        Cancelled -> "キャンセル"

statusToCssClass : Status -> String
statusToCssClass status =
    case status of
        Draft -> "status-draft"
        Pending -> "status-pending"
        ...
```

### 3. 詳細ページ（Detail.elm）

#### 機能

- ワークフローインスタンスの詳細表示
- フォームデータの表示（ラベル付き）
- 一覧への戻るリンク

#### 状態管理

```elm
type alias Model =
    { session : Session
    , workflowId : String
    , workflow : RemoteData WorkflowInstance
    , definition : RemoteData WorkflowDefinition
    }
```

2 つの API を順次呼び出す:

1. ワークフロー取得 → `definitionId` を取得
2. 定義取得 → フォームフィールドのラベルを取得

#### フォームデータ表示

フォームデータは JSON 形式で保存されている。定義からフィールドラベルを取得して表示する。

```elm
viewFormDataWithLabels : WorkflowDefinition -> Decode.Value -> Html Msg
viewFormDataWithLabels definition formData =
    case DynamicForm.extractFormFields definition.definition of
        Ok fields ->
            dl []
                (List.concatMap (viewFormField formData) fields)

        Err _ ->
            viewRawFormData formData  -- フォールバック: 生 JSON 表示
```

フォールバック設計:

- 定義取得失敗時 → 生 JSON を `<pre>` で表示
- フィールド抽出失敗時 → 同上

これにより、定義変更やデータ不整合があっても最低限の表示は保証される。

## 設計判断

### RemoteData の Failure から ApiError を削除

elm-review の警告「未使用の型引数」に対応。

```elm
-- 変更前
type RemoteData a
    = NotAsked
    | Loading
    | Failure ApiError  -- ApiError は使われていなかった
    | Success a

-- 変更後
type RemoteData a
    = Loading
    | Failure  -- 引数なし
    | Success a
```

理由:

- 現在の UI ではエラーの種類によらず同じメッセージを表示
- 将来エラー種別ごとの表示が必要になったら再度追加可能

### 順次 API 呼び出しの実装

詳細ページでは 2 つの API を順次呼び出す。

```elm
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotWorkflow result ->
            case result of
                Ok workflow ->
                    ( { model | workflow = Success workflow }
                    , WorkflowDefinitionApi.getDefinition
                        { config = Session.toRequestConfig model.session
                        , id = workflow.definitionId
                        , toMsg = GotDefinition
                        }
                    )
                ...
```

採用理由:

- 1 回目のレスポンス（`definitionId`）が 2 回目のリクエストに必要
- Elm では Cmd のチェーンは Task 変換が必要で複雑になるため、メッセージ駆動で実装

代替案:

- BFF で結合 API を提供する（サーバーサイド JOIN）
- `Task.andThen` で Cmd をチェーン

### クライアントサイドフィルタ

一覧ページのステータスフィルタはクライアントサイドで実装。

```elm
List.filter (\w -> w.status == status) workflows
```

採用理由:

- MVP では申請件数が少ないため、全件取得後にフィルタで十分
- 実装がシンプル

将来の改善:

- 件数が増えたらサーバーサイドフィルタ + ページネーション
- クエリパラメータ `?status=draft` でフィルタ

## ファイル構成

```
frontend/src/
├── Route.elm                 # ルーティング（Workflows, WorkflowDetail 追加）
├── Main.elm                  # エントリポイント（ページ統合）
├── Page/
│   └── Workflow/
│       ├── List.elm          # 申請一覧ページ
│       └── Detail.elm        # 申請詳細ページ
└── Data/
    └── WorkflowInstance.elm  # ステータスヘルパー追加
```

## テスト

### Route テスト

`tests/RouteTest.elm` に 10 件のテストを追加:

- URL → Route パース（各ルート + 不明 URL）
- Route → URL 文字列変換

### ステータスヘルパーテスト

`tests/Data/WorkflowInstanceTest.elm` に 12 件のテストを追加:

- `statusToJapanese`: 全ステータスの日本語変換
- `statusToCssClass`: 全ステータスの CSS クラス変換

## 学習ポイント

1. **Nested TEA**: Main.elm でサブページの Model/Msg/update/view を統合
2. **URL パース**: `Url.Parser` による型安全なルーティング
3. **順次 API 呼び出し**: メッセージ駆動での非同期処理チェーン
4. **フォールバック UI**: データ取得失敗時の graceful degradation

## 関連ドキュメント

- [申請フォーム UI 設計](../../03_詳細設計書/10_ワークフロー申請フォームUI設計.md)
- [Phase 8: フロントエンド API クライアント](./08_Phase8_フロントエンドAPIクライアント.md)
- [Phase 9: 申請フォーム UI](./09_Phase9_申請フォームUI.md)
