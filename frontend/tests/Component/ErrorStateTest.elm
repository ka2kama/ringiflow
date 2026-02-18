module Component.ErrorStateTest exposing (suite)

{-| Component.ErrorState モジュールのテスト

CSS クラス定数を検証する。
elm-html-test が未導入のため、公開されたヘルパー関数をテストする。

-}

import Component.ErrorState as ErrorState
import Expect
import Test exposing (..)


suite : Test
suite =
    describe "Component.ErrorState"
        [ containerClassTests
        ]



-- containerClass


containerClassTests : Test
containerClassTests =
    describe "containerClass"
        [ test "bg-error-50 を含む" <|
            \_ ->
                ErrorState.containerClass
                    |> String.contains "bg-error-50"
                    |> Expect.equal True
        , test "rounded-lg を含む" <|
            \_ ->
                ErrorState.containerClass
                    |> String.contains "rounded-lg"
                    |> Expect.equal True
        , test "text-error-700 を含む" <|
            \_ ->
                ErrorState.containerClass
                    |> String.contains "text-error-700"
                    |> Expect.equal True
        ]
