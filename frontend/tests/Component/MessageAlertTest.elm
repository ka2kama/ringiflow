module Component.MessageAlertTest exposing (suite)

{-| Component.MessageAlert モジュールのテスト

アラート表示の有無とアニメーションクラスの適用を検証する。

-}

import Component.MessageAlert as MessageAlert
import Html.Attributes
import Test exposing (..)
import Test.Html.Query as Query
import Test.Html.Selector as Selector


type TestMsg
    = Dismiss


defaultConfig :
    { onDismiss : TestMsg
    , successMessage : Maybe String
    , errorMessage : Maybe String
    }
defaultConfig =
    { onDismiss = Dismiss
    , successMessage = Nothing
    , errorMessage = Nothing
    }


suite : Test
suite =
    describe "Component.MessageAlert"
        [ viewTests
        ]



-- view


viewTests : Test
viewTests =
    describe "view"
        [ test "成功メッセージが表示されるとき animate-alert-in クラスを含む" <|
            \_ ->
                MessageAlert.view { defaultConfig | successMessage = Just "保存しました" }
                    |> Query.fromHtml
                    |> Query.find [ Selector.attribute (Html.Attributes.attribute "role" "alert") ]
                    |> Query.has [ Selector.class "animate-alert-in" ]
        , test "エラーメッセージが表示されるとき animate-alert-in クラスを含む" <|
            \_ ->
                MessageAlert.view { defaultConfig | errorMessage = Just "エラーが発生しました" }
                    |> Query.fromHtml
                    |> Query.find [ Selector.attribute (Html.Attributes.attribute "role" "alert") ]
                    |> Query.has [ Selector.class "animate-alert-in" ]
        , test "成功メッセージが Nothing のとき alert 要素が存在しない" <|
            \_ ->
                MessageAlert.view { defaultConfig | successMessage = Nothing }
                    |> Query.fromHtml
                    |> Query.hasNot [ Selector.attribute (Html.Attributes.attribute "role" "alert") ]
        , test "エラーメッセージが Nothing のとき alert 要素が存在しない" <|
            \_ ->
                MessageAlert.view { defaultConfig | errorMessage = Nothing }
                    |> Query.fromHtml
                    |> Query.hasNot [ Selector.attribute (Html.Attributes.attribute "role" "alert") ]
        , test "閉じボタンに focus-visible:ring-2 クラスが適用される" <|
            \_ ->
                MessageAlert.view { defaultConfig | successMessage = Just "成功" }
                    |> Query.fromHtml
                    |> Query.find [ Selector.attribute (Html.Attributes.attribute "aria-label" "閉じる") ]
                    |> Query.has [ Selector.class "focus-visible:ring-2" ]
        ]
