module Data.AdminUserTest exposing (suite)

{-| Data.AdminUser モジュールのテスト
-}

import Data.AdminUser as AdminUser
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.AdminUser"
        [ adminUserItemDecoderTests
        , adminUserItemListDecoderTests
        , userDetailDecoderTests
        , createUserResponseDecoderTests
        , statusToBadgeTests
        ]



-- AdminUserItem decoder


adminUserItemDecoderTests : Test
adminUserItemDecoderTests =
    describe "adminUserItemDecoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "00000000-0000-0000-0000-000000000001",
                            "display_id": "USR-000005",
                            "display_number": 5,
                            "name": "山田太郎",
                            "email": "yamada@example.com",
                            "status": "active",
                            "roles": ["admin", "user"]
                        }
                        """
                in
                Decode.decodeString AdminUser.adminUserItemDecoder json
                    |> Expect.equal
                        (Ok
                            { id = "00000000-0000-0000-0000-000000000001"
                            , displayId = "USR-000005"
                            , displayNumber = 5
                            , name = "山田太郎"
                            , email = "yamada@example.com"
                            , status = "active"
                            , roles = [ "admin", "user" ]
                            }
                        )
        , test "必須フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        { "id": "00000000-0000-0000-0000-000000000001" }
                        """
                in
                Decode.decodeString AdminUser.adminUserItemDecoder json
                    |> Expect.err
        ]



-- AdminUserItem listDecoder


adminUserItemListDecoderTests : Test
adminUserItemListDecoderTests =
    describe "adminUserItemListDecoder"
        [ test "data フィールドから一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "id-1",
                                    "display_id": "USR-000001",
                                    "display_number": 1,
                                    "name": "山田太郎",
                                    "email": "yamada@example.com",
                                    "status": "active",
                                    "roles": ["admin"]
                                },
                                {
                                    "id": "id-2",
                                    "display_id": "USR-000002",
                                    "display_number": 2,
                                    "name": "田中花子",
                                    "email": "tanaka@example.com",
                                    "status": "inactive",
                                    "roles": ["user"]
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString AdminUser.adminUserItemListDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 2)
        , test "空の一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        { "data": [] }
                        """
                in
                Decode.decodeString AdminUser.adminUserItemListDecoder json
                    |> Expect.equal (Ok [])
        ]



-- UserDetail decoder


userDetailDecoderTests : Test
userDetailDecoderTests =
    describe "userDetailDecoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "id": "00000000-0000-0000-0000-000000000001",
                                "display_id": "USR-000005",
                                "display_number": 5,
                                "name": "山田太郎",
                                "email": "yamada@example.com",
                                "status": "active",
                                "roles": ["admin"],
                                "permissions": ["workflow:read", "workflow:create"],
                                "tenant_name": "テスト企業"
                            }
                        }
                        """
                in
                Decode.decodeString AdminUser.userDetailDecoder json
                    |> Expect.equal
                        (Ok
                            { id = "00000000-0000-0000-0000-000000000001"
                            , displayId = "USR-000005"
                            , displayNumber = 5
                            , name = "山田太郎"
                            , email = "yamada@example.com"
                            , status = "active"
                            , roles = [ "admin" ]
                            , permissions = [ "workflow:read", "workflow:create" ]
                            , tenantName = "テスト企業"
                            }
                        )
        ]



-- CreateUserResponse decoder


createUserResponseDecoderTests : Test
createUserResponseDecoderTests =
    describe "createUserResponseDecoder"
        [ test "初期パスワード付きレスポンスをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "id": "00000000-0000-0000-0000-000000000001",
                                "display_id": "USR-000010",
                                "display_number": 10,
                                "name": "新規ユーザー",
                                "email": "new@example.com",
                                "role": "user",
                                "initial_password": "Abc123!@#$%^&*XY"
                            }
                        }
                        """
                in
                Decode.decodeString AdminUser.createUserResponseDecoder json
                    |> Expect.equal
                        (Ok
                            { id = "00000000-0000-0000-0000-000000000001"
                            , displayId = "USR-000010"
                            , displayNumber = 10
                            , name = "新規ユーザー"
                            , email = "new@example.com"
                            , role = "user"
                            , initialPassword = "Abc123!@#$%^&*XY"
                            }
                        )
        ]



-- statusToBadge


statusToBadgeTests : Test
statusToBadgeTests =
    describe "statusToBadge"
        [ test "active → 成功色とアクティブラベル" <|
            \_ ->
                AdminUser.statusToBadge "active"
                    |> Expect.equal { colorClass = "bg-success-100 text-success-800", label = "アクティブ" }
        , test "inactive → セカンダリ色と非アクティブラベル" <|
            \_ ->
                AdminUser.statusToBadge "inactive"
                    |> Expect.equal { colorClass = "bg-secondary-100 text-secondary-800", label = "非アクティブ" }
        , test "未知の値 → セカンダリ色でステータス値をラベルに" <|
            \_ ->
                AdminUser.statusToBadge "suspended"
                    |> Expect.equal { colorClass = "bg-secondary-100 text-secondary-800", label = "suspended" }
        ]
