module Component.EmptyStateTest exposing (suite)

{-| Component.EmptyState モジュールのテスト

CSS クラス定数を検証する。
elm-html-test が未導入のため、公開されたヘルパー関数をテストする。

-}

import Component.EmptyState as EmptyState
import Expect
import Test exposing (..)


suite : Test
suite =
    describe "Component.EmptyState"
        [ containerClassTests
        ]



-- containerClass


containerClassTests : Test
containerClassTests =
    describe "containerClass"
        [ test "py-12 を含む" <|
            \_ ->
                EmptyState.containerClass
                    |> String.contains "py-12"
                    |> Expect.equal True
        , test "text-center を含む" <|
            \_ ->
                EmptyState.containerClass
                    |> String.contains "text-center"
                    |> Expect.equal True
        ]
