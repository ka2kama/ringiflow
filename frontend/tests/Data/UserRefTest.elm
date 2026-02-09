module Data.UserRefTest exposing (suite)

{-| Data.UserRef のデコーダテスト

ユーザー参照型の JSON デコーダが正しく動作することを検証する。

-}

import Data.UserRef as UserRef
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.UserRef"
        [ decoderTests
        ]


decoderTests : Test
decoderTests =
    describe "decoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "user-001",
                            "name": "山田太郎"
                        }
                        """
                in
                Decode.decodeString UserRef.decoder json
                    |> Expect.equal
                        (Ok
                            { id = "user-001"
                            , name = "山田太郎"
                            }
                        )
        , test "必須フィールド欠落でエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "user-001"
                        }
                        """
                in
                Decode.decodeString UserRef.decoder json
                    |> Expect.err
        ]
