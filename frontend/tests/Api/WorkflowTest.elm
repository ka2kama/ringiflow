module Api.WorkflowTest exposing (suite)

{-| Api.Workflow のエンコーダテスト

ワークフロー API のリクエストエンコーダが正しく動作することを検証する。
エンコード結果を JSON 文字列化し、デコードして往復検証する。

-}

import Api.Workflow as Workflow
import Expect
import Json.Decode as Decode
import Json.Encode as Encode
import Test exposing (..)


suite : Test
suite =
    describe "Api.Workflow"
        [ encodeCreateRequestTests
        , encodeSubmitRequestTests
        , encodeApproveRejectRequestTests
        ]



-- ────────────────────────────────────
-- encodeCreateRequest
-- ────────────────────────────────────


encodeCreateRequestTests : Test
encodeCreateRequestTests =
    describe "encodeCreateRequest"
        [ test "全フィールドをエンコード" <|
            \_ ->
                let
                    request =
                        { definitionId = "def-001"
                        , title = "テスト申請"
                        , formData = Encode.object [ ( "field1", Encode.string "value1" ) ]
                        }

                    encoded =
                        Workflow.encodeCreateRequest request
                            |> Encode.encode 0

                    decodeField field decoder =
                        Decode.decodeString (Decode.field field decoder) encoded
                in
                Expect.all
                    [ \_ -> decodeField "definition_id" Decode.string |> Expect.equal (Ok "def-001")
                    , \_ -> decodeField "title" Decode.string |> Expect.equal (Ok "テスト申請")
                    , \_ ->
                        decodeField "form_data" (Decode.field "field1" Decode.string)
                            |> Expect.equal (Ok "value1")
                    ]
                    ()
        ]



-- ────────────────────────────────────
-- encodeSubmitRequest
-- ────────────────────────────────────


encodeSubmitRequestTests : Test
encodeSubmitRequestTests =
    describe "encodeSubmitRequest"
        [ test "approvers 配列をエンコード" <|
            \_ ->
                let
                    request =
                        { approvers =
                            [ { stepId = "approval", assignedTo = "user-002" }
                            ]
                        }

                    encoded =
                        Workflow.encodeSubmitRequest request
                            |> Encode.encode 0

                    approversDecoder =
                        Decode.field "approvers"
                            (Decode.list
                                (Decode.map2 Tuple.pair
                                    (Decode.field "step_id" Decode.string)
                                    (Decode.field "assigned_to" Decode.string)
                                )
                            )
                in
                Decode.decodeString approversDecoder encoded
                    |> Expect.equal (Ok [ ( "approval", "user-002" ) ])
        ]



-- ────────────────────────────────────
-- encodeApproveRejectRequest
-- ────────────────────────────────────


encodeApproveRejectRequestTests : Test
encodeApproveRejectRequestTests =
    describe "encodeApproveRejectRequest"
        [ test "コメントありの場合 version + comment" <|
            \_ ->
                let
                    request =
                        { version = 3
                        , comment = Just "承認します"
                        }

                    encoded =
                        Workflow.encodeApproveRejectRequest request
                            |> Encode.encode 0
                in
                Expect.all
                    [ \_ ->
                        Decode.decodeString (Decode.field "version" Decode.int) encoded
                            |> Expect.equal (Ok 3)
                    , \_ ->
                        Decode.decodeString (Decode.field "comment" Decode.string) encoded
                            |> Expect.equal (Ok "承認します")
                    ]
                    ()
        , test "コメントなしの場合 version のみ" <|
            \_ ->
                let
                    request =
                        { version = 1
                        , comment = Nothing
                        }

                    encoded =
                        Workflow.encodeApproveRejectRequest request
                            |> Encode.encode 0
                in
                Expect.all
                    [ \_ ->
                        Decode.decodeString (Decode.field "version" Decode.int) encoded
                            |> Expect.equal (Ok 1)
                    , \_ ->
                        -- comment フィールドが存在しないことを確認
                        Decode.decodeString (Decode.field "comment" Decode.string) encoded
                            |> Expect.err
                    ]
                    ()
        ]
