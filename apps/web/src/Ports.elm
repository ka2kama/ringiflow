port module Ports exposing
    ( receiveMessage
    , sendMessage
    )

{-| JavaScript との通信用 Ports モジュール

Elm と JavaScript の間に型安全な通信チャネルを提供する。

詳細: [Elm Ports](../../../docs/05_技術ノート/Elmポート.md)

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
