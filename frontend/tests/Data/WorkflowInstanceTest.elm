module Data.WorkflowInstanceTest exposing (suite)

{-| Data.WorkflowInstance モジュールのテスト

ステータス変換とJSONデコーダーの正確性を検証する。

-}

import Data.WorkflowInstance as WorkflowInstance exposing (Status(..))
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.WorkflowInstance"
        [ statusToStringTests
        , statusFromStringTests
        , statusToJapaneseTests
        , statusToCssClassTests
        , decoderTests
        , listDecoderTests
        ]



-- statusToString


statusToStringTests : Test
statusToStringTests =
    describe "statusToString"
        [ test "Draft → \"Draft\"" <|
            \_ ->
                WorkflowInstance.statusToString Draft
                    |> Expect.equal "Draft"
        , test "Pending → \"Pending\"" <|
            \_ ->
                WorkflowInstance.statusToString Pending
                    |> Expect.equal "Pending"
        , test "InProgress → \"InProgress\"" <|
            \_ ->
                WorkflowInstance.statusToString InProgress
                    |> Expect.equal "InProgress"
        , test "Approved → \"Approved\"" <|
            \_ ->
                WorkflowInstance.statusToString Approved
                    |> Expect.equal "Approved"
        , test "Rejected → \"Rejected\"" <|
            \_ ->
                WorkflowInstance.statusToString Rejected
                    |> Expect.equal "Rejected"
        , test "Cancelled → \"Cancelled\"" <|
            \_ ->
                WorkflowInstance.statusToString Cancelled
                    |> Expect.equal "Cancelled"
        ]



-- statusFromString


statusFromStringTests : Test
statusFromStringTests =
    describe "statusFromString"
        [ test "\"Draft\" → Just Draft" <|
            \_ ->
                WorkflowInstance.statusFromString "Draft"
                    |> Expect.equal (Just Draft)
        , test "\"Pending\" → Just Pending" <|
            \_ ->
                WorkflowInstance.statusFromString "Pending"
                    |> Expect.equal (Just Pending)
        , test "\"InProgress\" → Just InProgress" <|
            \_ ->
                WorkflowInstance.statusFromString "InProgress"
                    |> Expect.equal (Just InProgress)
        , test "\"Approved\" → Just Approved" <|
            \_ ->
                WorkflowInstance.statusFromString "Approved"
                    |> Expect.equal (Just Approved)
        , test "\"Rejected\" → Just Rejected" <|
            \_ ->
                WorkflowInstance.statusFromString "Rejected"
                    |> Expect.equal (Just Rejected)
        , test "\"Cancelled\" → Just Cancelled" <|
            \_ ->
                WorkflowInstance.statusFromString "Cancelled"
                    |> Expect.equal (Just Cancelled)
        , test "未知の文字列 → Nothing" <|
            \_ ->
                WorkflowInstance.statusFromString "Unknown"
                    |> Expect.equal Nothing
        , test "小文字は無効 → Nothing" <|
            \_ ->
                WorkflowInstance.statusFromString "draft"
                    |> Expect.equal Nothing
        ]



-- statusToJapanese


statusToJapaneseTests : Test
statusToJapaneseTests =
    describe "statusToJapanese"
        [ test "Draft → 下書き" <|
            \_ ->
                WorkflowInstance.statusToJapanese Draft
                    |> Expect.equal "下書き"
        , test "Pending → 申請待ち" <|
            \_ ->
                WorkflowInstance.statusToJapanese Pending
                    |> Expect.equal "申請待ち"
        , test "InProgress → 承認中" <|
            \_ ->
                WorkflowInstance.statusToJapanese InProgress
                    |> Expect.equal "承認中"
        , test "Approved → 承認済み" <|
            \_ ->
                WorkflowInstance.statusToJapanese Approved
                    |> Expect.equal "承認済み"
        , test "Rejected → 却下" <|
            \_ ->
                WorkflowInstance.statusToJapanese Rejected
                    |> Expect.equal "却下"
        , test "Cancelled → キャンセル" <|
            \_ ->
                WorkflowInstance.statusToJapanese Cancelled
                    |> Expect.equal "キャンセル"
        ]



-- statusToCssClass


