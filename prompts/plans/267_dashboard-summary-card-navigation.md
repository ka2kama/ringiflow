# Issue #267: ダッシュボードのサマリーカードをクリックで一覧ページに遷移可能にする

## Context

ダッシュボードの KPI カード（承認待ちタスク、申請中、本日完了）は現在、静的な数値表示のみ。件数をクリックして対応するフィルタ付き一覧ページに直接遷移できるようにする。

前提: フロントエンド（Elm）のみの変更。バックエンド変更なし。

## 設計判断

### 1. Route 型: `Workflows` のみにフィルタを埋め込む（`Tasks` は変更しない）

```elm
type Route
    = Home
    | Workflows WorkflowFilter  -- 変更: クエリパラメータを保持
    | WorkflowNew
    | WorkflowDetail Int
    | Tasks                     -- 変更なし
    | TaskDetail Int Int
    | NotFound
```

`Tasks` を変更しない理由: バックエンド Task API (`GET /api/v1/tasks/my`) は Active なタスクのみを返す（`TaskUseCaseImpl::list_my_tasks` で `StepActive` にフィルタ済み）。クライアント側のステータスフィルタは全アイテムが同一ステータスのため無意味。

`Workflows` にフィルタを埋め込む理由: Route を URL 状態の Single Source of Truth にすることで、URL とフィルタの乖離を型レベルで防ぐ。

### 2. クエリパラメータのパース: `Url.Parser.Query` + `<?>` コンビネータ

初版では手動 `url.query` パースを提案したが、再検討の結果、Elm 標準ライブラリ `Url.Parser.Query` を採用する。

```elm
parser =
    oneOf
        [ Parser.map Home top
        , Parser.map WorkflowNew (s "workflows" </> s "new")
        , Parser.map TaskDetail (s "workflows" </> int </> s "tasks" </> int)
        , Parser.map WorkflowDetail (s "workflows" </> int)
        , Parser.map Workflows (s "workflows" <?> workflowQueryParser)
        , Parser.map Tasks (s "tasks")
        ]
```

初版（手動パース）を却下した理由:
- `Url.Parser.Query` はまさにこの用途のために設計されたモジュール
- `Query.enum` で型安全なパースが宣言的に記述できる
- 手動の文字列分割・Dict 変換は車輪の再発明

`<?>` はパスマッチには影響しない（パスは `s "workflows"` のまま、クエリパラメータを追加でパース）。パーサー順序（`WorkflowNew` > `TaskDetail` > `WorkflowDetail` > `Workflows`）は現在と同じで問題なし。

### 3. URL クエリパラメータ形式: lowercase snake_case

Issue 仕様に合わせて lowercase snake_case を採用:
- `/workflows?status=in_progress`
- `/workflows?completed_today=true`

パース（URL → Route）は `Query.enum` で `Dict.fromList [("in_progress", InProgress), ...]` のマッピングを定義。
生成（Route → URL）は `Route.toString` 内の `statusToQueryValue` ヘルパーで変換。

### 4. フィルタ変更時の URL 同期: `Nav.replaceUrl` + 同一ページ判定

フィルタ dropdown 変更時:
1. ページの `update` で `Nav.replaceUrl` を呼ぶ（ページに `Nav.Key` を渡す）
2. `UrlChanged` が発火 → Main.elm で同一ページ判定
3. 同一ページなら `applyFilter` でフィルタのみ更新（データ再取得しない）
4. 異なるページなら通常の `initPage`

```elm
-- Main.elm の UrlChanged ハンドラ（抜粋）
( WorkflowsPage subModel, Route.Workflows newFilter ) ->
    let
        ( newSubModel, subCmd ) =
            WorkflowList.applyFilter newFilter subModel
    in
    -- 同一ページ: フィルタのみ更新、データ再取得しない
    ( { model | url = url, route = newRoute
      , page = WorkflowsPage newSubModel, sidebarOpen = False }
    , Cmd.map WorkflowsMsg subCmd
    )
```

