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
import Component.ConfirmDialog as ConfirmDialog
import Component.Icons as Icons
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick)
import Page.AuditLog.List as AuditLogList
import Page.Home as Home
import Page.NotFound
import Page.Role.Edit as RoleEdit
import Page.Role.List as RoleList
import Page.Role.New as RoleNew
import Page.Task.Detail as TaskDetail
import Page.Task.List as TaskList
import Page.User.Detail as UserDetail
import Page.User.Edit as UserEdit
import Page.User.List as UserList
import Page.User.New as UserNew
import Page.Workflow.Detail as WorkflowDetail
import Page.Workflow.List as WorkflowList
import Page.Workflow.New as WorkflowNew
import Ports
import Route exposing (Route)
import Shared exposing (Shared)
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
    , timezoneOffsetMinutes : Int
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
    | UsersPage UserList.Model
    | UserDetailPage UserDetail.Model
    | UserNewPage UserNew.Model
    | UserEditPage UserEdit.Model
    | RolesPage RoleList.Model
    | RoleNewPage RoleNew.Model
    | RoleEditPage RoleEdit.Model
    | AuditLogsPage AuditLogList.Model
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
    , pendingNavigation : Maybe Url
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
            Shared.init
                { apiBaseUrl = flags.apiBaseUrl
                , timezoneOffsetMinutes = flags.timezoneOffsetMinutes
                }

        ( page, pageCmd ) =
            initPage key route shared

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
      , pendingNavigation = Nothing
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
initPage : Nav.Key -> Route -> Shared -> ( Page, Cmd Msg )
initPage key route shared =
    case route of
        Route.Home ->
            let
                ( model, cmd ) =
                    Home.init shared
            in
            ( HomePage model, Cmd.map HomeMsg cmd )

        Route.Workflows filter ->
            let
                ( model, cmd ) =
                    WorkflowList.init shared key filter
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

        Route.TaskDetail workflowDisplayNumber stepDisplayNumber ->
            let
                ( model, cmd ) =
                    TaskDetail.init shared workflowDisplayNumber stepDisplayNumber
            in
            ( TaskDetailPage model, Cmd.map TaskDetailMsg cmd )

        Route.Users ->
            let
                ( model, cmd ) =
                    UserList.init shared
            in
            ( UsersPage model, Cmd.map UsersMsg cmd )

        Route.UserDetail displayNumber ->
            let
                ( model, cmd ) =
                    UserDetail.init shared displayNumber
            in
            ( UserDetailPage model, Cmd.map UserDetailMsg cmd )

        Route.UserNew ->
            let
                ( model, cmd ) =
                    UserNew.init shared
            in
            ( UserNewPage model, Cmd.map UserNewMsg cmd )

        Route.UserEdit displayNumber ->
            let
                ( model, cmd ) =
                    UserEdit.init shared key displayNumber
            in
            ( UserEditPage model, Cmd.map UserEditMsg cmd )

        Route.Roles ->
            let
                ( model, cmd ) =
                    RoleList.init shared
            in
            ( RolesPage model, Cmd.map RolesMsg cmd )

        Route.RoleNew ->
            let
                ( model, cmd ) =
                    RoleNew.init shared key
            in
            ( RoleNewPage model, Cmd.map RoleNewMsg cmd )

        Route.RoleEdit roleId ->
            let
                ( model, cmd ) =
                    RoleEdit.init shared key roleId
            in
            ( RoleEditPage model, Cmd.map RoleEditMsg cmd )

        Route.AuditLogs ->
            let
                ( model, cmd ) =
                    AuditLogList.init shared
            in
            ( AuditLogsPage model, Cmd.map AuditLogsMsg cmd )

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

        UsersPage subModel ->
            UsersPage (UserList.updateShared shared subModel)

        UserDetailPage subModel ->
            UserDetailPage (UserDetail.updateShared shared subModel)

        UserNewPage subModel ->
            UserNewPage (UserNew.updateShared shared subModel)

        UserEditPage subModel ->
            UserEditPage (UserEdit.updateShared shared subModel)

        RolesPage subModel ->
            RolesPage (RoleList.updateShared shared subModel)

        RoleNewPage subModel ->
            RoleNewPage (RoleNew.updateShared shared subModel)

        RoleEditPage subModel ->
            RoleEditPage (RoleEdit.updateShared shared subModel)

        AuditLogsPage subModel ->
            AuditLogsPage (AuditLogList.updateShared shared subModel)

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
    | ConfirmNavigation
    | CancelNavigation
    | HomeMsg Home.Msg
    | WorkflowsMsg WorkflowList.Msg
    | WorkflowNewMsg WorkflowNew.Msg
    | WorkflowDetailMsg WorkflowDetail.Msg
    | TasksMsg TaskList.Msg
    | TaskDetailMsg TaskDetail.Msg
    | UsersMsg UserList.Msg
    | UserDetailMsg UserDetail.Msg
    | UserNewMsg UserNew.Msg
    | UserEditMsg UserEdit.Msg
    | RolesMsg RoleList.Msg
    | RoleNewMsg RoleNew.Msg
    | RoleEditMsg RoleEdit.Msg
    | AuditLogsMsg AuditLogList.Msg


