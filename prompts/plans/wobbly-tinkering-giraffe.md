# #478 フロントエンド - 多段階承認・差し戻し・コメント UI

## Issue

[#478](https://github.com/ka2kama/ringiflow/issues/478) フロントエンド: 多段階承認・差し戻し・コメント UI

## 前提

- バックエンド API は実装済み（#475, #476, #477 すべて CLOSED）
- フロントエンドのみの変更
- Elm + TEA パターン、Tailwind CSS

## スコープ

対象:
- Data 層（型、デコーダー、API クライアント）
- Page/Workflow/Detail.elm（差し戻し、再提出、コメント）
- Page/Workflow/List.elm（ChangesRequested ステータス表示）
- Page/Workflow/New.elm（多段階承認者選択）
- Route.elm（クエリパラメータ追加）
- 関連テスト

対象外:
- バックエンド変更
- 新規コンポーネント作成（既存コンポーネントを再利用）
- E2E テスト（Elm のユニットテストのみ）

## 設計判断

### D1: コメントセクションの配置

選択肢:
1. 詳細ページに埋め込み（タブなし）
2. タブ切り替えで表示

採用: 1（埋め込み）
理由: コメントは承認フローの一部として常に表示すべき情報。タブに隠すとコンテキストを失う。

### D2: 再提出 UI

選択肢:
1. 詳細ページ内でインライン編集モード
2. 別ページに遷移

採用: 1（インライン編集）
理由: 差し戻しコメントを見ながら修正できる。New.elm の `encodeFormValues` を再利用する。

### D3: 多段階承認者選択の状態管理

選択肢:
1. `Dict String ApproverSelector.State`（ステップ ID をキー）
2. `List ( String, ApproverSelector.State )`

採用: 1（Dict）
理由: ステップ ID でのルックアップが頻繁。Dict が自然。

### D4: コメントデータの配置

選択肢:
1. `Data.WorkflowComment` として独立モジュール
2. `Data.WorkflowInstance` に追加

採用: 1（独立モジュール）
理由: コメントはワークフローインスタンスと異なるライフサイクル（後から追加される）。SRP に沿う。

### D5: ステップ名の取得方法

現状: `WorkflowDefinition.approvalStepIds` は ID のみ返す。
対応: `approvalStepInfos : WorkflowDefinition -> List { id : String, name : String }` を追加。
formData JSON から `approval_steps` 配列の各要素の `id` と `name` を抽出する。

---

## Phase A: Data 層（型・デコーダー・API）

### 確認事項
- [x] 型: Status 型 → `Data/WorkflowInstance.elm` L20-27, 6バリアント（ChangesRequested なし）
- [x] 型: Decision 型 → `Data/WorkflowInstance.elm` L30-33, 2バリアント（DecisionRequestChanges なし）
- [x] パターン: デコーダーパイプライン → `Data/WorkflowInstance.elm` L91-106, Json.Decode.Pipeline 使用
- [x] パターン: API 関数 → `Api/Workflow.elm` L32-67, record-based config パターン
- [x] パターン: ステータス文字列変換 → `Data/WorkflowInstance.elm` L127-169, PascalCase（"Pending", "InProgress" 等）
- [x] 型: UserRef → `Data/UserRef.elm` L12, `{ id : String, name : String }`

### 実装内容

#### A1: WorkflowInstance.elm に ChangesRequested を追加

```elm
-- Status に追加
type Status
    = Draft
    | Pending
    | InProgress
    | Approved
    | Rejected
    | ChangesRequested  -- 追加

-- Decision に追加
type Decision
    = DecisionApproved
    | DecisionRejected
    | DecisionRequestChanges  -- 追加
```

各ヘルパー関数を更新:
- `statusToString`: `ChangesRequested -> "ChangesRequested"`
- `statusFromString`: `"ChangesRequested" -> Just ChangesRequested`
- `statusToJapanese`: `ChangesRequested -> "差し戻し"`
- `statusToCssClass`: `ChangesRequested -> "warning"`
- `statusDecoder`: ChangesRequested ケース追加
- `decisionToString`: `DecisionRequestChanges -> "RequestChanges"`
- `decisionFromString`: `"RequestChanges" -> Just DecisionRequestChanges`
- `decisionDecoder`: RequestChanges ケース追加

#### A2: Data/WorkflowComment.elm（新規）

```elm
module Data.WorkflowComment exposing (WorkflowComment, decoder, listDecoder)

type alias WorkflowComment =
    { id : String
    , workflowInstanceId : String
    , authorId : String
    , authorName : String
    , content : String
    , createdAt : String
    }
```

#### A3: Api/Workflow.elm に新規 API 関数追加

```elm
-- 差し戻し
requestChangesStep :
    { config : RequestConfig
    , workflowId : String
    , stepId : String
    , body : ApproveRejectRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg

-- 再提出
resubmitWorkflow :
    { config : RequestConfig
    , workflowId : String
    , body : ResubmitRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg

-- コメント取得
listComments :
    { config : RequestConfig
    , workflowId : String
    , toMsg : Result ApiError (List WorkflowComment) -> msg
    }
    -> Cmd msg

-- コメント投稿
postComment :
    { config : RequestConfig
    , workflowId : String
    , body : PostCommentRequest
    , toMsg : Result ApiError WorkflowComment -> msg
    }
    -> Cmd msg
```

新規リクエスト型:
```elm
type alias ResubmitRequest =
    { version : Int
    , formData : Encode.Value
    , approvers : List StepApproverRequest
    }

type alias PostCommentRequest =
    { content : String
    }
```

#### A4: WorkflowDefinition.elm にステップ情報取得関数を追加

```elm
type alias ApprovalStepInfo =
    { id : String
    , name : String
    }

approvalStepInfos : WorkflowDefinition -> List ApprovalStepInfo
```

#### A5: Route.elm に ChangesRequested を追加

```elm
-- workflowQueryParser の Dict に追加
( "changes_requested", ChangesRequested )

-- statusToQueryValue に追加
ChangesRequested -> "changes_requested"
```

### テストリスト

ユニットテスト:
- [ ] ChangesRequested の statusToString / statusFromString 往復
- [ ] ChangesRequested の statusToJapanese が "差し戻し"
- [ ] ChangesRequested の statusToCssClass が "warning"
- [ ] DecisionRequestChanges の decisionToString / decisionFromString 往復
- [ ] WorkflowComment デコーダー: 正常系
- [ ] WorkflowComment リストデコーダー: 空リスト
- [ ] approvalStepInfos: 複数ステップ定義から id と name を抽出
- [ ] Route: changes_requested クエリパラメータのパース
- [ ] Route: ChangesRequested の statusToQueryValue

ハンドラテスト（該当なし — フロントエンドのみ）

API テスト（該当なし — フロントエンドのみ）

E2E テスト（該当なし — Elm ユニットテストでカバー）

---

## Phase B: ステータス表示（リスト・詳細）

### 確認事項
- [x] パターン: List.elm の statusOptions → L309-317, `( "ステータス名", Status )` のリスト
- [x] パターン: Badge 表示 → List.elm L390, `Badge.view (statusToCssClass status) (statusToJapanese status)`
- [x] パターン: Detail.elm のステップカード → L455-530, viewStep 関数

### 実装内容

#### B1: List.elm に ChangesRequested フィルター追加

`statusOptions` リストに `( "差し戻し", ChangesRequested )` を追加。Badge 表示は Phase A で追加した `statusToCssClass`/`statusToJapanese` により自動対応。

#### B2: Detail.elm のステップ進行状況表示を強化

`viewStepProgress` 関数を追加。全ステップをプログレスバー形式で表示し、現在のアクティブステップをハイライトする:

```elm
viewStepProgress : WorkflowInstance -> Html msg
```

- 各ステップを水平に並べる
- ステータスに応じた色分け（Approved: green, Rejected: red, ChangesRequested: warning, Active: blue, Pending: gray）
- ステップ名と担当者名を表示

### テストリスト

ユニットテスト:
- [ ] statusOptions に ChangesRequested が含まれる（Phase A のテストでカバー済みのため、ここでは List.elm 固有の表示テストは不要）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

注: Phase B のテストは主に Phase A で追加済みの型テストに依存。ステップ進行状況表示はビュー関数であり、Elm のユニットテストでは DOM テストが困難なため、手動確認で担保する。

---

## Phase C: 差し戻し（Request Changes）フロー

### 確認事項
- [x] パターン: PendingAction → Detail.elm L38-40, `ConfirmApprove WorkflowStep | ConfirmReject WorkflowStep`
- [x] パターン: ConfirmDialog 使用 → Detail.elm L336-359, `ConfirmDialog.view` with title/message/confirmText/cancelText
- [x] パターン: handleApprovalResult → Detail.elm L218-241, 成功時は workflow 更新 + success メッセージ
- [x] 型: Button.Warning → Component/Button.elm L18, `Warning` バリアント存在確認済み

### 実装内容

#### C1: PendingAction に ConfirmRequestChanges を追加

```elm
type PendingAction
    = ConfirmApprove WorkflowStep
    | ConfirmReject WorkflowStep
    | ConfirmRequestChanges WorkflowStep  -- 追加
```

#### C2: Msg に RequestChanges 関連メッセージを追加

```elm
| ClickRequestChanges WorkflowStep
| ConfirmRequestChanges_  -- confirm dialog の確定
| GotRequestChangesResult (Result ApiError WorkflowInstance)
```

#### C3: viewApprovalSection に差し戻しボタンを追加

承認・却下ボタンの横に Warning スタイルの「差し戻し」ボタンを追加。条件: 現在のユーザーがアクティブステップの担当者である場合のみ表示。

#### C4: update で RequestChanges メッセージを処理

`handleApprovalResult` を再利用。API 呼び出しは `Api.Workflow.requestChangesStep`。

#### C5: ConfirmDialog の差し戻し用表示

```elm
ConfirmRequestChanges step ->
    ConfirmDialog.view
        { title = "差し戻し確認"
        , message = "ステップ「" ++ step.stepName ++ "」を差し戻しますか？"
        , confirmText = "差し戻し"
        , cancelText = "キャンセル"
        , onConfirm = ConfirmRequestChanges_
        , onCancel = CancelAction
        , variant = ConfirmDialog.Warning
        }
```

### テストリスト

ユニットテスト:
- [ ] requestChangesStep API 関数のエンコード: version と comment が正しく JSON エンコードされる

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

注: Elm では副作用（Cmd）をユニットテストで直接テストできないため、API 関数のリクエストボディエンコードをテスト。ConfirmDialog の表示は手動確認で担保。

---

## Phase D: コメントセクション

### 確認事項
- [x] パターン: RemoteData 使用 → Detail.elm L27, `workflow : RemoteData ApiError WorkflowInstance`
- [x] パターン: API 呼び出しの init パターン → Detail.elm L67-97, init で API 呼び出し

### 実装内容

#### D1: Detail.elm の Model にコメント関連フィールドを追加

```elm
type alias Model =
    { ...
    , comments : RemoteData ApiError (List WorkflowComment)  -- 追加
    , newComment : String  -- 追加
    , isPostingComment : Bool  -- 追加
    }
```

#### D2: Msg にコメント関連メッセージを追加

```elm
| GotComments (Result ApiError (List WorkflowComment))
| UpdateNewComment String
| SubmitComment
| GotPostCommentResult (Result ApiError WorkflowComment)
```

#### D3: init でコメント一覧を取得

ワークフロー詳細の取得と並行して `Api.Workflow.listComments` を呼び出す。

#### D4: viewCommentSection を実装

```elm
viewCommentSection : Model -> Html Msg
```

- コメント一覧をタイムスタンプ順に表示
- 各コメント: 投稿者名、投稿日時、内容
- 入力フォーム: textarea + 投稿ボタン
- Loading / Error 状態のハンドリング

#### D5: コメント投稿後のリスト更新

投稿成功時、返された `WorkflowComment` をリストの末尾に追加（再取得不要）。

### テストリスト

ユニットテスト:
- [ ] PostCommentRequest のエンコード: content フィールドが正しくエンコードされる
- [ ] WorkflowComment デコーダー: 全フィールドが正しくデコードされる（Phase A でカバー済み）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

注: コメントの表示・投稿は主にビュー関数と Cmd であり、Elm ユニットテストでのカバー範囲は限定的。手動確認で UI 動作を担保する。

---

## Phase E: 再提出（Resubmit）フロー

### 確認事項
- [x] パターン: encodeFormValues → New.elm L768, `Dict String String -> Encode.Value`
- [x] パターン: フォームフィールドレンダリング → New.elm L543-622, viewFormField 関数
- [x] パターン: ApproverSelector 使用 → New.elm L195-210, Msg と state 管理パターン
- [x] 型: WorkflowInstance.formData → `Data/WorkflowInstance.elm` L51, `Dict String String`
- [x] 型: WorkflowInstance.steps → `Data/WorkflowInstance.elm` L56, `List WorkflowStep`

### 実装内容

#### E1: Detail.elm の Model に再提出関連フィールドを追加

```elm
type alias Model =
    { ...
    , isEditing : Bool  -- 追加
    , editFormData : Dict String String  -- 追加
    , editApprovers : Dict String ApproverSelector.State  -- 追加
    , users : RemoteData ApiError (List UserItem)  -- 追加（承認者選択用）
    , resubmitValidationErrors : Dict String String  -- 追加
    }
```

#### E2: Msg に再提出関連メッセージを追加

```elm
| StartEditing
| CancelEditing
| UpdateEditFormField String String
| UpdateEditApprover String ApproverSelector.Msg
| SubmitResubmit
| GotResubmitResult (Result ApiError WorkflowInstance)
| GotUsers (Result ApiError (List UserItem))
```

#### E3: 再提出ボタンの表示条件

ワークフローのステータスが `ChangesRequested` かつ現在のユーザーが起案者（`initiatedBy`）である場合のみ「再提出」ボタンを表示。

#### E4: 編集モードの UI

`isEditing = True` のとき:
- formData を編集可能なフォームフィールドとして表示（New.elm の viewFormField パターンを再利用）
- 各ステップの承認者を ApproverSelector で再選択可能
- 「再提出」「キャンセル」ボタン

#### E5: 再提出 API 呼び出し

```elm
Api.Workflow.resubmitWorkflow
    { config = shared.requestConfig
    , workflowId = workflowId
    , body =
        { version = workflow.version
        , formData = encodeFormValues model.editFormData
        , approvers = buildResubmitApprovers model
        }
    , toMsg = GotResubmitResult
    }
```

#### E6: 再提出成功後の処理

成功時: `isEditing = False` にリセット、更新された WorkflowInstance で表示を更新、成功メッセージ表示。

### テストリスト

ユニットテスト:
- [ ] ResubmitRequest のエンコード: version, formData, approvers が正しくエンコードされる
- [ ] approvers の StepApproverRequest エンコード: stepId と assignedTo が正しい

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase F: 多段階承認者選択（New.elm）

### 確認事項
- [x] パターン: ApproverSelector.State → Component/ApproverSelector.elm L28-33
- [x] パターン: ApproverSelector.init → Component/ApproverSelector.elm L36, `State { selection = NotSelected, ... }`
- [x] パターン: buildApprovers → New.elm L642-660, 全ステップに同じ承認者を割り当て（TODO #478 あり）
- [x] 型: WorkflowDefinition.approvalStepIds → Data/WorkflowDefinition.elm L95, `List String`
- [x] パターン: validateFormWithApprover → New.elm L662-693, 単一承認者のバリデーション

### 実装内容

#### F1: Model の approver を Dict に変更

```elm
-- Before
type alias Model =
    { ...
    , approver : ApproverSelector.State
    }

-- After
type alias Model =
    { ...
    , approvers : Dict String ApproverSelector.State  -- キー: ステップ ID
    }
```

#### F2: init で各ステップ用の ApproverSelector.State を初期化

`WorkflowDefinition.approvalStepInfos` でステップ情報を取得し、各ステップ ID に対して `ApproverSelector.init` を生成して Dict に格納。

#### F3: Msg を更新

```elm
-- Before
| ApproverSearchChanged String
| ApproverSelected UserItem
| ApproverCleared
| ApproverKeyDown String
| ApproverDropdownClosed

-- After
| ApproverSearchChanged String String  -- stepId, search
| ApproverSelected String UserItem     -- stepId, user
| ApproverCleared String               -- stepId
| ApproverKeyDown String String        -- stepId, key
| ApproverDropdownClosed String        -- stepId
```

#### F4: viewApproverSection を複数ステップ対応に更新

各ステップを縦に並べ、ステップ名ラベル + ApproverSelector を表示:

```elm
viewApproverSection : Model -> List UserItem -> Html Msg
```

ステップ情報がない場合（定義取得中等）はフォールバック表示。

#### F5: buildApprovers を更新

```elm
buildApprovers : Model -> List StepApproverRequest
```

Dict の各エントリから選択済みの承認者を `StepApproverRequest` に変換。

#### F6: validateFormWithApprover を更新

全ステップで承認者が選択されていることを検証。未選択のステップがあればバリデーションエラー。

### テストリスト

ユニットテスト:
- [ ] approvalStepInfos: 定義 JSON から id と name のペアリストを正しく抽出
- [ ] approvalStepInfos: approval_steps が空の場合は空リスト

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

注: ApproverSelector のインタラクションは既存テストでカバー済み。Dict ベースの状態管理は Elm の型システムで安全性が担保される。

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Status/Decision の文字列フォーマットが PascalCase であることを明記していなかった | 曖昧 | Phase A に PascalCase パターン（"ChangesRequested", "RequestChanges"）を明記 |
| 1回目 | Detail.elm が既に 643 行あり、Phase C-E の追加で 500 行閾値を大幅超過するリスク | 既存手段の見落とし | 必要に応じてヘルパーモジュール抽出を Phase E の Refactor で検討する旨を記録。ただし現時点では premature abstraction を避ける |
| 1回目 | ConfirmDialog.Warning バリアントの存在が未確認だった | 未定義 | ConfirmDialog.elm を確認。variant フィールドが存在しない場合は confirmText のスタイルで対応する方針に修正 |
| 2回目 | approvalStepIds が ID のみ返し名前が取得できない問題 | 不完全なパス | D5 として approvalStepInfos 関数の追加を設計判断に記載 |
| 2回目 | 再提出時にユーザー一覧が必要だが Detail.elm の init で取得していない | 不完全なパス | Phase E に users フィールドと GotUsers メッセージを追加。編集モード開始時にユーザー一覧を取得 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了基準 6 項目がすべて Phase に対応している | OK | 完了基準1→Phase B, 完了基準2→Phase C, 完了基準3→Phase E, 完了基準4→Phase D, 完了基準5→Phase B, 完了基準6→Phase F。Phase A は全 Phase の共通基盤 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 「必要に応じて」「あれば」等の記述なし。各 Phase の実装内容が型シグネチャレベルで確定 |
| 3 | 設計判断の完結性 | 全ての設計判断に選択肢・理由が記載されている | OK | D1-D5 の5つの設計判断すべてに選択肢と採用理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 「スコープ」セクションに明記。バックエンド変更・E2E テスト・新規コンポーネントは対象外 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | PascalCase フォーマット、ApproverSelector の既存インターフェース、ConfirmDialog の variant 有無を確認済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Issue #478 の完了基準、バックエンド API（#475, #476, #477）の仕様と整合 |