`Nav.replaceUrl` を選択した理由: フィルタ変更ごとに履歴エントリを作ると、戻るボタンが「フィルタ変更の巻き戻し」になり UX が悪い。`replaceUrl` は現在の URL を置き換えるため履歴を汚さない。ダッシュボードカードのクリックは通常のリンク遷移（`pushUrl`）なので履歴に残る。

### 5. `completed_today` フィルタ: クライアントサイドの日付フィルタ

`WorkflowInstance.updatedAt` (ISO 8601 文字列) を今日の日付と比較し、`status == Approved` のワークフローを抽出。

実装方法:
1. `Iso8601.toTime`（`rtfeldman/elm-iso8601-date-strings` 1.1.4、直接依存として利用可能）で `updatedAt` を `Time.Posix` にパース
2. `Util.DateFormat.formatPosixDate` と同じパターン（`Time.toYear`/`Time.toMonth`/`Time.toDay` + `Shared.zone`）で日付部分を比較
3. 純粋関数として抽出: `isCompletedToday : Time.Zone -> Time.Posix -> WorkflowInstance -> Bool`
4. `init` で `Time.now` を取得し Model に `now : Maybe Time.Posix` として保持

タイムゾーン対応: UTC の `updatedAt` をユーザーのタイムゾーン（`Shared.zone`）で日付変換するため、日付境界の UTC ↔ ローカルのずれは正しく処理される。

既知の制約: ダッシュボードの `completedToday` カウント（サーバー算出）とクライアントフィルタ結果が異なる可能性あり。サーバー側のフィルタ条件（含まれるステータス、時刻の丸め方）が不明なため。MVP として許容し、将来的にはサーバーサイドフィルタ API パラメータの追加で解決する

### 6. 「承認待ちタスク」カードの遷移先

`/tasks`（クエリパラメータなし）にリンク。
API がすでに Active タスクのみ返すため、フィルタ不要。Issue 原文の `/tasks?status=pending` からの変更点。

### 7. `completedToday` と `status` の優先順位: `completedToday` 優先

`completedToday=true` は「本日完了」のプリセットフィルタであり、`status` フィルタとは独立した概念。

優先順位ルール: **`completedToday=True` のとき `status` フィルタを無視する。**

```elm
filterWorkflows : Time.Zone -> Maybe Time.Posix -> WorkflowFilter -> List WorkflowInstance -> List WorkflowInstance
filterWorkflows zone maybeNow filter workflows =
    if filter.completedToday then
        filterByCompletedToday zone maybeNow workflows
    else
        filterByStatus filter.status workflows
```

理由:
- `completedToday` は `status == Approved && updatedAt == today` を暗黙に含むプリセット
- AND 結合にすると `/workflows?status=draft&completed_today=true` で空集合になり混乱を招く
- プリセット優先は UI の「本日完了のみ [×]」バッジと自然に対応する

検討した代替案:
- AND 結合（status && completedToday）: 技術的には正しいが、`status=draft&completed_today=true` の空結果が非直感的
- status 無視ではなく status=Approved に強制: Route の `status` フィールドと実際の挙動が乖離する

### 8. `completedToday` 有効時のフィルタ UI

ダッシュボードの「本日完了」カードから遷移した場合、ユーザーがどのフィルタが適用されているか明確にわかる必要がある。

UI 構成:
1. **バッジ表示**: フィルタ行に "本日完了のみ" バッジ + "×" 解除ボタンを表示
2. **ドロップダウン**: "全て" を表示（`status = Nothing` のため）。変更可能
3. **ドロップダウン変更時**: `completedToday` を自動クリア → `/workflows?status=xxx` に遷移
4. **"×" クリック時**: `completedToday` のみクリア → 現在の status を維持して遷移

ドロップダウン変更で `completedToday` をクリアする理由:
- `completedToday` がステータスを上書きするため、ドロップダウン変更が無効果になるのは非直感的
- 明示的なフィルタ選択は「プリセットからの離脱」として扱う

