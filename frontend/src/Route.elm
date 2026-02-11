module Route exposing (Route(..), WorkflowFilter, emptyWorkflowFilter, fromUrl, isRouteActive, pageTitle, toString)

{-| URL ルーティングモジュール

このモジュールは URL とアプリケーションの画面（Route）を
相互変換する責務を持つ。


## 設計方針

ルーティングを独立したモジュールに分離することで:

1.  **関心の分離**: Main.elm はルーティングの詳細を知らなくてよい
2.  **テスタビリティ**: ルーティングロジックを単独でテスト可能
3.  **拡張性**: 新しいルートの追加が容易


## Elm のルーティングアプローチ

Elm では URL パーサーを宣言的に定義する。
これは React Router や Vue Router とは異なるアプローチ:

  - **React/Vue**: コンポーネントベース（JSX 内でルート定義）
  - **Elm**: データ指向（パーサーコンビネータで定義）


## URL 解析の流れ

```text
URL 文字列
    ↓ (Browser がパース)
Url 型
    ↓ (Route.fromUrl)
Route 型
    ↓ (Main.view で分岐)
対応する画面
```


## 代替案と不採用理由

  - **elm-spa**: ルーティングを自動生成するが、
    学習目的で手動実装を選択
  - **Hash ベースルーティング**: SEO に不利、
    モダンな SPA では History API を使用するのが主流

-}

import Data.WorkflowInstance exposing (Status(..))
import Dict
import Url exposing (Url)
import Url.Builder as Builder
import Url.Parser as Parser exposing ((</>), (<?>), Parser, int, oneOf, s, string, top)
import Url.Parser.Query as Query


{-| アプリケーションのルート（画面）を表す型


## 設計意図

Route はカスタム型（Tagged Union / Sum Type）として定義。
これにより:

1.  **型安全**: 存在しないルートを参照できない
2.  **網羅性チェック**: case 式で全ルートを処理しないとコンパイルエラー
3.  **IDE サポート**: 補完やリファクタリングが容易


## 現在のルート

  - `Home`: トップページ（`/`）
  - `Workflows`: 申請一覧（`/workflows`）
  - `WorkflowNew`: 新規申請（`/workflows/new`）
  - `WorkflowDetail`: 申請詳細（`/workflows/{displayNumber}`）
  - `Tasks`: タスク一覧（`/tasks`）
  - `TaskDetail`: タスク詳細（`/workflows/{workflowDisplayNumber}/tasks/{stepDisplayNumber}`）
  - `NotFound`: 存在しないパス

-}
type Route
    = Home
    | Workflows WorkflowFilter
    | WorkflowNew
    | WorkflowDetail Int
    | Tasks
    | TaskDetail Int Int
    | Users
    | UserDetail Int
    | UserNew
    | UserEdit Int
    | Roles
    | RoleNew
    | RoleEdit String
    | AuditLogs
    | NotFound


{-| 申請一覧のフィルタ条件

URL クエリパラメータと対応する。
例: `/workflows?status=in_progress&completed_today=true`

-}
type alias WorkflowFilter =
    { status : Maybe Status
    , completedToday : Bool
    }


{-| フィルタなし（デフォルト状態）
-}
emptyWorkflowFilter : WorkflowFilter
emptyWorkflowFilter =
    { status = Nothing, completedToday = False }


{-| URL パーサー


## パーサーコンビネータについて

Elm の Url.Parser は「パーサーコンビネータ」パターンを使用。
小さなパーサーを組み合わせて複雑なパーサーを構築する。


## 主要なコンビネータ

  - `top`: ルートパス（`/`）にマッチ
  - `s "path"`: 固定文字列にマッチ
  - `</>`: パーサーを連結
  - `oneOf`: 複数のパーサーを試行
  - `int`: 整数にマッチしてキャプチャ
  - `string`: 文字列にマッチしてキャプチャ


## 例: 複数ルートの定義

    parser =
        oneOf
            [ Parser.map Home top
            , Parser.map Workflows (s "workflows")
            , Parser.map WorkflowDetail (s "workflows" </> int)
            , Parser.map Settings (s "settings")
            ]


## 型シグネチャの解説

`Parser (Route -> a) a` は「Route を受け取って a を返す関数」を
「a」に変換するパーサー。これは関数合成のためのトリックで、
複数のパーサーを `oneOf` で合成可能にしている。

-}
parser : Parser (Route -> a) a
parser =
    oneOf
        [ Parser.map Home top
        , Parser.map WorkflowNew (s "workflows" </> s "new")
        , Parser.map TaskDetail (s "workflows" </> int </> s "tasks" </> int)
        , Parser.map WorkflowDetail (s "workflows" </> int)
        , Parser.map Workflows (s "workflows" <?> workflowQueryParser)
        , Parser.map Tasks (s "tasks")
        , Parser.map UserNew (s "users" </> s "new")
        , Parser.map UserEdit (s "users" </> int </> s "edit")
        , Parser.map UserDetail (s "users" </> int)
        , Parser.map Users (s "users")
        , Parser.map RoleNew (s "roles" </> s "new")
        , Parser.map RoleEdit (s "roles" </> string </> s "edit")
        , Parser.map Roles (s "roles")
        , Parser.map AuditLogs (s "audit-logs")
        ]