{-| メッセージに基づいて Model を更新
-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        LinkClicked urlRequest ->
            case urlRequest of
                Browser.Internal url ->
                    if isCurrentPageDirty model then
                        ( { model | pendingNavigation = Just url }
                        , Ports.showModalDialog ConfirmDialog.dialogId
                        )

                    else
                        ( model, Nav.pushUrl model.key (Url.toString url) )

                Browser.External href ->
                    ( model, Nav.load href )

        UrlChanged url ->
            let
                newRoute =
                    Route.fromUrl url
            in
            case ( model.page, newRoute ) of
                ( WorkflowsPage subModel, Route.Workflows newFilter ) ->
                    -- 同一ページ: フィルタのみ更新（データ再取得しない）
                    let
                        ( newSubModel, subCmd ) =
                            WorkflowList.applyFilter newFilter subModel
                    in
                    ( { model
                        | url = url
                        , route = newRoute
                        , page = WorkflowsPage newSubModel
                        , sidebarOpen = False
                      }
                    , Cmd.map WorkflowsMsg subCmd
                    )

                _ ->
                    let
                        ( page, pageCmd ) =
                            initPage model.key newRoute model.shared
                    in
                    ( { model
                        | url = url
                        , route = newRoute
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

        ConfirmNavigation ->
            case model.pendingNavigation of
                Just url ->
                    ( { model | pendingNavigation = Nothing }
                    , Cmd.batch
                        [ Nav.pushUrl model.key (Url.toString url)
                        , Ports.setBeforeUnloadEnabled False
                        ]
                    )

                Nothing ->
                    ( model, Cmd.none )

        CancelNavigation ->
            ( { model | pendingNavigation = Nothing }
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

        UsersMsg subMsg ->
            case model.page of
                UsersPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            UserList.update subMsg subModel
                    in
                    ( { model | page = UsersPage newSubModel }
                    , Cmd.map UsersMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        UserDetailMsg subMsg ->
            case model.page of
                UserDetailPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            UserDetail.update subMsg subModel
                    in
                    ( { model | page = UserDetailPage newSubModel }
                    , Cmd.map UserDetailMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        UserNewMsg subMsg ->
            case model.page of
                UserNewPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            UserNew.update subMsg subModel
                    in
                    ( { model | page = UserNewPage newSubModel }
                    , Cmd.map UserNewMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        UserEditMsg subMsg ->
            case model.page of
                UserEditPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            UserEdit.update subMsg subModel
                    in
                    ( { model | page = UserEditPage newSubModel }
                    , Cmd.map UserEditMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        RolesMsg subMsg ->
            case model.page of
                RolesPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            RoleList.update subMsg subModel
                    in
                    ( { model | page = RolesPage newSubModel }
                    , Cmd.map RolesMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        RoleNewMsg subMsg ->
            case model.page of
                RoleNewPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            RoleNew.update subMsg subModel
                    in
                    ( { model | page = RoleNewPage newSubModel }
                    , Cmd.map RoleNewMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        RoleEditMsg subMsg ->
            case model.page of
                RoleEditPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            RoleEdit.update subMsg subModel
                    in
                    ( { model | page = RoleEditPage newSubModel }
                    , Cmd.map RoleEditMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )

        AuditLogsMsg subMsg ->
            case model.page of
                AuditLogsPage subModel ->
                    let
                        ( newSubModel, subCmd ) =
                            AuditLogList.update subMsg subModel
                    in
                    ( { model | page = AuditLogsPage newSubModel }
                    , Cmd.map AuditLogsMsg subCmd
                    )

                _ ->
                    ( model, Cmd.none )


{-| 現在のページに未保存の変更があるかを判定
-}
isCurrentPageDirty : Model -> Bool
isCurrentPageDirty model =
    case model.page of
        WorkflowNewPage subModel ->
            WorkflowNew.isDirty subModel

        UserNewPage subModel ->
            UserNew.isDirty subModel

        UserEditPage subModel ->
            UserEdit.isDirty subModel

        RoleNewPage subModel ->
            RoleNew.isDirty subModel

        RoleEditPage subModel ->
            RoleEdit.isDirty subModel

        _ ->
            False



-- SUBSCRIPTIONS


{-| 外部イベントの購読

現在のページに応じて、各ページの subscriptions にルーティングする。
詳細: [Ports 設計](../../../docs/06_ナレッジベース/elm/Elmポート.md)

-}
subscriptions : Model -> Sub Msg
subscriptions model =
    case model.page of
        TaskDetailPage _ ->
            Sub.map TaskDetailMsg TaskDetail.subscriptions

        WorkflowDetailPage _ ->
            Sub.map WorkflowDetailMsg WorkflowDetail.subscriptions

        _ ->
            Sub.none



-- VIEW


{-| Model から HTML を生成

アプリシェルレイアウト:

  - デスクトップ（lg 以上）: 固定サイドバー + コンテンツエリア
  - モバイル: トップバー + スライドインサイドバー + オーバーレイ

-}
view : Model -> Browser.Document Msg
view model =
    { title = Route.pageTitle model.route ++ " | RingiFlow"
    , body =
        [ a [ class "sr-only focus:not-sr-only focus:absolute focus:z-50 focus:bg-white focus:p-4 focus:text-primary-600", href "#main-content" ]
            [ text "メインコンテンツにスキップ" ]
        , div [ class "flex h-screen bg-secondary-50 overflow-hidden" ]
            [ viewSidebar model.route model.sidebarOpen model.shared
            , viewMobileOverlay model.sidebarOpen
            , div [ class "flex flex-col flex-1 overflow-hidden" ]
                [ viewTopBar
                , main_
                    [ id "main-content", class "flex-1 overflow-y-auto p-6" ]
                    [ div [ class "mx-auto max-w-5xl" ]
                        [ viewPage model ]
                    ]
                ]
            ]
        , viewNavigationConfirmDialog model.pendingNavigation
        ]
    }


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
            [ a [ href (Route.toString Route.Home), class "text-xl font-bold tracking-wide text-white no-underline" ]
                [ text "RingiFlow" ]
            ]

        -- ナビゲーションリンク
        , nav [ class "flex-1 space-y-1 px-3 py-4" ]
            ([ viewNavItem currentRoute Route.Home "ダッシュボード" Icons.dashboard
             , viewNavItem currentRoute (Route.Workflows Route.emptyWorkflowFilter) "申請一覧" Icons.workflows
             , viewNavItem currentRoute Route.Tasks "タスク一覧" Icons.tasks
             ]
                ++ viewAdminSection currentRoute shared
            )

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


{-| 管理セクション（admin ロール限定）

テナント管理者のみに表示するサイドバーセクション。
ユーザー管理、ロール管理、監査ログへのリンクを提供する。

-}
viewAdminSection : Route -> Shared -> List (Html Msg)
viewAdminSection currentRoute shared =
    if Shared.isAdmin shared then
        [ div [ class "mt-6 px-3 text-xs font-semibold uppercase tracking-wider text-secondary-500" ]
            [ text "管理" ]
        , viewNavItem currentRoute Route.Users "ユーザー管理" Icons.users
        , viewNavItem currentRoute Route.Roles "ロール管理" Icons.roles
        , viewNavItem currentRoute Route.AuditLogs "監査ログ" Icons.auditLog
        ]

    else
        []


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
            , attribute "aria-label" "メニューを開く"
            , onClick ToggleSidebar
            ]
            [ Icons.menu ]

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

        UsersPage subModel ->
            UserList.view subModel
                |> Html.map UsersMsg

        UserDetailPage subModel ->
            UserDetail.view subModel
                |> Html.map UserDetailMsg

        UserNewPage subModel ->
            UserNew.view subModel
                |> Html.map UserNewMsg

        UserEditPage subModel ->
            UserEdit.view subModel
                |> Html.map UserEditMsg

        RolesPage subModel ->
            RoleList.view subModel
                |> Html.map RolesMsg

        RoleNewPage subModel ->
            RoleNew.view subModel
                |> Html.map RoleNewMsg

        RoleEditPage subModel ->
            RoleEdit.view subModel
                |> Html.map RoleEditMsg

        AuditLogsPage subModel ->
            AuditLogList.view subModel
                |> Html.map AuditLogsMsg

        NotFoundPage ->
            Page.NotFound.view


{-| ナビゲーション確認ダイアログ

フォームに未保存の変更がある状態でページ離脱を試みた場合に表示する。

-}
viewNavigationConfirmDialog : Maybe Url -> Html Msg
viewNavigationConfirmDialog maybePendingUrl =
    case maybePendingUrl of
        Just _ ->
            ConfirmDialog.view
                { title = "ページを離れますか？"
                , message = "入力中のデータは保存されません。このページを離れてもよろしいですか？"
                , confirmLabel = "ページを離れる"
                , cancelLabel = "このページに留まる"
                , onConfirm = ConfirmNavigation
                , onCancel = CancelNavigation
                , actionStyle = ConfirmDialog.Destructive
                }

        Nothing ->
            text ""
