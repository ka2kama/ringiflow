module Component.FormFieldTest exposing (suite)

{-| Component.FormField モジュールのテスト

CSS クラス計算ロジックを検証する。
elm-html-test が未導入のため、公開されたヘルパー関数をテストする。

-}

import Component.FormField as FormField
import Expect
import Test exposing (..)


suite : Test
suite =
    describe "Component.FormField"
        [ inputClassTests
        ]



-- inputClass


inputClassTests : Test
inputClassTests =
    describe "inputClass"
        [ test "エラーなし → 通常スタイル（border-secondary-300）を含む" <|
            \_ ->
                FormField.inputClass Nothing
                    |> String.contains "border-secondary-300"
                    |> Expect.equal True
        , test "エラーなし → focus-visible:border-primary-500 を含む" <|
            \_ ->
                FormField.inputClass Nothing
                    |> String.contains "focus-visible:border-primary-500"
                    |> Expect.equal True
        , test "エラーあり → エラースタイル（border-error-300）を含む" <|
            \_ ->
                FormField.inputClass (Just "エラーメッセージ")
                    |> String.contains "border-error-300"
                    |> Expect.equal True
        , test "エラーあり → focus-visible:border-error-500 を含む" <|
            \_ ->
                FormField.inputClass (Just "エラーメッセージ")
                    |> String.contains "focus-visible:border-error-500"
                    |> Expect.equal True
        , test "エラーなし → エラースタイルを含まない" <|
            \_ ->
                FormField.inputClass Nothing
                    |> String.contains "border-error-300"
                    |> Expect.equal False
        , test "エラーあり → 通常のフォーカススタイルを含まない" <|
            \_ ->
                FormField.inputClass (Just "エラー")
                    |> String.contains "focus-visible:border-primary-500"
                    |> Expect.equal False
        , test "共通の基本クラス（rounded-lg）を含む" <|
            \_ ->
                FormField.inputClass Nothing
                    |> String.contains "rounded-lg"
                    |> Expect.equal True
        ]
