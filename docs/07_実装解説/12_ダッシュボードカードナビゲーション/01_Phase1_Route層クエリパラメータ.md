# Phase 1: Route 層のクエリパラメータ対応

## 対応 Issue

[#267](https://github.com/ka2kama/ringiflow/issues/267)

## 概要

`Route` 型に `WorkflowFilter` を埋め込み、URL クエリパラメータ（`?status=in_progress`, `?completed_today=true`）のパースと生成をサポートする。

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`frontend/src/Route.elm`](../../../../frontend/src/Route.elm) | フィルタ型定義、クエリパラメータのパース/生成 |
| [`frontend/tests/RouteTest.elm`](../../../../frontend/tests/RouteTest.elm) | パース/生成/ラウンドトリップテスト |

## 実装内容

### WorkflowFilter 型

```elm
type alias WorkflowFilter =
    { status : Maybe Status
    , completedToday : Bool
    }

emptyWorkflowFilter : WorkflowFilter
emptyWorkflowFilter =
    { status = Nothing, completedToday = False }
```

### Route 型の変更

```elm
type Route
    = Home
    | Workflows WorkflowFilter  -- 変更: フィルタを保持
    | WorkflowNew
    | WorkflowDetail Int
    | Tasks
    | TaskDetail Int Int
    | NotFound
```

### クエリパラメータのパース

```elm
workflowQueryParser : Query.Parser WorkflowFilter
workflowQueryParser =
    Query.map2 WorkflowFilter
        (Query.enum "status"
            (Dict.fromList
                [ ( "draft", Draft ), ( "pending", Pending )
                , ( "in_progress", InProgress ), ( "approved", Approved )
                , ( "rejected", Rejected ), ( "cancelled", Cancelled )
                ]
            )
        )
        completedTodayParser

completedTodayParser : Query.Parser Bool
completedTodayParser =
    Query.custom "completed_today"
        (\values ->
            case values of
                [ "true" ] -> True
                _ -> False
        )
```

### URL 生成

```elm
Workflows filter ->
    Builder.absolute [ "workflows" ]
        (List.filterMap identity
            [ filter.status
                |> Maybe.map (\s -> Builder.string "status" (statusToQueryValue s))
            , if filter.completedToday then
                Just (Builder.string "completed_today" "true")
              else
                Nothing
            ]
        )
```

## テスト

| テストケース | 検証内容 |
|-------------|---------|
| fromUrl: クエリなし | `Workflows emptyWorkflowFilter` |
| fromUrl: `?status=in_progress` | `Workflows { status = Just InProgress, completedToday = False }` |
| fromUrl: `?completed_today=true` | `Workflows { status = Nothing, completedToday = True }` |
| fromUrl: `?status=invalid` | 無効値は無視 |
| toString: フィルタなし | `"/workflows"` |
| toString: status 付き | `"/workflows?status=in_progress"` |
| ラウンドトリップ | `fromUrl (toString route) == route` |
| isRouteActive | フィルタ無視で一致判定 |

## 設計解説

### 1. `Query.enum` vs `Query.custom` の使い分け

場所: [`Route.elm`](../../../../frontend/src/Route.elm) の `workflowQueryParser`

`Query.enum` は `Dict String a` から `Maybe a` を返す。status は列挙型であり absent/invalid 時に `Nothing` を返すのが自然なため `Query.enum` が適する。

一方 `completedToday` は `Bool` であり、absent は `False` として扱いたい。`Query.enum` だと `Maybe Bool` を返し、`Nothing` と `Just False` の区別が必要になる。`Query.custom` はキーの全値リスト（`List String`）を受け取るため、absent → `[]` → `False` を直接表現できる。

### 2. Route を URL 状態の Single Source of Truth にする

場所: [`Route.elm`](../../../../frontend/src/Route.elm) の `type Route`

代替案: フィルタ状態をページモデルだけに持ち、Route は `Workflows` のまま

採用理由: Route にフィルタを埋め込むことで、URL とアプリケーション状態の乖離を型レベルで防ぐ。URL を直接入力した場合もフィルタが正しく復元される。ブックマークや URL 共有にも対応。

### 3. `Url.Builder` による URL 生成

場所: [`Route.elm`](../../../../frontend/src/Route.elm) の `toString`

代替案: 手動の文字列結合（`"/workflows?" ++ String.join "&" parts`）

採用理由: `Url.Builder.absolute` は URL エンコーディングを自動処理する。特殊文字を含むパラメータ値のエスケープ漏れを防ぐ。他のルート（パスのみ）は既存の文字列結合のままで問題ないが、クエリパラメータを扱う `Workflows` のみ `Url.Builder` を使用。
