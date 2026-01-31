module Util.KeyEventTest exposing (suite)

{-| Util.KeyEvent モジュールのテスト

ESC キーデコーダーの動作を検証する。

-}

import Expect
import Json.Decode as Decode
import Test exposing (..)
import Util.KeyEvent as KeyEvent


type TestMsg
    = EscPressed


suite : Test
suite =
    describe "Util.KeyEvent"
        [ escKeyDecoderTests
        ]


escKeyDecoderTests : Test
escKeyDecoderTests =
    describe "escKeyDecoder"
        [ test "\"Escape\" キーで指定のメッセージが返る" <|
            \_ ->
                Decode.decodeString (KeyEvent.escKeyDecoder EscPressed) """{"key": "Escape"}"""
                    |> Expect.equal (Ok EscPressed)
        , test "\"Escape\" 以外のキー（例: \"Enter\"）では fail する" <|
            \_ ->
                Decode.decodeString (KeyEvent.escKeyDecoder EscPressed) """{"key": "Enter"}"""
                    |> Expect.err
        ]
