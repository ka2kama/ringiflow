module Main exposing (main)

{-| RingiFlow メインモジュール

TEA (The Elm Architecture) に基づく SPA のエントリーポイント。
サイドバーナビゲーション付きのアプリシェルレイアウトを提供する。

詳細: [TEA パターン](../../../docs/06_ナレッジベース/elm/Elmアーキテクチャ.md)

-}

import Api exposing (ApiError)
import Api.Auth as AuthApi
import Browser
import Browser.Navigation as Nav
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick)
import Page.Home as Home
import Page.NotFound
import Page.Task.Detail as TaskDetail
import Page.Task.List as TaskList
import Page.Workflow.Detail as WorkflowDetail
import Page.Workflow.List as WorkflowList
import Page.Workflow.New as WorkflowNew
import Route exposing (Route)
import Shared exposing (Shared)
import Svg exposing (svg)
import Svg.Attributes as SvgAttr
import Url exposing (Url)



-- MAIN


{-| アプリケーションのエントリーポイント
-}
main : Program Flags Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        , onUrlChange = UrlChanged
        , onUrlRequest = LinkClicked
        }



-- FLAGS


{-| JavaScript から Elm に渡される初期化データ
-}
type alias Flags =
    { apiBaseUrl : String
    , timestamp : Int
    }



-- MODEL


{-| 現在のページ状態

Nested TEA パターンにより、各ページの Model を Page 型で保持する。
状態を持たないページ（Home, NotFound）は専用のコンストラクタを使用。

-}
type Page
    = HomePage Home.Model
    | WorkflowsPage WorkflowList.Model
    | WorkflowNewPage WorkflowNew.Model
    | WorkflowDetailPage WorkflowDetail.Model
    | TasksPage TaskList.Model
    | TaskDetailPage TaskDetail.Model
    | NotFoundPage


{-| アプリケーションの状態

グローバル状態（Shared）と現在のページ状態、サイドバーの開閉状態を保持する。

-}
type alias Model =
    { key : Nav.Key
    , url : Url
    , route : Route
    , shared : Shared
    , page : Page
    , sidebarOpen : Bool
    }


{-| アプリケーションの初期化

Shared を初期化し、初期ルートに対応するページを初期化する。
起動時に CSRF トークンを取得して Shared に設定する。

-}
init : Flags -> Url -> Nav.Key -> ( Model, Cmd Msg )
init flags url key =
    let
        route =
            Route.fromUrl url

        shared =
            Shared.init { apiBaseUrl = flags.apiBaseUrl }

        ( page, pageCmd ) =
            initPage route shared

        csrfCmd =
            fetchCsrfToken shared

        userCmd =
            fetchUser shared
    in
    ( { key = key
      , url = url
      , route = route
      , shared = shared
      , page = page
      , sidebarOpen = False
      }
    , Cmd.batch [ pageCmd, csrfCmd, userCmd ]
    )


{-| CSRF トークンを取得

セッションが存在しない場合は 401 が返されるが、無視する。
ログイン後に再度取得される。

-}
fetchCsrfToken : Shared -> Cmd Msg
fetchCsrfToken shared =
    AuthApi.getCsrfToken
        { config = Shared.toRequestConfig shared
        , toMsg = GotCsrfToken
        }


{-| ユーザー情報を取得

セッションが有効な場合、ユーザー情報を取得して Shared に設定する。
未認証の場合は 401 が返されるが、無視する。

-}
fetchUser : Shared -> Cmd Msg
fetchUser shared =
    AuthApi.getMe
        { config = Shared.toRequestConfig shared
        , toMsg = GotUser
        }


{-| ルートに応じたページを初期化
-}
initPage : Route -> Shared -> ( Page, Cmd Msg )
initPage route shared =
    case route of
        Route.Home ->
            let
                ( model, cmd ) =
                    Home.init shared
            in
            ( HomePage model, Cmd.map HomeMsg cmd )

        Route.Workflows ->
            let
                ( model, cmd ) =
                    WorkflowList.init shared
            in
            ( WorkflowsPage model, Cmd.map WorkflowsMsg cmd )

        Route.WorkflowNew ->
            let
                ( model, cmd ) =
                    WorkflowNew.init shared
            in
            ( WorkflowNewPage model, Cmd.map WorkflowNewMsg cmd )

        Route.WorkflowDetail id ->
            let
                ( model, cmd ) =
                    WorkflowDetail.init shared id
            in
            ( WorkflowDetailPage model, Cmd.map WorkflowDetailMsg cmd )

        Route.Tasks ->
            let
                ( model, cmd ) =
                    TaskList.init shared
            in
            ( TasksPage model, Cmd.map TasksMsg cmd )

        Route.TaskDetail id ->
            let
                ( model, cmd ) =
                    TaskDetail.init shared id
            in
            ( TaskDetailPage model, Cmd.map TaskDetailMsg cmd )

        Route.NotFound ->
            ( NotFoundPage, Cmd.none )