{-| 申請一覧のクエリパラメータパーサー

`status` と `completed_today` をパースする。

-}
workflowQueryParser : Query.Parser WorkflowFilter
workflowQueryParser =
    Query.map2 WorkflowFilter
        (Query.enum "status"
            (Dict.fromList
                [ ( "draft", Draft )
                , ( "pending", Pending )
                , ( "in_progress", InProgress )
                , ( "approved", Approved )
                , ( "rejected", Rejected )
                , ( "cancelled", Cancelled )
                ]
            )
        )
        completedTodayParser


{-| `completed_today` クエリパラメータのパーサー

  - キー不在 → `False`
  - `?completed_today=true` → `True`
  - その他の値 → `False`

`Query.enum` は `Maybe a` を返すため `Bool` に不適。
`Query.custom` でキーの全値リストを受け取り、`["true"]` のみ `True` にする。

-}
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


{-| URL を Route に変換


## 処理フロー

1.  `Parser.parse` で URL をパース
2.  成功すれば `Just Route` を返す
3.  失敗すれば `Nothing` を返す
4.  `Maybe.withDefault` で失敗時は `NotFound` に変換


## なぜ Maybe を使うか

URL パースは失敗する可能性がある（ユーザーが直接 URL を入力など）。
Elm は例外を使わず、Maybe 型で失敗を表現する。

-}
fromUrl : Url -> Route
fromUrl url =
    Parser.parse parser url
        |> Maybe.withDefault NotFound


{-| Route を URL 文字列に変換


## 用途

  - リンクの生成
  - プログラム的なナビゲーション
  - ブックマーク URL の生成


## 双方向性

`fromUrl` と `toString` は互いに逆変換の関係にある。
これにより、URL とルートの一貫性が保証される:

    -- この等式が常に成り立つべき
    fromUrl (parseUrl (toString route)) == route

-}
toString : Route -> String
toString route =
    case route of
        Home ->
            "/"

        Workflows filter ->
            Builder.absolute [ "workflows" ]
                (List.filterMap identity
                    [ filter.status
                        |> Maybe.map (\st -> Builder.string "status" (statusToQueryValue st))
                    , if filter.completedToday then
                        Just (Builder.string "completed_today" "true")

                      else
                        Nothing
                    ]
                )

        WorkflowNew ->
            "/workflows/new"

        WorkflowDetail displayNumber ->
            "/workflows/" ++ String.fromInt displayNumber

        Tasks ->
            "/tasks"

        TaskDetail workflowDisplayNumber stepDisplayNumber ->
            "/workflows/" ++ String.fromInt workflowDisplayNumber ++ "/tasks/" ++ String.fromInt stepDisplayNumber

        Users ->
            "/users"

        UserDetail displayNumber ->
            "/users/" ++ String.fromInt displayNumber

        UserNew ->
            "/users/new"

        UserEdit displayNumber ->
            "/users/" ++ String.fromInt displayNumber ++ "/edit"

        Roles ->
            "/roles"

        RoleNew ->
            "/roles/new"

        RoleEdit roleId ->
            "/roles/" ++ roleId ++ "/edit"

        AuditLogs ->
            "/audit-logs"

        NotFound ->
            "/not-found"


{-| ワークフローステータスをクエリパラメータ値に変換
-}
statusToQueryValue : Status -> String
statusToQueryValue status =
    case status of
        Draft ->
            "draft"

        Pending ->
            "pending"

        InProgress ->
            "in_progress"

        Approved ->
            "approved"

        Rejected ->
            "rejected"

        Cancelled ->
            "cancelled"


{-| 現在のルートがナビゲーション項目に対応するかを判定

子ルートの場合、親ルートもアクティブとして扱う:

  - `WorkflowNew`, `WorkflowDetail _` → `Workflows` がアクティブ
  - `TaskDetail _` → `Tasks` がアクティブ

-}
isRouteActive : Route -> Route -> Bool
isRouteActive navRoute currentRoute =
    case ( navRoute, currentRoute ) of
        ( Home, Home ) ->
            True

        ( Workflows _, Workflows _ ) ->
            True

        ( Workflows _, WorkflowNew ) ->
            True

        ( Workflows _, WorkflowDetail _ ) ->
            True

        ( Tasks, Tasks ) ->
            True

        ( Tasks, TaskDetail _ _ ) ->
            True

        ( Users, Users ) ->
            True

        ( Users, UserDetail _ ) ->
            True

        ( Users, UserNew ) ->
            True

        ( Users, UserEdit _ ) ->
            True

        ( Roles, Roles ) ->
            True

        ( Roles, RoleNew ) ->
            True

        ( Roles, RoleEdit _ ) ->
            True

        ( AuditLogs, AuditLogs ) ->
            True

        _ ->
            False


{-| ルートに対応するページタイトル
-}
pageTitle : Route -> String
pageTitle route =
    case route of
        Home ->
            "ダッシュボード"

        Workflows _ ->
            "申請一覧"

        WorkflowNew ->
            "新規申請"

        WorkflowDetail _ ->
            "申請詳細"

        Tasks ->
            "タスク一覧"

        TaskDetail _ _ ->
            "タスク詳細"

        Users ->
            "ユーザー管理"

        UserDetail _ ->
            "ユーザー詳細"

        UserNew ->
            "ユーザー作成"

        UserEdit _ ->
            "ユーザー編集"

        Roles ->
            "ロール管理"

        RoleNew ->
            "ロール作成"

        RoleEdit _ ->
            "ロール編集"

        AuditLogs ->
            "監査ログ"

        NotFound ->
            "ページが見つかりません"