実装:
```elm
-- ドロップダウン変更: completedToday を常にクリア
SetStatusFilter maybeStatus ->
    ( model
    , Nav.replaceUrl model.key
        (Route.toString (Route.Workflows { status = maybeStatus, completedToday = False }))
    )

-- "×" ボタン: completedToday のみクリア、status は維持
ClearCompletedToday ->
    ( model
    , Nav.replaceUrl model.key
        (Route.toString (Route.Workflows { status = model.statusFilter, completedToday = False }))
    )
```

### 9. `toString` のクエリ生成: `Url.Builder` モジュール活用

手動の文字列結合ではなく、Elm 標準ライブラリの `Url.Builder` を使用する。

```elm
import Url.Builder as Builder

-- Workflows の toString
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

`Url.Builder` を選択した理由:
- URL エンコーディングを自動処理（特殊文字のエスケープ）
- `Builder.absolute` がパスとクエリを正しく結合
- 手動の `"?" ++ String.join "&" parts` は車輪の再発明

他のルートは単純なパスのみのため、既存の文字列結合のままで良い。`Workflows` のみ `Url.Builder` を使用。

### 10. `applyFilter` の返り値: `( Model, Cmd Msg )`

`applyFilter` は Main.elm の同一ページ判定から呼ばれる。`completedToday` が `True` に変わった場合、フレッシュな現在時刻が必要。

```elm
applyFilter : WorkflowFilter -> Model -> ( Model, Cmd Msg )
applyFilter filter model =
    let
        newModel =
            { model | statusFilter = filter.status, completedToday = filter.completedToday }

        cmd =
            if filter.completedToday then
                Task.perform GotCurrentTime Time.now
            else
                Cmd.none
    in
    ( newModel, cmd )
```

純粋関数 `Model -> Model` ではなく `( Model, Cmd Msg )` にした理由:
- `/workflows` に滞在中に URL を `/workflows?completed_today=true` に変更されるフローがある
- この場合 `model.now` が `Nothing` または stale（ページ読み込み時の時刻）
- `completedToday=True` に遷移するたびに `Time.now` を取得することで、常にフレッシュな日付比較が可能
- `Time.now` は即座に完了するため、UX への影響はゼロ

## 対象ファイル一覧

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Route.elm` | `WorkflowFilter` 型定義、`Workflows WorkflowFilter` への変更、`<?>` + `Query.enum` によるパース、`toString` のクエリ生成 |
| `frontend/src/Main.elm` | `initPage` 引数変更（`Nav.Key` 追加）、`UrlChanged` に同一ページ判定、`pageTitle`・`viewSidebar` 更新 |
| `frontend/src/Page/Workflow/List.elm` | `init` にフィルタ/Nav.Key、`applyFilter` 公開、URL 同期、`completedToday` フィルタ |
| `frontend/src/Page/Home.elm` | カードを `<a>` 要素に変更、フィルタ付き URL 設定 |
| `frontend/tests/RouteTest.elm` | クエリパラメータ付きテスト追加、既存テスト更新 |

## 変更不要なファイル（理由付き）

| ファイル | 変更しない理由 |
|---------|--------------|
| `frontend/src/Page/Task/List.elm` | API が Active タスクのみ返すため、クライアント側フィルタ不要 |
| `frontend/src/Data/WorkflowInstance.elm` | URL 用の変換は Route.elm 内部ヘルパーとして実装。ドメイン型に URL 表現の責務を持たせない |

## 対象外

- バックエンド API 変更
- サーバーサイドフィルタリング（`completed_today` を含む将来課題）
- ページネーション
- タスク一覧のフィルタ UI（API が Active のみ返すため不要）

## Phase 0: 設計ブラッシュアップフローの形式知化

Issue #267 の設計過程で確立したブラッシュアップループを、今後の Issue でも再現可能な形式知に落とし込む。

### 変更内容

**`.claude/rules/zoom-rhythm.md`:**

「設計段階でも同じリズムを適用する」セクション（現在の4ステップ）を、具体的な「設計ブラッシュアップループ」手法で拡張する:

1. 設計ブラッシュアップループの具体プロセス:
   ```
   1. 初版を書く
   2. ギャップを体系的に洗い出す（未定義、未決定、矛盾、暗黙の前提）
   3. 各ギャップを調査・解決する
   4. 計画を更新する
   5. ループ記録に追記する
   6. 2 に戻る（ギャップゼロになるまで）
   ```