{-| ページの Shared を更新

CSRF トークン取得後など、グローバルな Shared が更新されたときに
各ページの Shared も同期する。

-}
updatePageShared : Shared -> Page -> Page
updatePageShared shared page =
    case page of
        HomePage subModel ->
            HomePage (Home.updateShared shared subModel)

        WorkflowsPage subModel ->
            WorkflowsPage (WorkflowList.updateShared shared subModel)

        WorkflowNewPage subModel ->
            WorkflowNewPage (WorkflowNew.updateShared shared subModel)

        WorkflowDetailPage subModel ->
            WorkflowDetailPage (WorkflowDetail.updateShared shared subModel)

        TasksPage subModel ->
            TasksPage (TaskList.updateShared shared subModel)

        TaskDetailPage subModel ->
            TaskDetailPage (TaskDetail.updateShared shared subModel)

        NotFoundPage ->
            NotFoundPage



-- UPDATE


{-| アプリケーションで発生するメッセージ

グローバルメッセージと、各ページのメッセージをラップした形式。

-}
type Msg
    = LinkClicked Browser.UrlRequest
    | UrlChanged Url
    | GotCsrfToken (Result ApiError String)
    | GotUser (Result ApiError Shared.User)
    | ToggleSidebar
    | CloseSidebar
    | HomeMsg Home.Msg
    | WorkflowsMsg WorkflowList.Msg
    | WorkflowNewMsg WorkflowNew.Msg
    | WorkflowDetailMsg WorkflowDetail.Msg
    | TasksMsg TaskList.Msg
    | TaskDetailMsg TaskDetail.Msg


