# Phase 2: 申請一覧のフィルタ連動

## 対応 Issue

[#267](https://github.com/ka2kama/ringiflow/issues/267)

## 概要

申請一覧ページに URL クエリパラメータとの双方向フィルタ同期を実装。`completedToday` クライアントサイドフィルタとフィルタ UI を追加。

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`frontend/src/Page/Workflow/List.elm`](../../../../frontend/src/Page/Workflow/List.elm) | フィルタロジック、URL 同期、フィルタ UI |
| [`frontend/src/Main.elm`](../../../../frontend/src/Main.elm) | 同一ページ判定 |
| [`frontend/tests/Page/Workflow/ListFilterTest.elm`](../../../../frontend/tests/Page/Workflow/ListFilterTest.elm) | フィルタロジックのテスト |

## 実装内容

### フィルタロジック

```elm
filterWorkflows : Time.Zone -> Maybe Time.Posix -> WorkflowFilter -> List WorkflowInstance -> List WorkflowInstance
filterWorkflows zone maybeNow filter workflows =
    if filter.completedToday then
        -- completedToday 優先: status フィルタを無視
        case maybeNow of
            Just now -> List.filter (isCompletedToday zone now) workflows
            Nothing -> workflows  -- Time.now 未取得時はフィルタ不能
    else
        case filter.status of
            Nothing -> workflows
            Just status -> List.filter (\w -> w.status == status) workflows

isCompletedToday : Time.Zone -> Time.Posix -> WorkflowInstance -> Bool
isCompletedToday zone now workflow =
    (workflow.status == Approved) && (updatedAt が今日の日付)
```

### URL 同期パターン

```elm
-- フィルタ変更 → URL 更新（履歴を汚さない）
SetStatusFilter maybeStatus ->
    ( model
    , Nav.replaceUrl model.key
        (Route.toString (Route.Workflows { status = maybeStatus, completedToday = False }))
    )
```

### 同一ページ判定（Main.elm）

```elm
UrlChanged url ->
    let newRoute = Route.fromUrl url in
    case ( model.page, newRoute ) of
        ( WorkflowsPage subModel, Route.Workflows newFilter ) ->
            -- 同一ページ: フィルタのみ更新、データ再取得しない
            let ( newSubModel, subCmd ) = WorkflowList.applyFilter newFilter subModel in
            ( { model | page = WorkflowsPage newSubModel, ... }, Cmd.map WorkflowsMsg subCmd )
        _ ->
            -- 異なるページ: 通常の初期化
            ...
```

### フィルタ UI

`completedToday` 有効時:
- "本日完了のみ" バッジ + "×" 解除ボタンを表示
- ドロップダウン変更で `completedToday` を自動クリア（プリセット離脱）

## テスト

| テストケース | 検証内容 |
|-------------|---------|
| isCompletedToday: Approved + 今日 | `True` |
| isCompletedToday: Approved + 昨日 | `False` |
| isCompletedToday: InProgress + 今日 | `False`（Approved 以外） |
| filterWorkflows: completedToday=True | 今日の Approved のみ |
| filterWorkflows: completedToday 優先 | status を無視 |
| filterWorkflows: status フィルタ | completedToday=False 時に適用 |
| filterWorkflows: now=Nothing | フィルタ不能で全件返す |

## 設計解説

### 1. `completedToday` 優先（プリセットフィルタパターン）

場所: [`List.elm`](../../../../frontend/src/Page/Workflow/List.elm) の `filterWorkflows`

`completedToday=True` は「status=Approved かつ updatedAt=今日」を暗黙に含むプリセットフィルタ。AND 結合にすると `status=draft&completed_today=true` で空結果になり混乱を招く。プリセット優先は UI のバッジ表示と自然に対応する。

### 2. `applyFilter` が `(Model, Cmd Msg)` を返す理由

場所: [`List.elm`](../../../../frontend/src/Page/Workflow/List.elm) の `applyFilter`

同一ページ遷移フロー（`/workflows` → `/workflows?completed_today=true`）で `completedToday` が `True` に変わる際、日付比較にフレッシュな `Time.now` が必要。純粋関数 `Model -> Model` では `Cmd` を発行できないため、`(Model, Cmd Msg)` を返す設計にした。`Time.now` は即座に完了するため UX への影響はゼロ。

### 3. `Nav.replaceUrl` で履歴を汚さない

場所: [`List.elm`](../../../../frontend/src/Page/Workflow/List.elm) の `SetStatusFilter`

代替案: `Nav.pushUrl`（履歴エントリを追加）

フィルタ変更ごとに履歴エントリを作ると、戻るボタンが「フィルタ変更の巻き戻し」になり UX が悪い。`replaceUrl` は現在の URL を置き換えるため履歴を汚さない。ダッシュボードカードのクリックは通常のリンク遷移（`<a>` 要素）なので `pushUrl` 相当で履歴に残る。

### 4. 同一ページ判定パターン

場所: [`Main.elm`](../../../../frontend/src/Main.elm) の `UrlChanged`

`Nav.replaceUrl` は `onUrlChange` を発火する。これにより `UrlChanged` → `initPage` → API 再取得が発生してしまう。`(model.page, newRoute)` のパターンマッチで「現在のページが WorkflowsPage かつ新しいルートも Workflows」のケースを検出し、`applyFilter` でフィルタ状態のみ更新する。

この手法は Elm 公式ドキュメントでも推奨される「同一ページでの URL 変更検出」パターン。
