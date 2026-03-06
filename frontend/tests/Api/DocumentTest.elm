module Api.DocumentTest exposing (suite)

{-| Api.Document のエンコーダテスト

アップロード URL リクエストのエンコーダが OpenAPI 仕様（content\_length）と一致することを検証する。

-}

import Api.Document as DocumentApi
import Expect
import Json.Decode as Decode
import Json.Encode as Encode
import Test exposing (..)


suite : Test
suite =
    describe "Api.Document"
        [ encodeUploadRequestTests
        ]



-- ────────────────────────────────────
-- encodeUploadRequest
-- ────────────────────────────────────


encodeUploadRequestTests : Test
encodeUploadRequestTests =
    describe "encodeUploadRequest"
        [ test "content_length フィールドでファイルサイズをエンコードする" <|
            \_ ->
                let
                    request =
                        { filename = "test.pdf"
                        , contentType = "application/pdf"
                        , size = 1024
                        , workflowInstanceId = "wf-001"
                        }

                    encoded =
                        DocumentApi.encodeUploadRequest request
                            |> Encode.encode 0

                    decodeField field decoder =
                        Decode.decodeString (Decode.field field decoder) encoded
                in
                Expect.all
                    [ \_ -> decodeField "filename" Decode.string |> Expect.equal (Ok "test.pdf")
                    , \_ -> decodeField "content_type" Decode.string |> Expect.equal (Ok "application/pdf")
                    , \_ -> decodeField "content_length" Decode.int |> Expect.equal (Ok 1024)
                    , \_ -> decodeField "workflow_instance_id" Decode.string |> Expect.equal (Ok "wf-001")
                    ]
                    ()
        ]
