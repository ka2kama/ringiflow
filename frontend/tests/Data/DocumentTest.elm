module Data.DocumentTest exposing (suite)

{-| Data.Document モジュールのテスト

Document, UploadUrlResponse, DownloadUrlResponse の JSON デコーダーの正確性を検証する。

-}

import Data.Document as Document
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.Document"
        [ documentDecoderTests
        , uploadUrlResponseDecoderTests
        , downloadUrlResponseDecoderTests
        , listDecoderTests
        ]



-- decoder


documentDecoderTests : Test
documentDecoderTests =
    describe "decoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "doc-001",
                            "filename": "領収書.pdf",
                            "content_type": "application/pdf",
                            "size": 1258291,
                            "status": "active",
                            "created_at": "2026-03-01T10:00:00Z"
                        }
                        """
                in
                Decode.decodeString Document.decoder json
                    |> Result.map
                        (\d ->
                            { id = d.id
                            , filename = d.filename
                            , contentType = d.contentType
                            , size = d.size
                            , status = d.status
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "doc-001"
                            , filename = "領収書.pdf"
                            , contentType = "application/pdf"
                            , size = 1258291
                            , status = "active"
                            }
                        )
        , test "必須フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "doc-001"
                        }
                        """
                in
                Decode.decodeString Document.decoder json
                    |> Expect.err
        ]



-- uploadUrlResponseDecoder


uploadUrlResponseDecoderTests : Test
uploadUrlResponseDecoderTests =
    describe "uploadUrlResponseDecoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "document_id": "doc-001",
                                "upload_url": "https://s3.example.com/presigned-put-url",
                                "expires_in": 300
                            }
                        }
                        """
                in
                Decode.decodeString Document.uploadUrlResponseDecoder json
                    |> Result.map
                        (\r ->
                            { documentId = r.documentId
                            , expiresIn = r.expiresIn
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { documentId = "doc-001"
                            , expiresIn = 300
                            }
                        )
        ]



-- downloadUrlResponseDecoder


downloadUrlResponseDecoderTests : Test
downloadUrlResponseDecoderTests =
    describe "downloadUrlResponseDecoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "download_url": "https://s3.example.com/presigned-get-url",
                                "expires_in": 900
                            }
                        }
                        """
                in
                Decode.decodeString Document.downloadUrlResponseDecoder json
                    |> Result.map .expiresIn
                    |> Expect.equal (Ok 900)
        ]



-- listDecoder


listDecoderTests : Test
listDecoderTests =
    describe "listDecoder"
        [ test "data フィールドからドキュメント一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "doc-001",
                                    "filename": "領収書.pdf",
                                    "content_type": "application/pdf",
                                    "size": 1258291,
                                    "status": "active",
                                    "created_at": "2026-03-01T10:00:00Z"
                                },
                                {
                                    "id": "doc-002",
                                    "filename": "見積書.xlsx",
                                    "content_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                                    "size": 524288,
                                    "status": "active",
                                    "created_at": "2026-03-01T11:00:00Z"
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString Document.listDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 2)
        , test "空のドキュメント一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": []
                        }
                        """
                in
                Decode.decodeString Document.listDecoder json
                    |> Expect.equal (Ok [])
        ]
