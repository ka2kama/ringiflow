port module Ports exposing
    ( receiveCanvasBounds
    , receiveMessage
    , requestCanvasBounds
    , sendMessage
    , setBeforeUnloadEnabled
    , showModalDialog
    )

{-| JavaScript との通信用 Ports モジュール

Elm と JavaScript の間に型安全な通信チャネルを提供する。

詳細: [Elm Ports](../../../docs/06_ナレッジベース/elm/Elmポート.md)

-}

import Json.Encode as Encode


{-| JavaScript へメッセージを送信

    sendMessage
        (Encode.object
            [ ( "type", Encode.string "NOTIFY" )
            , ( "payload", Encode.string content )
            ]
        )

-}
port sendMessage : Encode.Value -> Cmd msg


{-| JavaScript からメッセージを受信

    subscriptions model =
        receiveMessage ReceivedMessage

-}
port receiveMessage : (Encode.Value -> msg) -> Sub msg


{-| ブラウザの beforeunload イベントの有効/無効を制御

フォーム入力中の未保存データ損失を防ぐため、
ページ離脱時にブラウザの警告ダイアログを表示する。

    -- フォームが dirty になったとき
    Ports.setBeforeUnloadEnabled True

    -- 保存/送信成功時
    Ports.setBeforeUnloadEnabled False

-}
port setBeforeUnloadEnabled : Bool -> Cmd msg


{-| モーダルダイアログを表示

`<dialog>` 要素の `showModal()` を呼び出す。
引数はダイアログ要素の HTML id。

    -- ConfirmDialog の表示
    Ports.showModalDialog ConfirmDialog.dialogId

-}
port showModalDialog : String -> Cmd msg


{-| キャンバス SVG 要素の境界情報をリクエスト

getBoundingClientRect の結果を receiveCanvasBounds で受け取る。
引数は SVG 要素の HTML id。

-}
port requestCanvasBounds : String -> Cmd msg


{-| キャンバス SVG 要素の境界情報を受信

JSON 形式: { x, y, width, height }

-}
port receiveCanvasBounds : (Encode.Value -> msg) -> Sub msg
