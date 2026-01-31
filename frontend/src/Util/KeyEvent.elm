module Util.KeyEvent exposing (escKeyDecoder)

{-| キーボードイベントのユーティリティ

`Browser.Events.onKeyDown` と組み合わせて使用する。


## 使用例

    import Browser.Events
    import Util.KeyEvent as KeyEvent

    subscriptions : Model -> Sub Msg
    subscriptions model =
        Browser.Events.onKeyDown (KeyEvent.escKeyDecoder EscPressed)

-}

import Json.Decode as Decode


{-| ESC キーのデコーダー

KeyboardEvent の `key` フィールドが "Escape" の場合に指定のメッセージを返す。
それ以外のキーでは `Decode.fail` する（購読がメッセージを発行しない）。

-}
escKeyDecoder : msg -> Decode.Decoder msg
escKeyDecoder msg =
    Decode.field "key" Decode.string
        |> Decode.andThen
            (\key ->
                if key == "Escape" then
                    Decode.succeed msg

                else
                    Decode.fail ("Not Escape key: " ++ key)
            )