2. ギャップ発見の観点（ループ内で使うチェック）:
   - 参照されているが定義されていないもの（未定義）
   - 複数の解釈が可能なもの（曖昧）
   - 組み合わせの挙動が未定義のもの（競合・エッジケース）
   - UI の状態遷移が不完全なもの（状態漏れ）
   - 純粋関数 vs 副作用の境界が曖昧なもの（アーキテクチャ矛盾）
   - 標準ライブラリに代替手段があるもの（車輪の再発明）

3. 完了基準: 「この設計が現時点での最適解だ」と胸を張れるレベル。ギャップが見つからなくなるまで回す。ループ数に上限なし

4. 計画ファイルの必須要素にブラッシュアップループの記録を追加:
   ```markdown
   ### ブラッシュアップループの記録

   | ループ | きっかけ | 調査内容 | 結果 |
   |-------|---------|---------|------|
   | 1回目 | ... | ... | ... |
   ```

**`prompts/improvements/2026-02/2026-02-06_HHMM_設計ブラッシュアップループの形式知化.md`:**
- 改善記録: なぜこの形式知化が必要だったか（Issue #267 の設計過程の振り返り）

### テストリスト

ドキュメント変更のため、テスト不要。

## Phase 1: URL クエリパラメータ対応（Route 層）

### 変更内容

**Route.elm:**

1. フィルタ型を定義:
```elm
type alias WorkflowFilter =
    { status : Maybe Status
    , completedToday : Bool
    }

emptyWorkflowFilter : WorkflowFilter
emptyWorkflowFilter =
    { status = Nothing, completedToday = False }
```

2. Route 型を変更: `Workflows` → `Workflows WorkflowFilter`

3. パーサーで `<?>` + `Query.enum` / `Query.custom` を使用:
```elm
import Url.Parser.Query as Query

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
                [ "true" ] ->
                    True

                _ ->
                    False
        )
```

`completedTodayParser` の挙動:
- キー absent → `[]` → `False`
- `?completed_today=true` → `["true"]` → `True`
- `?completed_today=false` → `["false"]` → `False`
- `?completed_today=invalid` → `["invalid"]` → `False`

`Query.enum` を使わない理由: `Query.enum` は `Maybe a` を返す（absent/invalid → `Nothing`）。`completedToday` は `Bool` であり、absent は `False` として扱いたい。`Query.custom` はキーの全値リストを受け取るため、absent → `[]` → `False` を自然に表現できる。

4. `toString` でフィルタからクエリ文字列を生成（`Url.Builder` + 内部ヘルパー `statusToQueryValue`）

5. `isRouteActive` でフィルタを無視して比較

**Main.elm:**
- `initPage` の `Route.Workflows` / `Route.Tasks` パターン更新
- `pageTitle` の `Route.Workflows _` パターン更新
- `viewSidebar` で `Route.Workflows Route.emptyWorkflowFilter` を使用

**Page/Home.elm:**
- `viewQuickActions` の `Route.Workflows` → `Route.Workflows Route.emptyWorkflowFilter`

**RouteTest.elm:**
- `parseUrl` ヘルパーに `parseUrlWithQuery : String -> String -> Route` を追加
- 既存テストを新しい Route コンストラクタに更新

### テストリスト

`RouteTest.elm` — fromUrl:
- [ ] `/workflows` (クエリなし) → `Workflows emptyWorkflowFilter`
- [ ] `/workflows?status=in_progress` → `Workflows { status = Just InProgress, completedToday = False }`
- [ ] `/workflows?status=draft` → `Workflows { status = Just Draft, completedToday = False }`
- [ ] `/workflows?completed_today=true` → `Workflows { status = Nothing, completedToday = True }`
- [ ] `/workflows?status=approved&completed_today=true` → 両方反映
- [ ] `/workflows?status=invalid` → 無効値は無視、`Workflows emptyWorkflowFilter`
- [ ] `/tasks` → `Tasks`（変更なし）

