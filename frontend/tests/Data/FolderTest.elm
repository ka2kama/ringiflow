module Data.FolderTest exposing (suite)

{-| Data.Folder モジュールのテスト
-}

import Data.Folder as Folder
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.Folder"
        [ folderDecoderTests
        , folderListDecoderTests
        ]



-- Folder decoder


folderDecoderTests : Test
folderDecoderTests =
    describe "decoder"
        [ test "全フィールドをデコード（parent_id あり）" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "folder-001",
                            "name": "経費精算",
                            "parent_id": "folder-root",
                            "path": "/2026年度/経費精算/",
                            "depth": 2,
                            "created_at": "2026-03-01T00:00:00Z",
                            "updated_at": "2026-03-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString Folder.decoder json
                    |> Expect.equal
                        (Ok
                            { id = "folder-001"
                            , name = "経費精算"
                            , parentId = Just "folder-root"
                            , path = "/2026年度/経費精算/"
                            , depth = 2
                            , createdAt = "2026-03-01T00:00:00Z"
                            , updatedAt = "2026-03-01T00:00:00Z"
                            }
                        )
        , test "parent_id が null の場合 Nothing" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "folder-root",
                            "name": "2026年度",
                            "parent_id": null,
                            "path": "/2026年度/",
                            "depth": 1,
                            "created_at": "2026-03-01T00:00:00Z",
                            "updated_at": "2026-03-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString Folder.decoder json
                    |> Result.map .parentId
                    |> Expect.equal (Ok Nothing)
        ]



-- Folder list decoder


folderListDecoderTests : Test
folderListDecoderTests =
    describe "listDecoder"
        [ test "フォルダ一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        [
                            {
                                "id": "folder-001",
                                "name": "2026年度",
                                "parent_id": null,
                                "path": "/2026年度/",
                                "depth": 1,
                                "created_at": "2026-03-01T00:00:00Z",
                                "updated_at": "2026-03-01T00:00:00Z"
                            },
                            {
                                "id": "folder-002",
                                "name": "経費精算",
                                "parent_id": "folder-001",
                                "path": "/2026年度/経費精算/",
                                "depth": 2,
                                "created_at": "2026-03-01T00:00:00Z",
                                "updated_at": "2026-03-01T00:00:00Z"
                            }
                        ]
                        """
                in
                Decode.decodeString Folder.listDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 2)
        ]