{-| メッセージに基づいて Model を更新
-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        LinkClicked urlRequest ->
            case urlRequest of
                Browser.Internal url ->
                    ( model, Nav.pushUrl model.key (Url.toString url) )

                Browser.External href ->
                    ( model, Nav.load href )

        UrlChanged url ->
            let
                route =
                    Route.fromUrl url

                ( page, pageCmd ) =
                    initPage route model.shared
            in
            ( { model
                | url = url
                , route = route
                , page = page
                , sidebarOpen = False
              }
            , pageCmd
            )

        GotCsrfToken result ->
            case result of
                Ok token ->
                    let
                        newShared =
                            Shared.withCsrfToken token model.shared

                        newPage =
                            updatePageShared newShared model.page
                    in
                    ( { model | shared = newShared, page = newPage }
                    , Cmd.none
                    )

                Err _ ->
                    -- 未認証の場合は 401 が返されるが、無視する
                    -- ログイン後に再度取得される
                    ( model, Cmd.none )

        GotUser result ->
            case result of
                Ok user ->
                    let
                        newShared =
                            Shared.withUser user model.shared

                        newPage =
                            updatePageShared newShared model.page
                    in
                    ( { model | shared = newShared, page = newPage }
                    , Cmd.none
                    )

                Err _ ->
                    -- 未認証の場合は 401 が返されるが、無視する
                    ( model, Cmd.none )

        ToggleSidebar ->
            ( { model | sidebarOpen = not model.sidebarOpen }
            , Cmd.none
            )

        CloseSidebar ->
            ( { model | sidebarOpen = False }
            , Cmd.none
            )

        HomeMsg subMsg ->
            case model.page of
                HomePage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            Home.update subMsg subModel
                    in
                    ( { model | page = HomePage newSubModel }
                    , Cmd.map HomeMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        WorkflowsMsg subMsg ->
            case model.page of
                WorkflowsPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            WorkflowList.update subMsg subModel
                    in
                    ( { model | page = WorkflowsPage newSubModel }
                    , Cmd.map WorkflowsMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        WorkflowNewMsg subMsg ->
            case model.page of
                WorkflowNewPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            WorkflowNew.update subMsg subModel
                    in
                    ( { model | page = WorkflowNewPage newSubModel }
                    , Cmd.map WorkflowNewMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        WorkflowDetailMsg subMsg ->
            case model.page of
                WorkflowDetailPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            WorkflowDetail.update subMsg subModel
                    in
                    ( { model | page = WorkflowDetailPage newSubModel }
                    , Cmd.map WorkflowDetailMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        TasksMsg subMsg ->
            case model.page of
                TasksPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            TaskList.update subMsg subModel
                    in
                    ( { model | page = TasksPage newSubModel }
                    , Cmd.map TasksMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        TaskDetailMsg subMsg ->
            case model.page of
                TaskDetailPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            TaskDetail.update subMsg subModel
                    in
                    ( { model | page = TaskDetailPage newSubModel }
                    , Cmd.map TaskDetailMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )



-- SUBSCRIPTIONS


{-| 外部イベントの購読

現在は購読なし。将来的に WebSocket、Ports、タイマーを追加予定。
詳細: [Ports 設計](../../../docs/06_ナレッジベース/elm/Elmポート.md)

-}
subscriptions : Model -> Sub Msg
subscriptions _ =
    Sub.none



-- VIEW


{-| Model から HTML を生成

アプリシェルレイアウト:

  - デスクトップ（lg 以上）: 固定サイドバー + コンテンツエリア
  - モバイル: トップバー + スライドインサイドバー + オーバーレイ

-}
view : Model -> Browser.Document Msg
view model =
    { title = pageTitle model.route ++ " | RingiFlow"
    , body =
        [ div [ class "flex h-screen bg-secondary-50 overflow-hidden" ]
            [ viewSidebar model.route model.sidebarOpen model.shared
            , viewMobileOverlay model.sidebarOpen
            , div [ class "flex flex-col flex-1 overflow-hidden" ]
                [ viewTopBar
                , main_
                    [ class "flex-1 overflow-y-auto p-6" ]
                    [ div [ class "mx-auto max-w-5xl" ]
                        [ viewPage model ]
                    ]
                ]
            ]
        ]
    }


{-| ルートに応じたページタイトル
-}
pageTitle : Route -> String
pageTitle route =
    case route of
        Route.Home ->
            "ダッシュボード"

        Route.Workflows ->
            "申請一覧"

        Route.WorkflowNew ->
            "新規申請"

        Route.WorkflowDetail _ ->
            "申請詳細"

        Route.Tasks ->
            "タスク一覧"

        Route.TaskDetail _ ->
            "タスク詳細"

        Route.NotFound ->
            "ページが見つかりません"


{-| サイドバーナビゲーション

デスクトップ: 常に表示（w-64）
モバイル: sidebarOpen 時にスライドイン

-}
viewSidebar : Route -> Bool -> Shared -> Html Msg
viewSidebar currentRoute isOpen shared =
    let
        mobileVisibility =
            if isOpen then
                "translate-x-0"

            else
                "-translate-x-full"
    in
    aside
        [ class
            ("fixed inset-y-0 left-0 z-30 flex w-sidebar flex-col bg-secondary-900 text-white transition-transform duration-200 ease-in-out lg:static lg:translate-x-0 "
                ++ mobileVisibility
            )
        ]
        [ -- ロゴエリア
          div [ class "flex h-16 items-center px-6" ]
            [ span [ class "text-xl font-bold tracking-wide" ]
                [ text "RingiFlow" ]
            ]

        -- ナビゲーションリンク
        , nav [ class "flex-1 space-y-1 px-3 py-4" ]
            [ viewNavItem currentRoute Route.Home "ダッシュボード" iconDashboard
            , viewNavItem currentRoute Route.Workflows "申請一覧" iconWorkflows
            , viewNavItem currentRoute Route.Tasks "タスク一覧" iconTasks
            ]

        -- フッター（ユーザー情報 + Copyright）
        , div [ class "border-t border-secondary-700 px-4 py-4" ]
            [ viewUserInfo shared
            , div [ class "mt-3 text-center text-xs text-secondary-500" ]
                [ text "© 2026 RingiFlow" ]
            ]
        ]


{-| ナビゲーション項目

アクティブ状態は `isRouteActive` で判定し、背景色を変更する。

-}
viewNavItem : Route -> Route -> String -> Html Msg -> Html Msg
viewNavItem currentRoute targetRoute label icon =
    let
        isActive =
            Route.isRouteActive targetRoute currentRoute

        activeClass =
            if isActive then
                "bg-primary-600 text-white"

            else
                "text-secondary-100 hover:bg-secondary-700"
    in
    a
        [ href (Route.toString targetRoute)
        , class ("flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors " ++ activeClass)
        ]
        [ icon
        , span [] [ text label ]
        ]


{-| ユーザー情報表示
-}
viewUserInfo : Shared -> Html Msg
viewUserInfo shared =
    case shared.user of
        Just user ->
            div [ class "flex items-center gap-3" ]
                [ div [ class "flex h-8 w-8 items-center justify-center rounded-full bg-primary-600 text-sm font-medium" ]
                    [ text (String.left 1 user.name) ]
                , div [ class "min-w-0 flex-1" ]
                    [ div [ class "truncate text-sm font-medium" ] [ text user.name ]
                    , div [ class "truncate text-xs text-secondary-500" ] [ text user.email ]
                    ]
                ]

        Nothing ->
            div [ class "text-sm text-secondary-500" ]
                [ text "未ログイン" ]


{-| トップバー（モバイル用ハンバーガーメニュー）
-}
viewTopBar : Html Msg
viewTopBar =
    header [ class "flex h-16 items-center border-b border-secondary-100 bg-white px-4 lg:px-6" ]
        [ -- ハンバーガーボタン（モバイルのみ）
          button
            [ class "mr-4 rounded-lg p-2 text-secondary-500 hover:bg-secondary-50 lg:hidden"
            , onClick ToggleSidebar
            ]
            [ iconMenu ]

        -- ページヘッダー領域（将来の検索バー等に使用）
        , div [ class "flex-1" ] []
        ]


{-| モバイルオーバーレイ（サイドバー表示時の背景暗転）
-}
viewMobileOverlay : Bool -> Html Msg
viewMobileOverlay isOpen =
    if isOpen then
        div
            [ class "fixed inset-0 z-20 bg-black/50 lg:hidden"
            , onClick CloseSidebar
            ]
            []

    else
        text ""


{-| ページコンテンツの描画

Page に応じて対応するページモジュールの view を呼び出す。
Nested TEA パターンにより、ページの Msg は Main の Msg にマップされる。

-}
viewPage : Model -> Html Msg
viewPage model =
    case model.page of
        HomePage subModel ->
            Home.view subModel
                |> Html.map HomeMsg

        WorkflowsPage subModel ->
            WorkflowList.view subModel
                |> Html.map WorkflowsMsg

        WorkflowNewPage subModel ->
            WorkflowNew.view subModel
                |> Html.map WorkflowNewMsg

        WorkflowDetailPage subModel ->
            WorkflowDetail.view subModel
                |> Html.map WorkflowDetailMsg

        TasksPage subModel ->
            TaskList.view subModel
                |> Html.map TasksMsg

        TaskDetailPage subModel ->
            TaskDetail.view subModel
                |> Html.map TaskDetailMsg

        NotFoundPage ->
            Page.NotFound.view



-- ICONS


{-| SVG アイコン: ダッシュボード（グリッド）
-}
iconDashboard : Html msg
iconDashboard =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.rect [ SvgAttr.x "3", SvgAttr.y "3", SvgAttr.width "7", SvgAttr.height "7", SvgAttr.rx "1" ] []
        , Svg.rect [ SvgAttr.x "14", SvgAttr.y "3", SvgAttr.width "7", SvgAttr.height "7", SvgAttr.rx "1" ] []
        , Svg.rect [ SvgAttr.x "3", SvgAttr.y "14", SvgAttr.width "7", SvgAttr.height "7", SvgAttr.rx "1" ] []
        , Svg.rect [ SvgAttr.x "14", SvgAttr.y "14", SvgAttr.width "7", SvgAttr.height "7", SvgAttr.rx "1" ] []
        ]


{-| SVG アイコン: 申請一覧（ドキュメント）
-}
iconWorkflows : Html msg
iconWorkflows =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.path [ SvgAttr.d "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z" ] []
        , Svg.path [ SvgAttr.d "M14 2v6h6" ] []
        , Svg.line [ SvgAttr.x1 "16", SvgAttr.y1 "13", SvgAttr.x2 "8", SvgAttr.y2 "13" ] []
        , Svg.line [ SvgAttr.x1 "16", SvgAttr.y1 "17", SvgAttr.x2 "8", SvgAttr.y2 "17" ] []
        ]


{-| SVG アイコン: タスク一覧（チェックリスト）
-}
iconTasks : Html msg
iconTasks =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.path [ SvgAttr.d "M9 11l3 3L22 4" ] []
        , Svg.path [ SvgAttr.d "M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" ] []
        ]


{-| SVG アイコン: ハンバーガーメニュー
-}
iconMenu : Html msg
iconMenu =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-6 w-6"
        ]
        [ Svg.line [ SvgAttr.x1 "3", SvgAttr.y1 "6", SvgAttr.x2 "21", SvgAttr.y2 "6" ] []
        , Svg.line [ SvgAttr.x1 "3", SvgAttr.y1 "12", SvgAttr.x2 "21", SvgAttr.y2 "12" ] []
        , Svg.line [ SvgAttr.x1 "3", SvgAttr.y1 "18", SvgAttr.x2 "21", SvgAttr.y2 "18" ] []
        ]