`RouteTest.elm` — toString:
- [ ] `toString (Workflows emptyWorkflowFilter)` → `"/workflows"`
- [ ] `toString (Workflows { status = Just InProgress, completedToday = False })` → `"/workflows?status=in_progress"`
- [ ] `toString (Workflows { status = Nothing, completedToday = True })` → `"/workflows?completed_today=true"`
- [ ] `toString (Workflows { status = Just Approved, completedToday = True })` → 両パラメータ含む

`RouteTest.elm` — ラウンドトリップ・その他:
- [ ] ラウンドトリップ: `fromUrl (toString route) == route` 代表的な組み合わせ
- [ ] `isRouteActive (Workflows anyFilter) (Workflows otherFilter)` → True（フィルタ無視）
- [ ] 既存テスト（Home, WorkflowNew, WorkflowDetail, Tasks, TaskDetail, NotFound）が引き続き pass

## Phase 2: 申請一覧のフィルタ連動

### 変更内容

**Page/Workflow/List.elm:**
1. `init` シグネチャ変更: `Shared -> Nav.Key -> WorkflowFilter -> ( Model, Cmd Msg )`
2. Model に追加: `key : Nav.Key`、`completedToday : Bool`、`now : Maybe Time.Posix`
3. `SetStatusFilter` で `Nav.replaceUrl` を呼び、URL と同期。`completedToday` を同時にクリア（設計判断 #8 参照）
4. `completedToday` フィルタロジック: `completedToday` 優先（設計判断 #7 参照）
5. `completedToday == True` なら `init` で `Time.now` を取得
6. `applyFilter : WorkflowFilter -> Model -> ( Model, Cmd Msg )` を公開（設計判断 #10 参照）
7. `ClearCompletedToday` Msg 追加: "×" ボタンで `completedToday` を解除
8. `completedToday` 有効時に "本日完了のみ" バッジ + "×" ボタンを `viewStatusFilter` 内に表示
9. `GotCurrentTime Time.Posix` Msg 追加: `Time.now` の結果を `model.now` に格納

**Main.elm:**
1. `initPage` シグネチャ変更: `Nav.Key -> Route -> Shared -> ( Page, Cmd Msg )`
2. `UrlChanged` で同一ページ判定追加:
   - `(WorkflowsPage subModel, Route.Workflows newFilter)` → `applyFilter` のみ（Cmd を `WorkflowsMsg` でマップ）
   - それ以外 → 通常の `initPage`（データ再取得あり）
3. 全 `initPage` 呼び出し箇所を更新

```elm
-- Main.elm の UrlChanged ハンドラ: 同一ページ判定
UrlChanged url ->
    let
        newRoute = Route.fromUrl url
    in
    case ( model.page, newRoute ) of
        ( WorkflowsPage subModel, Route.Workflows newFilter ) ->
            let
                ( newSubModel, subCmd ) =
                    WorkflowList.applyFilter newFilter subModel
            in
            ( { model | url = url, route = newRoute
              , page = WorkflowsPage newSubModel, sidebarOpen = False }
            , Cmd.map WorkflowsMsg subCmd
            )

        _ ->
            let
                ( page, pageCmd ) = initPage model.key newRoute model.shared
            in
            ( { model | url = url, route = newRoute
              , page = page, sidebarOpen = False }
            , pageCmd
            )
```

### テストリスト

純粋関数のテスト（`Cmd` に依存しない部分）:

`Page/Workflow/List` のフィルタロジック:
- [ ] `completedToday` フィルタ: status が Approved かつ `updatedAt` が今日のワークフローのみ返す
- [ ] `completedToday` フィルタ: 昨日の Approved ワークフローは除外
- [ ] `completedToday` フィルタ: status が InProgress かつ `updatedAt` が今日のワークフローは除外（Approved 以外）
- [ ] `completedToday=True` 時に `status` フィルタが無視される（優先順位: 設計判断 #7）
- [ ] `completedToday=False` 時は通常の `status` フィルタが適用される
- [ ] `applyFilter` がフィルタ状態を正しく更新し、`completedToday=True` 時に `Time.now` Cmd を発行

