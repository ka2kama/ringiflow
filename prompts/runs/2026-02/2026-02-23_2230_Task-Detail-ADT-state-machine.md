# 2026-02-23 Task/Detail.elm ADT ベースステートマシンリファクタリング

## 概要

#818: Task/Detail.elm のフラットな Model（9 フィールド + `RemoteData`）を ADT ベースステートマシンパターン（ADR-054）にリファクタリングした。Loading/Failed 状態で承認操作フィールド（comment, isSubmitting, pendingAction 等）が型レベルで存在しない構造にし、不正な状態を表現不可能にした。

## 実施内容

### 型構造の変更

フラットな Model を共有フィールド + PageState ADT に変更:

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

- `RemoteData` の import を削除し、自前の `PageState` カスタム型に置換
- `LoadedState` に 6 フィールドを格納（Loaded 時のみ存在）
- `NotAsked` バリアントを除去（dead code だった）

### update 関数の分割

- 外側 `update`: `GotTaskDetail` と `Refresh` を処理（状態遷移を担当）
- 内側 `updateLoaded`: Loaded 状態での全 Msg を処理
- `updateLoaded` は `shared`, `workflowDisplayNumber`, `stepDisplayNumber` をパラメータとして受け取る設計

`handleGotTaskDetail` で 2 つの文脈を区別:
- Loading/Failed → Loaded: 新しい `LoadedState` を `initLoaded` で構築
- Loaded → Loaded: 既存の `LoadedState` の `taskDetail` のみ更新（承認成功後の再取得で successMessage を保持）

### view 関数の変更

- `viewBody` で `model.state` をパターンマッチ（Loading / Failed / Loaded）
- `viewLoaded : Time.Zone -> LoadedState -> Html Msg` を新設
- view サブ関数のシグネチャ: `TaskDetail -> Model ->` → `LoadedState ->` に変更

### API 呼び出し関数の簡素化

- `approveStep : Shared -> LoadedState -> WorkflowStep -> Cmd Msg`（旧: `Model -> WorkflowStep -> Cmd Msg`）
- `rejectStep`, `requestChangesStep` も同様
- Loaded 状態が確定しているため、`model.task` のパターンマッチが不要に

### エラー表示の改善

elm-review の `NoUnused.CustomTypeConstructorArgs` により `Failed ApiError` の `ApiError` 値の使用が求められた。これを活用して `viewError` を改善:

- Before: 汎用メッセージ「データの取得に失敗しました。」
- After: `ErrorMessage.toUserMessage` による具体的なエラーメッセージ（ネットワークエラー、404 等を区別）

## 設計判断

| # | 判断 | 理由 |
|---|------|------|
| 1 | 全操作フィールドを LoadedState に配置 | 承認操作とそのフィードバックは Loaded 時のみ意味がある |
| 2 | GotTaskDetail を状態に応じて分岐 | 承認成功後の再取得で successMessage を保持するため |
| 3 | Refresh を外側 update で処理 | Loaded → Loading の状態遷移は外側の責務 |
| 4 | API 関数のシグネチャ簡素化 | Loaded 確定のためパターンマッチ不要 |
| 5 | NotAsked バリアント除去 | init で使用されておらず dead code |

## Designer.elm（#796）との共通パターン

| パターン | Designer.elm | Task/Detail.elm |
|---------|-------------|-----------------|
| 外側 Model | shared + ID + state | shared + displayNumbers + state |
| PageState | Loading / Failed / Loaded | 同一 |
| update 分割 | GotDefinition が外側 | GotTaskDetail + Refresh が外側 |
| Loaded 時の文脈保持 | N/A | Loaded→Loaded で taskDetail のみ更新 |

## 検証結果

- Elm コンパイル: OK
- elm-test: 454 tests passed
- elm-review: OK（`NoUnused.CustomTypeConstructorArgs` 対応後）
- Rust tests: 全パス
- API tests (Hurl): 全パス
- E2E tests (Playwright): 21 tests passed（approval, rejection, request-changes 含む）
- `just check-all`: exit code 0
