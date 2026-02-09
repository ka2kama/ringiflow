module Api.AuthTest exposing (suite)

{-| Api.Auth のデコーダテスト

認証 API レスポンスの JSON デコーダが正しく動作することを検証する。

-}

import Api.Auth as Auth
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Api.Auth"
        [ csrfTokenDecoderTests
        , userDecoderTests
        ]



-- ────────────────────────────────────
-- csrfTokenDecoder
-- ────────────────────────────────────


csrfTokenDecoderTests : Test
csrfTokenDecoderTests =
    describe "csrfTokenDecoder"
        [ test "data.token をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "token": "csrf-token-abc123"
                            }
                        }
                        """
                in
                Decode.decodeString Auth.csrfTokenDecoder json
                    |> Expect.equal (Ok "csrf-token-abc123")
        , test "data フィールド欠落でエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "token": "csrf-token-abc123"
                        }
                        """
                in
                Decode.decodeString Auth.csrfTokenDecoder json
                    |> Expect.err
        , test "token フィールド欠落でエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {}
                        }
                        """
                in
                Decode.decodeString Auth.csrfTokenDecoder json
                    |> Expect.err
        ]



-- ────────────────────────────────────
-- userDecoder
-- ────────────────────────────────────


userDecoderTests : Test
userDecoderTests =
    describe "userDecoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "id": "user-001",
                                "email": "yamada@example.com",
                                "name": "山田太郎",
                                "tenant_id": "tenant-001",
                                "roles": ["admin", "approver"]
                            }
                        }
                        """
                in
                Decode.decodeString Auth.userDecoder json
                    |> Expect.equal
                        (Ok
                            { id = "user-001"
                            , email = "yamada@example.com"
                            , name = "山田太郎"
                            , tenantId = "tenant-001"
                            , roles = [ "admin", "approver" ]
                            }
                        )
        , test "roles が空配列" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "id": "user-001",
                                "email": "yamada@example.com",
                                "name": "山田太郎",
                                "tenant_id": "tenant-001",
                                "roles": []
                            }
                        }
                        """
                in
                Decode.decodeString Auth.userDecoder json
                    |> Result.map .roles
                    |> Expect.equal (Ok [])
        , test "必須フィールド欠落でエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "id": "user-001",
                                "email": "yamada@example.com"
                            }
                        }
                        """
                in
                Decode.decodeString Auth.userDecoder json
                    |> Expect.err
        ]
