module Data.WorkflowCommentTest exposing (suite)

{-| Data.WorkflowComment モジュールのテスト

コメントの JSON デコーダーの正確性を検証する。

-}

import Data.WorkflowComment as WorkflowComment
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.WorkflowComment"
        [ decoderTests
        , listDecoderTests
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
                            "id": "comment-001",
                            "posted_by": {"id": "user-001", "name": "テストユーザー"},
                            "body": "承認をお願いします",
                            "created_at": "2026-01-15T10:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowComment.decoder json
                    |> Result.map
                        (\c ->
                            { id = c.id
                            , postedByName = c.postedBy.name
                            , body = c.body
                            , createdAt = c.createdAt
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "comment-001"
                            , postedByName = "テストユーザー"
                            , body = "承認をお願いします"
                            , createdAt = "2026-01-15T10:00:00Z"
                            }
                        )
        , test "必須フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "comment-001"
                        }
                        """
                in
                Decode.decodeString WorkflowComment.decoder json
                    |> Expect.err
        ]



-- listDecoder


listDecoderTests : Test
listDecoderTests =
    describe "listDecoder"
        [ test "data フィールドからコメント一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "comment-001",
                                    "posted_by": {"id": "user-001", "name": "ユーザー1"},
                                    "body": "コメント1",
                                    "created_at": "2026-01-15T10:00:00Z"
                                },
                                {
                                    "id": "comment-002",
                                    "posted_by": {"id": "user-002", "name": "ユーザー2"},
                                    "body": "コメント2",
                                    "created_at": "2026-01-15T11:00:00Z"
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString WorkflowComment.listDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 2)
        , test "空のコメント一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": []
                        }
                        """
                in
                Decode.decodeString WorkflowComment.listDecoder json
                    |> Expect.equal (Ok [])
        , test "data フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        []
                        """
                in
                Decode.decodeString WorkflowComment.listDecoder json
                    |> Expect.err
        ]
