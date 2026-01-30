module Main exposing (main)

{-| RingiFlow メインモジュール

TEA (The Elm Architecture) に基づく SPA のエントリーポイント。

詳細: [TEA パターン](../../../docs/05_技術ノート/Elmアーキテクチャ.md)

-}

import Api.Auth as AuthApi
import Api.Http exposing (ApiError)
import Browser
import Browser.Navigation as Nav
import Html exposing (..)
import Html.Attributes exposing (..)
import Page.Home
import Page.NotFound
import Page.Workflow.Detail as WorkflowDetail
import Page.Workflow.List as WorkflowList
import Page.Workflow.New as WorkflowNew
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
    }



-- MODEL


{-| 現在のページ状態

Nested TEA パターンにより、各ページの Model を Page 型で保持する。
状態を持たないページ（Home, NotFound）は専用のコンストラクタを使用。

-}
type Page
    = HomePage
    | WorkflowsPage WorkflowList.Model
    | WorkflowNewPage WorkflowNew.Model
    | WorkflowDetailPage WorkflowDetail.Model
    | NotFoundPage


{-| アプリケーションの状態

グローバル状態（Shared）と現在のページ状態を保持する。

-}
type alias Model =
    { key : Nav.Key
    , url : Url
    , route : Route
    , shared : Shared
    , page : Page
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
            ( HomePage, Cmd.none )

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

        Route.NotFound ->
            ( NotFoundPage, Cmd.none )


{-| ページの Shared を更新

CSRF トークン取得後など、グローバルな Shared が更新されたときに
各ページの Shared も同期する。

-}
updatePageShared : Shared -> Page -> Page
updatePageShared shared page =
    case page of
        HomePage ->
            HomePage

        WorkflowsPage subModel ->
            WorkflowsPage (WorkflowList.updateShared shared subModel)

        WorkflowNewPage subModel ->
            WorkflowNewPage (WorkflowNew.updateShared shared subModel)

        WorkflowDetailPage subModel ->
            WorkflowDetailPage (WorkflowDetail.updateShared shared subModel)

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
    | WorkflowsMsg WorkflowList.Msg
    | WorkflowNewMsg WorkflowNew.Msg
    | WorkflowDetailMsg WorkflowDetail.Msg


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
            ( { model | url = url, route = route, page = page }
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



-- SUBSCRIPTIONS


{-| 外部イベントの購読

現在は購読なし。将来的に WebSocket、Ports、タイマーを追加予定。
詳細: [Ports 設計](../../../docs/05_技術ノート/Elmポート.md)

-}
subscriptions : Model -> Sub Msg
subscriptions _ =
    Sub.none



-- VIEW


{-| Model から HTML を生成
-}
view : Model -> Browser.Document Msg
view model =
    { title = "RingiFlow"
    , body =
        [ viewHeader
        , viewMain model
        , viewFooter
        ]
    }


{-| ヘッダー部分の描画
-}
viewHeader : Html Msg
viewHeader =
    header
        [ style "background-color" "#1a73e8"
        , style "color" "white"
        , style "padding" "1rem"
        ]
        [ h1 [ style "margin" "0", style "font-size" "1.5rem" ]
            [ text "RingiFlow" ]
        ]


{-| メインコンテンツ部分の描画

Page に応じて対応するページモジュールの view を呼び出す。
Nested TEA パターンにより、ページの Msg は Main の Msg にマップされる。

-}
viewMain : Model -> Html Msg
viewMain model =
    main_
        [ style "padding" "2rem"
        , style "max-width" "1200px"
        , style "margin" "0 auto"
        ]
        [ case model.page of
            HomePage ->
                Page.Home.view

            WorkflowsPage subModel ->
                WorkflowList.view subModel
                    |> Html.map WorkflowsMsg

            WorkflowNewPage subModel ->
                WorkflowNew.view subModel
                    |> Html.map WorkflowNewMsg

            WorkflowDetailPage subModel ->
                WorkflowDetail.view subModel
                    |> Html.map WorkflowDetailMsg

            NotFoundPage ->
                Page.NotFound.view
        ]


{-| フッター部分の描画
-}
viewFooter : Html Msg
viewFooter =
    footer
        [ style "background-color" "#f1f3f4"
        , style "padding" "1rem"
        , style "text-align" "center"
        , style "margin-top" "2rem"
        ]
        [ text "© 2026 RingiFlow" ]
