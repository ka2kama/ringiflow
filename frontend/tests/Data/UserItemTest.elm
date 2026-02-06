module Data.UserItemTest exposing (suite)

{-| Data.UserItem モジュールのテスト
-}

import Data.UserItem as UserItem
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.UserItem"
        [ decoderTests
        , listDecoderTests
        , filterUsersTests
        ]



-- decoder


decoderTests : Test
decoderTests =
    describe "decoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "00000000-0000-0000-0000-000000000001",
                            "display_id": "USER-5",
                            "display_number": 5,
                            "name": "山田太郎",
                            "email": "yamada.taro@example.com"
                        }
                        """
                in
                Decode.decodeString UserItem.decoder json
                    |> Expect.equal
                        (Ok
                            { id = "00000000-0000-0000-0000-000000000001"
                            , displayId = "USER-5"
                            , displayNumber = 5
                            , name = "山田太郎"
                            , email = "yamada.taro@example.com"
                            }
                        )
        , test "必須フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "00000000-0000-0000-0000-000000000001"
                        }
                        """
                in
                Decode.decodeString UserItem.decoder json
                    |> Expect.err
        ]



-- listDecoder


listDecoderTests : Test
listDecoderTests =
    describe "listDecoder"
        [ test "data フィールドから一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "00000000-0000-0000-0000-000000000001",
                                    "display_id": "USER-1",
                                    "display_number": 1,
                                    "name": "山田太郎",
                                    "email": "yamada.taro@example.com"
                                },
                                {
                                    "id": "00000000-0000-0000-0000-000000000002",
                                    "display_id": "USER-2",
                                    "display_number": 2,
                                    "name": "田中花子",
                                    "email": "tanaka.hanako@example.com"
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString UserItem.listDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 2)
        , test "空の一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": []
                        }
                        """
                in
                Decode.decodeString UserItem.listDecoder json
                    |> Expect.equal (Ok [])
        , test "data フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        []
                        """
                in
                Decode.decodeString UserItem.listDecoder json
                    |> Expect.err
        ]



-- filterUsers


{-| テスト用のサンプルユーザー
-}
sampleUsers : List UserItem.UserItem
sampleUsers =
    [ { id = "id-1", displayId = "USER-1", displayNumber = 1, name = "山田太郎", email = "yamada.taro@example.com" }
    , { id = "id-2", displayId = "USER-2", displayNumber = 2, name = "山田花子", email = "yamada.hanako@example.com" }
    , { id = "id-3", displayId = "USER-3", displayNumber = 3, name = "田中一郎", email = "tanaka.ichiro@example.com" }
    ]


filterUsersTests : Test
filterUsersTests =
    describe "filterUsers"
        [ test "名前で部分一致フィルタリング" <|
            \_ ->
                UserItem.filterUsers "山田" sampleUsers
                    |> List.map .name
                    |> Expect.equal [ "山田太郎", "山田花子" ]
        , test "display_id でフィルタリング" <|
            \_ ->
                UserItem.filterUsers "USER-1" sampleUsers
                    |> List.map .name
                    |> Expect.equal [ "山田太郎" ]
        , test "email でフィルタリング" <|
            \_ ->
                UserItem.filterUsers "tanaka" sampleUsers
                    |> List.map .name
                    |> Expect.equal [ "田中一郎" ]
        , test "大文字小文字を無視" <|
            \_ ->
                UserItem.filterUsers "user-2" sampleUsers
                    |> List.map .name
                    |> Expect.equal [ "山田花子" ]
        , test "空クエリは空リストを返す" <|
            \_ ->
                UserItem.filterUsers "" sampleUsers
                    |> Expect.equal []
        , test "一致なしは空リストを返す" <|
            \_ ->
                UserItem.filterUsers "存在しない名前" sampleUsers
                    |> Expect.equal []
        , test "前後の空白をトリム" <|
            \_ ->
                UserItem.filterUsers " 山田 " sampleUsers
                    |> List.map .name
                    |> Expect.equal [ "山田太郎", "山田花子" ]
        ]
