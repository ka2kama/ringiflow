module SharedTest exposing (suite)

{-| Shared モジュールのテスト
-}

import Expect
import Shared exposing (Shared)
import Test exposing (..)


suite : Test
suite =
    describe "Shared"
        [ isAdminTests
        ]



-- isAdmin


isAdminTests : Test
isAdminTests =
    describe "isAdmin"
        [ test "tenant_admin ロールを持つユーザー → True" <|
            \_ ->
                let
                    shared =
                        baseShared
                            |> Shared.withUser
                                { id = "user-1"
                                , email = "admin@example.com"
                                , name = "管理者"
                                , tenantId = "tenant-1"
                                , roles = [ "tenant_admin", "user" ]
                                }
                in
                Shared.isAdmin shared
                    |> Expect.equal True
        , test "tenant_admin ロールを持たないユーザー → False" <|
            \_ ->
                let
                    shared =
                        baseShared
                            |> Shared.withUser
                                { id = "user-2"
                                , email = "user@example.com"
                                , name = "一般ユーザー"
                                , tenantId = "tenant-1"
                                , roles = [ "user" ]
                                }
                in
                Shared.isAdmin shared
                    |> Expect.equal False
        , test "未ログイン → False" <|
            \_ ->
                Shared.isAdmin baseShared
                    |> Expect.equal False
        ]



-- Helpers


{-| テスト用の基本 Shared 状態
-}
baseShared : Shared
baseShared =
    Shared.init { apiBaseUrl = "http://localhost:3000", timezoneOffsetMinutes = 540 }
