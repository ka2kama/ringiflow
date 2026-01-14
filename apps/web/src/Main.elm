module Main exposing (main)

{-| RingiFlow メインモジュール

このモジュールは Elm アプリケーションのエントリーポイントであり、
The Elm Architecture (TEA) に基づいて構築されている。


## アーキテクチャ概要

TEA は以下の 3 つの要素で構成される:

  - **Model**: アプリケーションの状態を表す不変のデータ構造
  - **Update**: メッセージを受け取り、Model を更新する純粋関数
  - **View**: Model から HTML を生成する純粋関数

この単方向データフローにより、状態管理が予測可能になり、
バグの発生源を特定しやすくなる。


## なぜ Browser.application を選択したか

Elm には 4 種類のプログラム構造がある:

| 関数 | URL 管理 | 履歴 | 用途 |
| ---------------------- | -------- | ---- | ---------------------- |
| Browser.sandbox | なし | なし | 純粋な UI コンポーネント |
| Browser.element | なし | なし | 既存ページへの埋め込み |
| Browser.document | なし | なし | 全画面 SPA（URL 不要） |
| Browser.application | あり | あり | フル機能 SPA |

RingiFlow は複数のページ（ワークフロー一覧、詳細、設定等）を持つ
SPA であるため、URL ベースのルーティングが必要。
よって Browser.application を採用した。


## 代替案と不採用理由

  - **elm-spa パッケージ**: ルーティングを抽象化するが、
    学習目的で TEA の仕組みを理解するため、手動実装を選択
  - **Browser.document + hash routing**: URL 管理が手動になり煩雑


## 関連知識

TEA は Redux の設計に大きな影響を与えた。
Redux の reducer は TEA の update 関数に相当し、
action は Msg に相当する。

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

Program 型の 3 つの型パラメータ:

  - `Flags`: JavaScript から渡される初期データの型
  - `Model`: アプリケーション状態の型
  - `Msg`: 状態更新トリガーとなるメッセージの型

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


## 設計意図

Elm は純粋関数型言語であり、外部環境（現在時刻、環境変数等）に
直接アクセスできない。Flags を通じて JavaScript 側から
これらの値を注入する。


## フィールド説明

  - `apiBaseUrl`: API サーバーのベース URL。
    開発時は空文字（Vite のプロキシを使用）、
    本番時は実際の URL を設定
  - `timestamp`: アプリ起動時刻。キャッシュバスティングや
    セッション識別に使用可能

-}
type alias Flags =
    { apiBaseUrl : String
    , timestamp : Int
    }


{-| Flags のデコーダー


## なぜデコーダーが必要か

JavaScript から渡されるデータは型安全ではない。
Elm はデコーダーを通じて、期待する型に変換し、
不正なデータを検出する。


## 現在の実装について

この flagsDecoder は定義のみで、実際には使用していない。
Browser.application は Flags 型を直接受け取るため。
将来的に Flags の検証を厳密にする場合に備えて定義している。

-}
flagsDecoder : Decode.Decoder Flags
flagsDecoder =
    Decode.map2 Flags
        (Decode.field "apiBaseUrl" Decode.string)
        (Decode.field "timestamp" Decode.int)



-- MODEL


{-| アプリケーションの状態


## 設計方針

Model は最小限の状態のみを保持する。
派生値（例: 現在のルートに基づく表示内容）は
view 関数内で計算する。


## フィールド説明

  - `key`: ブラウザ履歴を操作するためのキー。
    Nav.pushUrl や Nav.back で使用。
    Elm ランタイムから提供され、自分で生成できない
  - `url`: 現在の URL。デバッグや条件分岐で使用
  - `route`: パース済みのルート。view での分岐に使用
  - `apiBaseUrl`: API 呼び出し時のベース URL

-}
type alias Model =
    { key : Nav.Key
    , url : Url
    , route : Route
    , apiBaseUrl : String
    }


{-| アプリケーションの初期化


## 処理フロー

1.  Flags から設定値を取得
2.  現在の URL をルートにパース
3.  初期 Model を構築
4.  初期コマンド（なし）を返す


## 戻り値の型 ( Model, Cmd Msg )

Elm の init/update は常に「新しい状態」と「副作用」の
タプルを返す。副作用がない場合は Cmd.none を使用。

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


## 設計方針

Msg は「何が起きたか」を表現し、「何をすべきか」ではない。
これにより update 関数がメッセージの解釈を担当し、
関心の分離が実現される。


## バリアント説明

  - `LinkClicked`: ユーザーがリンクをクリックした
      - `Internal`: 同一オリジン内のリンク → SPA 内遷移
      - `External`: 外部サイトへのリンク → 通常のページ遷移
  - `UrlChanged`: URL が変更された（履歴操作含む）

-}
type Msg
    = LinkClicked Browser.UrlRequest
    | UrlChanged Url


{-| メッセージに基づいて Model を更新


## パターンマッチングについて

Elm の case 式は網羅性チェックがある。
新しい Msg バリアントを追加すると、
ここでコンパイルエラーが発生し、対応漏れを防げる。


## 副作用の扱い

  - `Nav.pushUrl`: URL を変更し、履歴に追加
  - `Nav.load`: 通常のページ遷移（Elm アプリを離れる）

-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        LinkClicked urlRequest ->
            case urlRequest of
                Browser.Internal url ->
                    -- SPA 内遷移: 履歴に追加して URL 変更
                    ( model, Nav.pushUrl model.key (Url.toString url) )

                Browser.External href ->
                    -- 外部遷移: Elm アプリを離れる
                    ( model, Nav.load href )

        UrlChanged url ->
            -- URL 変更後の処理: Model を更新
            -- 将来的にはページ固有のデータ取得もここで行う
            ( { model | url = url, route = Route.fromUrl url }
            , Cmd.none
            )



-- SUBSCRIPTIONS


{-| 外部イベントの購読


## 現在の実装

購読なし。将来的に以下を追加予定:

  - WebSocket からのリアルタイム更新
  - Ports からの JavaScript イベント
  - タイマーによる定期更新

-}
subscriptions : Model -> Sub Msg
subscriptions _ =
    Sub.none



-- VIEW


{-| Model から HTML を生成


## Browser.Document 型について

Browser.application では view は Browser.Document を返す。
これは title（タブに表示）と body（HTML 要素のリスト）を持つ。


## 構造化について

view 関数を viewHeader, viewMain, viewFooter に分割することで、
各部分の責務を明確にし、再利用性を高めている。

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


## インラインスタイルについて

Phase 0 では CSS フレームワークを導入せず、
インラインスタイルで最小限のスタイリングを行っている。
Phase 1 以降で Tailwind CSS または elm-css の導入を検討。

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


## ルーティングの実現

route に基づいて case 式で分岐し、
該当するページコンポーネントを描画する。

将来的にページが増えた場合:

    case model.route of
        Route.Home ->
            viewHome

        Route.Workflows ->
            viewWorkflows model

        Route.WorkflowDetail id ->
            viewWorkflowDetail model id

        Route.NotFound ->
            viewNotFound

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
