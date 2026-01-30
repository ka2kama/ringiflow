# Phase 6: フロントエンド（Elm）

## 目的

ワークフロー詳細画面に承認/却下ボタンを追加し、楽観的ロックによる競合検出（409 Conflict）を適切にハンドリングする。

## 変更内容

### 1. エラー型の拡張

```elm
-- Api.elm
type ApiError
    = BadRequest ProblemDetails
    | Unauthorized
    | Forbidden ProblemDetails
    | NotFound ProblemDetails
    | Conflict ProblemDetails  -- 追加: 409 Conflict
    | ServerError ProblemDetails
    | NetworkError
    | Timeout
    | DecodeError String
```

### 2. データ型の拡張

```elm
-- Data/WorkflowInstance.elm
type alias WorkflowInstance =
    { ...
    , version : Int           -- 追加: 楽観的ロック用
    , steps : List WorkflowStep  -- 追加: 承認ステップ
    }

type alias WorkflowStep =
    { id : String
    , stepName : String
    , status : StepStatus
    , decision : Maybe Decision
    , assignedTo : Maybe String
    , comment : Maybe String
    , version : Int
    }

type StepStatus
    = StepPending
    | StepActive
    | StepCompleted
    | StepSkipped

type Decision
    = DecisionApproved
    | DecisionRejected
```

### 3. API クライアントの追加

```elm
-- Api/Workflow.elm
approveStep :
    { config : RequestConfig
    , workflowId : String
    , stepId : String
    , body : ApproveRejectRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg

rejectStep :
    { config : RequestConfig
    , workflowId : String
    , stepId : String
    , body : ApproveRejectRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
```

### 4. 詳細ページの拡張

```elm
-- Page/Workflow/Detail.elm

type Msg
    = ...
    | ClickApprove WorkflowStep
    | ClickReject WorkflowStep
    | GotApproveResult (Result ApiError WorkflowInstance)
    | GotRejectResult (Result ApiError WorkflowInstance)
    | DismissMessage
```

## 設計判断

### なぜ StepStatus を別名（StepPending 等）にしたか

Elm は同一モジュール内でコンストラクタ名の重複を許容しない。
`Status.Pending` と `StepStatus.Pending` が衝突するため、プレフィックスを付けた。

代替案:
- 別モジュールに分離する
  - トレードオフ: インポートが複雑になる
- qualified import を強制する
  - トレードオフ: 既存コードへの影響が大きい

### エラーメッセージのユーザーフレンドリー化

```elm
apiErrorToMessage : ApiError -> String
apiErrorToMessage error =
    case error of
        Conflict problem ->
            "このワークフローは既に更新されています。最新の状態を取得してください。（" ++ problem.detail ++ "）"

        Forbidden problem ->
            "この操作を実行する権限がありません。（" ++ problem.detail ++ "）"
        ...
```

409 Conflict はユーザーにとって理解しづらいエラー。
「別のユーザーが更新しました」という文脈で説明し、リカバリー手順（再読み込み）を示す。

### 承認ボタンの表示条件

```elm
findActiveStepForUser : List WorkflowStep -> Maybe String -> Maybe WorkflowStep
findActiveStepForUser steps maybeUserId =
    case maybeUserId of
        Nothing -> Nothing  -- 未ログインなら非表示
        Just userId ->
            steps
                |> List.filter (\step ->
                    step.status == WorkflowInstance.StepActive
                    && step.assignedTo == Just userId  -- 担当者のみ
                )
                |> List.head
```

承認/却下ボタンは以下の条件をすべて満たす場合のみ表示:
1. ユーザーがログイン済み
2. アクティブなステップが存在する
3. そのステップの担当者が現在のユーザー

これにより、権限のないユーザーには操作できないことが視覚的に明確になる。

## RemoteData パターン

```elm
type RemoteData a
    = NotAsked   -- まだリクエストしていない
    | Loading    -- リクエスト中
    | Failure    -- エラー
    | Success a  -- 成功
```

非同期データの状態を型で表現し、各状態に応じた UI を描画する。
これにより「ローディング中にボタンを押せてしまう」などの不正状態を防ぐ。

## 次のステップ

- 統合テスト（E2E テスト）
- コメント入力 UI（任意コメントの送信）
- 承認履歴の表示
