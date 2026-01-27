module Form.ValidationTest exposing (suite)

{-| Form.Validation モジュールのテスト
-}

import Data.FormField exposing (FieldType(..), FormField, Validation)
import Dict
import Expect
import Form.Validation as Validation
import Test exposing (..)


suite : Test
suite =
    describe "Form.Validation"
        [ validateTitleTests
        , validateAllFieldsTests
        ]



-- validateTitle


validateTitleTests : Test
validateTitleTests =
    describe "validateTitle"
        [ test "空文字列はエラー" <|
            \_ ->
                Validation.validateTitle ""
                    |> Expect.err
        , test "空白のみはエラー" <|
            \_ ->
                Validation.validateTitle "   "
                    |> Expect.err
        , test "1文字以上は OK" <|
            \_ ->
                Validation.validateTitle "a"
                    |> Expect.ok
        , test "200文字以内は OK" <|
            \_ ->
                Validation.validateTitle (String.repeat 200 "a")
                    |> Expect.ok
        , test "201文字以上はエラー" <|
            \_ ->
                Validation.validateTitle (String.repeat 201 "a")
                    |> Expect.err
        , test "日本語タイトルも OK" <|
            \_ ->
                Validation.validateTitle "経費精算申請"
                    |> Expect.ok
        ]



-- validateAllFields


validateAllFieldsTests : Test
validateAllFieldsTests =
    describe "validateAllFields"
        [ describe "required"
            [ test "必須フィールドが空だとエラー" <|
                \_ ->
                    let
                        fields =
                            [ requiredTextField "name" "名前" ]

                        values =
                            Dict.empty
                    in
                    Validation.validateAllFields fields values
                        |> Dict.get "name"
                        |> Expect.notEqual Nothing
            , test "必須フィールドに値があれば OK" <|
                \_ ->
                    let
                        fields =
                            [ requiredTextField "name" "名前" ]

                        values =
                            Dict.singleton "name" "田中太郎"
                    in
                    Validation.validateAllFields fields values
                        |> Dict.isEmpty
                        |> Expect.equal True
            , test "任意フィールドが空でも OK" <|
                \_ ->
                    let
                        fields =
                            [ optionalTextField "note" "備考" ]

                        values =
                            Dict.empty
                    in
                    Validation.validateAllFields fields values
                        |> Dict.isEmpty
                        |> Expect.equal True
            ]
        , describe "minLength"
            [ test "最小文字数未満はエラー" <|
                \_ ->
                    let
                        fields =
                            [ textFieldWithMinLength "code" "コード" 3 ]

                        values =
                            Dict.singleton "code" "ab"
                    in
                    Validation.validateAllFields fields values
                        |> Dict.get "code"
                        |> Expect.notEqual Nothing
            , test "最小文字数以上は OK" <|
                \_ ->
                    let
                        fields =
                            [ textFieldWithMinLength "code" "コード" 3 ]

                        values =
                            Dict.singleton "code" "abc"
                    in
                    Validation.validateAllFields fields values
                        |> Dict.isEmpty
                        |> Expect.equal True
            ]
        , describe "maxLength"
            [ test "最大文字数超過はエラー" <|
                \_ ->
                    let
                        fields =
                            [ textFieldWithMaxLength "code" "コード" 5 ]

                        values =
                            Dict.singleton "code" "abcdef"
                    in
                    Validation.validateAllFields fields values
                        |> Dict.get "code"
                        |> Expect.notEqual Nothing
            , test "最大文字数以内は OK" <|
                \_ ->
                    let
                        fields =
                            [ textFieldWithMaxLength "code" "コード" 5 ]

                        values =
                            Dict.singleton "code" "abcde"
                    in
                    Validation.validateAllFields fields values
                        |> Dict.isEmpty
                        |> Expect.equal True
            ]
        , describe "Number min/max"
            [ test "最小値未満はエラー" <|
                \_ ->
                    let
                        fields =
                            [ numberFieldWithMin "amount" "金額" 100 ]

                        values =
                            Dict.singleton "amount" "99"
                    in
                    Validation.validateAllFields fields values
                        |> Dict.get "amount"
                        |> Expect.notEqual Nothing
            , test "最小値以上は OK" <|
                \_ ->
                    let
                        fields =
                            [ numberFieldWithMin "amount" "金額" 100 ]

                        values =
                            Dict.singleton "amount" "100"
                    in
                    Validation.validateAllFields fields values
                        |> Dict.isEmpty
                        |> Expect.equal True
            , test "最大値超過はエラー" <|
                \_ ->
                    let
                        fields =
                            [ numberFieldWithMax "amount" "金額" 10000 ]

                        values =
                            Dict.singleton "amount" "10001"
                    in
                    Validation.validateAllFields fields values
                        |> Dict.get "amount"
                        |> Expect.notEqual Nothing
            ]
        , describe "複数フィールド"
            [ test "複数エラーを返す" <|
                \_ ->
                    let
                        fields =
                            [ requiredTextField "name" "名前"
                            , requiredTextField "email" "メール"
                            ]

                        values =
                            Dict.empty
                    in
                    Validation.validateAllFields fields values
                        |> Dict.size
                        |> Expect.equal 2
            ]
        ]



-- Test Helpers


defaultValidation : Validation
defaultValidation =
    { required = False
    , minLength = Nothing
    , maxLength = Nothing
    , min = Nothing
    , max = Nothing
    }


requiredTextField : String -> String -> FormField
requiredTextField id label =
    { id = id
    , label = label
    , fieldType = Text
    , placeholder = Nothing
    , validation = { defaultValidation | required = True }
    }


optionalTextField : String -> String -> FormField
optionalTextField id label =
    { id = id
    , label = label
    , fieldType = Text
    , placeholder = Nothing
    , validation = defaultValidation
    }


textFieldWithMinLength : String -> String -> Int -> FormField
textFieldWithMinLength id label minLen =
    { id = id
    , label = label
    , fieldType = Text
    , placeholder = Nothing
    , validation = { defaultValidation | minLength = Just minLen }
    }


textFieldWithMaxLength : String -> String -> Int -> FormField
textFieldWithMaxLength id label maxLen =
    { id = id
    , label = label
    , fieldType = Text
    , placeholder = Nothing
    , validation = { defaultValidation | maxLength = Just maxLen }
    }


numberFieldWithMin : String -> String -> Float -> FormField
numberFieldWithMin id label minVal =
    { id = id
    , label = label
    , fieldType = Number
    , placeholder = Nothing
    , validation = { defaultValidation | min = Just minVal }
    }


numberFieldWithMax : String -> String -> Float -> FormField
numberFieldWithMax id label maxVal =
    { id = id
    , label = label
    , fieldType = Number
    , placeholder = Nothing
    , validation = { defaultValidation | max = Just maxVal }
    }
