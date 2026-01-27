module Data.FormFieldTest exposing (suite)

{-| Data.FormField モジュールのテスト

JSON デコーダーの正確性を検証する。

-}

import Data.FormField as FormField exposing (FieldType(..))
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.FormField"
        [ decoderTests
        , fieldTypeTests
        ]



-- decoder


decoderTests : Test
decoderTests =
    describe "decoder"
        [ test "最小限のフィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "name",
                            "label": "名前",
                            "type": "text"
                        }
                        """
                in
                Decode.decodeString FormField.decoder json
                    |> Result.map .id
                    |> Expect.equal (Ok "name")
        , test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "amount",
                            "label": "金額",
                            "type": "number",
                            "placeholder": "金額を入力",
                            "validation": {
                                "required": true,
                                "min": 0,
                                "max": 1000000
                            }
                        }
                        """
                in
                Decode.decodeString FormField.decoder json
                    |> Result.map
                        (\f ->
                            { id = f.id
                            , required = f.validation.required
                            , min = f.validation.min
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "amount"
                            , required = True
                            , min = Just 0
                            }
                        )
        , test "validation がない場合はデフォルト値" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "note",
                            "label": "備考",
                            "type": "text"
                        }
                        """
                in
                Decode.decodeString FormField.decoder json
                    |> Result.map .validation
                    |> Result.map .required
                    |> Expect.equal (Ok False)
        ]



-- FieldType


fieldTypeTests : Test
fieldTypeTests =
    describe "FieldType"
        [ test "text タイプ" <|
            \_ ->
                decodeFieldType "text"
                    |> Expect.equal (Ok Text)
        , test "number タイプ" <|
            \_ ->
                decodeFieldType "number"
                    |> Expect.equal (Ok Number)
        , test "date タイプ" <|
            \_ ->
                decodeFieldType "date"
                    |> Expect.equal (Ok Date)
        , test "file タイプ" <|
            \_ ->
                decodeFieldType "file"
                    |> Expect.equal (Ok File)
        , test "select タイプ" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "category",
                            "label": "カテゴリ",
                            "type": "select",
                            "options": [
                                {"value": "travel", "label": "交通費"},
                                {"value": "meal", "label": "飲食費"}
                            ]
                        }
                        """
                in
                Decode.decodeString FormField.decoder json
                    |> Result.map .fieldType
                    |> Result.map
                        (\ft ->
                            case ft of
                                Select options ->
                                    List.length options

                                _ ->
                                    0
                        )
                    |> Expect.equal (Ok 2)
        , test "未知のタイプは Text にフォールバック" <|
            \_ ->
                decodeFieldType "unknown"
                    |> Expect.equal (Ok Text)
        ]


{-| FieldType だけをデコードするヘルパー
-}
decodeFieldType : String -> Result Decode.Error FieldType
decodeFieldType typeStr =
    let
        json =
            """
            {
                "id": "test",
                "label": "Test",
                "type": \""""
                ++ typeStr
                ++ """"
            }
            """
    in
    Decode.decodeString FormField.decoder json
        |> Result.map .fieldType