statusToCssClassTests : Test
statusToCssClassTests =
    describe "statusToCssClass"
        [ test "Draft → Tailwind gray classes" <|
            \_ ->
                WorkflowInstance.statusToCssClass Draft
                    |> Expect.equal "bg-gray-100 text-gray-600"
        , test "Pending → Tailwind warning classes" <|
            \_ ->
                WorkflowInstance.statusToCssClass Pending
                    |> Expect.equal "bg-warning-50 text-warning-600"
        , test "InProgress → Tailwind info classes" <|
            \_ ->
                WorkflowInstance.statusToCssClass InProgress
                    |> Expect.equal "bg-info-50 text-info-600"
        , test "Approved → Tailwind success classes" <|
            \_ ->
                WorkflowInstance.statusToCssClass Approved
                    |> Expect.equal "bg-success-50 text-success-600"
        , test "Rejected → Tailwind error classes" <|
            \_ ->
                WorkflowInstance.statusToCssClass Rejected
                    |> Expect.equal "bg-error-50 text-error-600"
        , test "Cancelled → Tailwind secondary classes" <|
            \_ ->
                WorkflowInstance.statusToCssClass Cancelled
                    |> Expect.equal "bg-secondary-100 text-secondary-500"
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
                            "id": "inst-001",
                            "title": "経費精算申請",
                            "definition_id": "def-001",
                            "status": "Draft",
                            "form_data": {"amount": 10000},
                            "initiated_by": "user-001",
                            "current_step_id": "step-1",
                            "submitted_at": "2026-01-15T10:00:00Z",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowInstance.decoder json
                    |> Result.map
                        (\i ->
                            { id = i.id
                            , title = i.title
                            , status = i.status
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "inst-001"
                            , title = "経費精算申請"
                            , status = Draft
                            }
                        )
        , test "オプショナルフィールドが null の場合" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "inst-002",
                            "title": "休暇申請",
                            "definition_id": "def-002",
                            "status": "Pending",
                            "form_data": {},
                            "initiated_by": "user-002",
                            "current_step_id": null,
                            "submitted_at": null,
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowInstance.decoder json
                    |> Result.map
                        (\i ->
                            { currentStepId = i.currentStepId
                            , submittedAt = i.submittedAt
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { currentStepId = Nothing
                            , submittedAt = Nothing
                            }
                        )
        , test "オプショナルフィールドが省略された場合" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "inst-003",
                            "title": "購買申請",
                            "definition_id": "def-003",
                            "status": "Approved",
                            "form_data": {},
                            "initiated_by": "user-003",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowInstance.decoder json
                    |> Result.map
                        (\i ->
                            { currentStepId = i.currentStepId
                            , submittedAt = i.submittedAt
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { currentStepId = Nothing
                            , submittedAt = Nothing
                            }
                        )
        , test "各ステータスが正しくデコードされる" <|
            \_ ->
                let
                    makeJson status =
                        """
                        {
                            "id": "inst-001",
                            "title": "テスト",
                            "definition_id": "def-001",
                            "status": \""""
                            ++ status
                            ++ """",
                            "form_data": {},
                            "initiated_by": "user-001",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """

                    decodeStatus statusStr =
                        Decode.decodeString WorkflowInstance.decoder (makeJson statusStr)
                            |> Result.map .status
                in
                [ decodeStatus "Draft"
                , decodeStatus "Pending"
                , decodeStatus "InProgress"
                , decodeStatus "Approved"
                , decodeStatus "Rejected"
                , decodeStatus "Cancelled"
                ]
                    |> Expect.equal
                        [ Ok Draft
                        , Ok Pending
                        , Ok InProgress
                        , Ok Approved
                        , Ok Rejected
                        , Ok Cancelled
                        ]
        , test "未知のステータスはエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "inst-001",
                            "title": "テスト",
                            "definition_id": "def-001",
                            "status": "Unknown",
                            "form_data": {},
                            "initiated_by": "user-001",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowInstance.decoder json
                    |> Expect.err
        , test "必須フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "inst-001"
                        }
                        """
                in
                Decode.decodeString WorkflowInstance.decoder json
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
                                    "id": "inst-001",
                                    "title": "経費精算",
                                    "definition_id": "def-001",
                                    "status": "Draft",
                                    "form_data": {},
                                    "initiated_by": "user-001",
                                    "created_at": "2026-01-01T00:00:00Z",
                                    "updated_at": "2026-01-01T00:00:00Z"
                                },
                                {
                                    "id": "inst-002",
                                    "title": "休暇申請",
                                    "definition_id": "def-002",
                                    "status": "Approved",
                                    "form_data": {},
                                    "initiated_by": "user-002",
                                    "created_at": "2026-01-01T00:00:00Z",
                                    "updated_at": "2026-01-01T00:00:00Z"
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString WorkflowInstance.listDecoder json
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
                Decode.decodeString WorkflowInstance.listDecoder json
                    |> Expect.equal (Ok [])
        , test "data フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        []
                        """
                in
                Decode.decodeString WorkflowInstance.listDecoder json
                    |> Expect.err
        ]
