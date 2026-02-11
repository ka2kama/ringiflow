module Data.RoleTest exposing (suite)

{-| Data.Role モジュールのテスト
-}

import Data.Role as Role
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.Role"
        [ roleItemDecoderTests
        , roleItemListDecoderTests
        , roleDetailDecoderTests
        ]



-- RoleItem decoder


roleItemDecoderTests : Test
roleItemDecoderTests =
    describe "roleItemDecoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "00000000-0000-0000-0000-000000000001",
                            "name": "admin",
                            "description": "管理者ロール",
                            "permissions": ["workflow:read", "user:read"],
                            "is_system": true,
                            "user_count": 3
                        }
                        """
                in
                Decode.decodeString Role.roleItemDecoder json
                    |> Expect.equal
                        (Ok
                            { id = "00000000-0000-0000-0000-000000000001"
                            , name = "admin"
                            , description = Just "管理者ロール"
                            , permissions = [ "workflow:read", "user:read" ]
                            , isSystem = True
                            , userCount = 3
                            }
                        )
        , test "description が null の場合 Nothing" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "00000000-0000-0000-0000-000000000002",
                            "name": "custom",
                            "description": null,
                            "permissions": [],
                            "is_system": false,
                            "user_count": 0
                        }
                        """
                in
                Decode.decodeString Role.roleItemDecoder json
                    |> Result.map .description
                    |> Expect.equal (Ok Nothing)
        ]



-- RoleItem listDecoder


roleItemListDecoderTests : Test
roleItemListDecoderTests =
    describe "roleItemListDecoder"
        [ test "data フィールドから一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "id-1",
                                    "name": "admin",
                                    "description": "管理者",
                                    "permissions": ["workflow:read"],
                                    "is_system": true,
                                    "user_count": 2
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString Role.roleItemListDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 1)
        ]



-- RoleDetail decoder


roleDetailDecoderTests : Test
roleDetailDecoderTests =
    describe "roleDetailDecoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "id": "00000000-0000-0000-0000-000000000001",
                                "name": "admin",
                                "description": "管理者ロール",
                                "permissions": ["workflow:read", "user:read"],
                                "is_system": true,
                                "created_at": "2026-01-01T00:00:00Z",
                                "updated_at": "2026-02-01T00:00:00Z"
                            }
                        }
                        """
                in
                Decode.decodeString Role.roleDetailDecoder json
                    |> Expect.equal
                        (Ok
                            { id = "00000000-0000-0000-0000-000000000001"
                            , name = "admin"
                            , description = Just "管理者ロール"
                            , permissions = [ "workflow:read", "user:read" ]
                            , isSystem = True
                            , createdAt = "2026-01-01T00:00:00Z"
                            , updatedAt = "2026-02-01T00:00:00Z"
                            }
                        )
        ]
