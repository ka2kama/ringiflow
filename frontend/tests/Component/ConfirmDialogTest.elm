module Component.ConfirmDialogTest exposing (suite)

{-| Component.ConfirmDialog モジュールのテスト

dialog 要素の ARIA 属性とバックドロップクリックデコーダの動作を検証する。

-}

import Component.ConfirmDialog as ConfirmDialog
import Expect
import Html.Attributes
import Json.Decode as Decode
import Json.Encode as Encode
import Test exposing (..)
import Test.Html.Query as Query
import Test.Html.Selector as Selector


type TestMsg
    = Confirm
    | Cancel


defaultConfig :
    { title : String
    , message : String
    , confirmLabel : String
    , cancelLabel : String
    , onConfirm : TestMsg
    , onCancel : TestMsg
    , actionStyle : ConfirmDialog.ActionStyle
    }
defaultConfig =
    { title = "テストタイトル"
    , message = "テストメッセージ"
    , confirmLabel = "確認"
    , cancelLabel = "キャンセル"
    , onConfirm = Confirm
    , onCancel = Cancel
    , actionStyle = ConfirmDialog.Positive
    }


suite : Test
suite =
    describe "Component.ConfirmDialog"
        [ viewTests
        , backdropClickDecoderTests
        ]



-- view


viewTests : Test
viewTests =
    describe "view"
        [ test "dialog 要素で描画される" <|
            \_ ->
                ConfirmDialog.view defaultConfig
                    |> Query.fromHtml
                    |> Query.has [ Selector.tag "dialog" ]
        , test "aria-labelledby がタイトル要素の ID を参照する" <|
            \_ ->
                ConfirmDialog.view defaultConfig
                    |> Query.fromHtml
                    |> Query.has
                        [ Selector.attribute
                            (Html.Attributes.attribute "aria-labelledby" "confirm-dialog-title")
                        ]
        , test "aria-describedby がメッセージ要素の ID を参照する" <|
            \_ ->
                ConfirmDialog.view defaultConfig
                    |> Query.fromHtml
                    |> Query.has
                        [ Selector.attribute
                            (Html.Attributes.attribute "aria-describedby" "confirm-dialog-message")
                        ]
        , test "タイトル要素に正しい ID が付与されている" <|
            \_ ->
                ConfirmDialog.view defaultConfig
                    |> Query.fromHtml
                    |> Query.find [ Selector.id "confirm-dialog-title" ]
                    |> Query.has [ Selector.text "テストタイトル" ]
        , test "メッセージ要素に正しい ID が付与されている" <|
            \_ ->
                ConfirmDialog.view defaultConfig
                    |> Query.fromHtml
                    |> Query.find [ Selector.id "confirm-dialog-message" ]
                    |> Query.has [ Selector.text "テストメッセージ" ]
        ]



-- backdropClickDecoder


backdropClickDecoderTests : Test
backdropClickDecoderTests =
    describe "backdropClickDecoder"
        [ test "DIALOG ノードへのクリックで成功する" <|
            \_ ->
                let
                    json =
                        Encode.object
                            [ ( "target"
                              , Encode.object [ ( "nodeName", Encode.string "DIALOG" ) ]
                              )
                            ]
                in
                Decode.decodeValue (ConfirmDialog.backdropClickDecoder Cancel) json
                    |> Expect.equal (Ok Cancel)
        , test "子要素へのクリックで失敗する" <|
            \_ ->
                let
                    json =
                        Encode.object
                            [ ( "target"
                              , Encode.object [ ( "nodeName", Encode.string "DIV" ) ]
                              )
                            ]
                in
                Decode.decodeValue (ConfirmDialog.backdropClickDecoder Cancel) json
                    |> Expect.err
        ]
