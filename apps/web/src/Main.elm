module Main exposing (main)

{-| RingiFlow メインモジュール

TEA (The Elm Architecture) に基づく SPA のエントリーポイント。

詳細: [TEA パターン](../../../docs/05_技術ノート/Elmアーキテクチャ.md)

-}

import Browser
import Browser.Navigation as Nav
import Html exposing (..)
import Html.Attributes exposing (..)
import Json.Decode as Decode
import Route exposing (Route)
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


{-| Flags のデコーダー（将来の検証強化用に定義）
-}
flagsDecoder : Decode.Decoder Flags
flagsDecoder =
    Decode.map2 Flags
        (Decode.field "apiBaseUrl" Decode.string)
        (Decode.field "timestamp" Decode.int)



-- MODEL


{-| アプリケーションの状態

最小限の状態のみを保持し、派生値は view 関数内で計算する。

-}
type alias Model =
    { key : Nav.Key
    , url : Url
    , route : Route
    , apiBaseUrl : String
    }


{-| アプリケーションの初期化
-}
init : Flags -> Url -> Nav.Key -> ( Model, Cmd Msg )
init flags url key =
    let
        route =
            Route.fromUrl url
    in
    ( { key = key
      , url = url
      , route = route
      , apiBaseUrl = flags.apiBaseUrl
      }
    , Cmd.none
    )



-- UPDATE


{-| アプリケーションで発生するメッセージ
-}
type Msg
    = LinkClicked Browser.UrlRequest
    | UrlChanged Url


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
            ( { model | url = url, route = Route.fromUrl url }
            , Cmd.none
            )



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
-}
viewMain : Model -> Html Msg
viewMain model =
    main_
        [ style "padding" "2rem"
        , style "max-width" "1200px"
        , style "margin" "0 auto"
        ]
        [ case model.route of
            Route.Home ->
                viewHome

            Route.NotFound ->
                viewNotFound
        ]


{-| ホームページの描画
-}
viewHome : Html Msg
viewHome =
    div []
        [ h2 [] [ text "ようこそ RingiFlow へ" ]
        , p [] [ text "ワークフロー管理システムです。" ]
        , div
            [ style "background-color" "white"
            , style "padding" "1.5rem"
            , style "border-radius" "8px"
            , style "box-shadow" "0 2px 4px rgba(0,0,0,0.1)"
            , style "margin-top" "1rem"
            ]
            [ h3 [] [ text "Phase 0 完了" ]
            , p [] [ text "Elm フロントエンドが正常に動作しています。" ]
            ]
        ]


{-| 404 ページの描画
-}
viewNotFound : Html Msg
viewNotFound =
    div []
        [ h2 [] [ text "404 - ページが見つかりません" ]
        , p []
            [ text "お探しのページは存在しません。"
            , a [ href "/" ] [ text "ホームに戻る" ]
            ]
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