## Phase 3: ダッシュボードカードのリンク化

### 変更内容

**Page/Home.elm:**
1. `viewStatCard` を `viewStatCardLink` に変更: `div` → `a` 要素（`href` 付き）
2. 各カードの遷移先:
   - 承認待ちタスク → `/tasks`（フィルタなし。API が Active のみ返すため）
   - 申請中 → `/workflows?status=in_progress`
   - 本日完了 → `/workflows?completed_today=true`
3. ホバーエフェクト（`hover:shadow-md`, `transition-shadow`）追加

### テストリスト

Route.toString のテストで URL の正しさは Phase 1 で保証済み。Phase 3 では:
- [ ] カードが `<a>` 要素としてレンダリングされる
- [ ] 承認待ちタスクカードの href が `/tasks`
- [ ] 申請中カードの href が `/workflows?status=in_progress`
- [ ] 本日完了カードの href が `/workflows?completed_today=true`

## 検証方法

```bash
# テスト実行
cd frontend && pnpm run test

# 開発サーバーで手動確認
just dev-all
```

手動確認シナリオ:
1. ダッシュボードの各カードをクリック → 対応するフィルタ付きページに遷移
2. URL バーのクエリパラメータが正しい
3. ブラウザの戻る → ダッシュボードに戻る
4. 申請一覧のフィルタ dropdown を変更 → URL が更新される（履歴は増えない）
5. フィルタ付き URL を直接入力 → フィルタが適用される
6. `/workflows?status=invalid` → フィルタなしの一覧表示（エラーにならない）
7. 「本日完了」カードクリック → "本日完了のみ" バッジが表示される
8. バッジの "×" クリック → バッジ消滅、全件表示に戻る
9. `completedToday` 有効時にドロップダウン変更 → バッジ消滅、選択したステータスでフィルタ

## 設計ブラッシュアップの経緯

| 版 | 変更内容 | 理由 |
|----|---------|------|
| 初版 | 手動 `url.query` パース | — |
| 改訂1 | `Url.Parser.Query` + `<?>` に変更 | 標準ライブラリの `Query.enum` で型安全なパースが宣言的に書ける。手動パースは車輪の再発明 |
| 初版 | `Tasks TaskFilter` で Route 型変更 | — |
| 改訂1 | `Tasks` は変更なし | バックエンド API が Active タスクのみ返すことを確認。クライアント側フィルタは無意味 |
| 初版 | `Data/WorkflowInstance.elm` に `stepStatusToString`/`stepStatusFromString` 追加 | — |
| 改訂1 | 不要（削除） | Task フィルタ削除に伴い URL 用の StepStatus 変換も不要 |
| 初版 | 「承認待ちタスク」→ `/tasks?status=active` | — |
| 改訂1 | 「承認待ちタスク」→ `/tasks` | API が Active のみ返すため、フィルタパラメータ自体が不要 |
| 改訂1 | `completedTodayParser` 未定義 | — |
| 改訂2 | `Query.custom` で具体定義 | `Query.enum` は `Maybe a` を返すため `Bool` に不適。`Query.custom` でキー absent → `[]` → `False` を自然に表現 |
| 改訂1 | `applyFilter : WorkflowFilter -> Model -> Model` | — |
| 改訂2 | `applyFilter : WorkflowFilter -> Model -> ( Model, Cmd Msg )` | 同一ページ遷移フローで `completedToday` が `True` に変わる際、フレッシュな `Time.now` が必要。純粋関数では Cmd を発行できない |
| 改訂1 | `completedToday` と `status` の優先順位未定義 | — |
| 改訂2 | `completedToday` 優先（`status` を無視） | AND 結合では `status=draft&completed_today=true` で空結果。プリセット優先が UI のバッジ表示と自然に対応 |
| 改訂1 | `completedToday` 有効時の UI 未定義 | — |
| 改訂2 | バッジ + "×" ボタン + ドロップダウン連動 | ドロップダウン変更で `completedToday` をクリア（プリセット離脱）。"×" で `completedToday` のみクリア |
| 改訂1 | `toString` 手動文字列結合 | — |
| 改訂2 | `Url.Builder.absolute` 使用 | URL エンコーディング自動処理。標準ライブラリの正しい使い方 |

