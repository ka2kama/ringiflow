module Component.ButtonTest exposing (suite)

{-| Component.Button モジュールのテスト

各 Variant が正しい CSS クラスを生成するか検証する。

-}

import Component.Button as Button exposing (Variant(..))
import Expect
import Test exposing (..)


suite : Test
suite =
    describe "Component.Button"
        [ variantClassTests
        ]



-- variantClass


variantClassTests : Test
variantClassTests =
    describe "variantClass"
        [ test "Primary → bg-primary-500 を含む" <|
            \_ ->
                Button.variantClass Primary
                    |> String.contains "bg-primary-500"
                    |> Expect.equal True
        , test "Primary → hover:bg-primary-600 を含む" <|
            \_ ->
                Button.variantClass Primary
                    |> String.contains "hover:bg-primary-600"
                    |> Expect.equal True
        , test "Success → bg-success-600 を含む" <|
            \_ ->
                Button.variantClass Success
                    |> String.contains "bg-success-600"
                    |> Expect.equal True
        , test "Success → hover:bg-success-700 を含む" <|
            \_ ->
                Button.variantClass Success
                    |> String.contains "hover:bg-success-700"
                    |> Expect.equal True
        , test "Error → bg-error-600 を含む" <|
            \_ ->
                Button.variantClass Error
                    |> String.contains "bg-error-600"
                    |> Expect.equal True
        , test "Error → hover:bg-error-700 を含む" <|
            \_ ->
                Button.variantClass Error
                    |> String.contains "hover:bg-error-700"
                    |> Expect.equal True
        , test "Warning → bg-warning-600 を含む" <|
            \_ ->
                Button.variantClass Warning
                    |> String.contains "bg-warning-600"
                    |> Expect.equal True
        , test "Warning → hover:bg-warning-700 を含む" <|
            \_ ->
                Button.variantClass Warning
                    |> String.contains "hover:bg-warning-700"
                    |> Expect.equal True
        , test "Outline → border-secondary-300 を含む" <|
            \_ ->
                Button.variantClass Outline
                    |> String.contains "border-secondary-300"
                    |> Expect.equal True
        , test "Outline → bg-white を含む" <|
            \_ ->
                Button.variantClass Outline
                    |> String.contains "bg-white"
                    |> Expect.equal True
        , test "Outline → text-secondary-700 を含む" <|
            \_ ->
                Button.variantClass Outline
                    |> String.contains "text-secondary-700"
                    |> Expect.equal True
        , test "Primary → text-white を含む（Outline 以外は白文字）" <|
            \_ ->
                Button.variantClass Primary
                    |> String.contains "text-white"
                    |> Expect.equal True
        ]