## 収束確認（設計・計画）

### ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → 「本当に最適か？」 | `Url.Parser.Query` の API を elm/url 1.0.0 で検証。Task API の実装を確認 | `<?>` + `Query.enum` に変更。Task フィルタ削除（API が Active のみ返すため不要） |
| 2回目 | 改訂後 → 残りの不確実性を洗い出し | `completed_today` の日付比較方法を検証（`Iso8601.toTime` の利用可能性、タイムゾーン処理）。`Nav.replaceUrl` の `onUrlChange` 発火挙動。`Query.enum` の absent/invalid 時の挙動 | `Iso8601.toTime` + `formatPosixDate` パターンで実装確定。同一ページ判定パターン確定。エッジケース（empty value、case sensitivity、multiple values）を検証 |
| 3回目 | 改訂2 → 未定義・未決定の全箇所を洗い出し | (1) `completedTodayParser` 未定義 → `Query.custom` で実装確定 (2) `applyFilter` + `Time.now` 矛盾 → 同一ページ遷移フロー分析で `(Model, Cmd Msg)` に変更 (3) `completedToday` と `status` の競合 → 優先順位ルール確定 (4) `completedToday` UI 未定義 → バッジ + ドロップダウン連動仕様確定 (5) `toString` → `Url.Builder` 活用確定 (6) `SetStatusFilter` の `completedToday` クリア挙動 → プリセット解除パターン確定 | 6 ギャップを発見・全解決。設計判断 #7〜#10 追加 |

### チェックリスト

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Route.elm, Main.elm, Page/Workflow/List.elm, Page/Home.elm, RouteTest.elm を対象に含む。Page/Task/List.elm（API が Active のみ返す）と Data/WorkflowInstance.elm（URL 表現は Route の責務）は「変更不要なファイル」に技術的根拠付きで記載 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase をコード構造・関数シグネチャで記述。`completedTodayParser` を `Query.custom` で具体定義（ループ3）。`applyFilter` の返り値を `(Model, Cmd Msg)` に明示（ループ3）。`completedToday` と `status` の優先順位ルールを設計判断 #7 で明示（ループ3）。フィルタ UI の具体的挙動を設計判断 #8 で明示（ループ3）。既知の制約も明記 |
| 3 | 設計判断の完結性 | 全ての差異・バリエーションに判断が記載されている | OK | 10 判断すべてに選択肢・理由・トレードオフを記載。ブラッシュアップの経緯で却下理由を記録。Issue 原文との差異（`/tasks?status=pending` → `/tasks`）の根拠も明記。フィルタ間の優先順位、UI 連動、`Url.Builder` 選択もすべて判断済み |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象（Workflow フィルタ、URL クエリパラメータ、`completedToday` UI）と対象外（バックエンド変更、Task フィルタ UI、ページネーション）を明記。対象外にした理由を技術調査結果に基づき記載 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | (1) `Nav.replaceUrl` → `onUrlChange` 発火: 同一ページ判定で対処 (2) `Query.enum`: absent → `Nothing`、invalid → `Nothing`、case-sensitive を確認 (3) Task API: Active のみ返すことをバックエンド実装で確認 (4) `<?>`: パスマッチに影響しないことを確認 (5) `Iso8601.toTime`: elm.json で直接依存として利用可能を確認 (6) `Query.custom`: `completed_today` absent → `[]` → `False`、`["true"]` → `True` の挙動を確認 (7) `applyFilter` 呼び出しフロー: 3パターン（init, 同一ページ, ドロップダウン変更）を分析し、全パターンで `Time.now` が取得可能であることを確認（ループ3） (8) `Url.Builder.absolute`: パス + クエリパラメータの正しい結合を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Route.elm の設計方針コメント、TEA パターン、Workflow List の既存フィルタパターンに準拠。`Util.DateFormat` の `Iso8601.toTime` + `formatPosixDate` パターンを再利用。Issue #267 の完了基準 4 項目を Phase 1-3 で充足 |
