module Data.WorkflowInstanceTest exposing (suite)

{-| Data.WorkflowInstance モジュールのテスト

ステータス変換とJSONデコーダーの正確性を検証する。

-}

import Data.WorkflowInstance as WorkflowInstance exposing (Decision(..), Status(..), StepStatus(..))
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
        , stepStatusToCssClassTests
        , decisionToStringTests
        , decisionFromStringTests
        , decoderTests
        , detailDecoderTests
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
        , test "ChangesRequested → \"ChangesRequested\"" <|
            \_ ->
                WorkflowInstance.statusToString ChangesRequested
                    |> Expect.equal "ChangesRequested"
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
        , test "\"ChangesRequested\" → Just ChangesRequested" <|
            \_ ->
                WorkflowInstance.statusFromString "ChangesRequested"
                    |> Expect.equal (Just ChangesRequested)
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
        , test "ChangesRequested → 差し戻し" <|
            \_ ->
                WorkflowInstance.statusToJapanese ChangesRequested
                    |> Expect.equal "差し戻し"
        ]



-- statusToCssClass


statusToCssClassTests : Test
statusToCssClassTests =
    describe "statusToCssClass"
        [ test "Draft → Tailwind secondary classes with border" <|
            \_ ->
                WorkflowInstance.statusToCssClass Draft
                    |> Expect.equal "bg-secondary-100 text-secondary-600 border-secondary-200"
        , test "Pending → Tailwind warning classes with border" <|
            \_ ->
                WorkflowInstance.statusToCssClass Pending
                    |> Expect.equal "bg-warning-50 text-warning-600 border-warning-200"
        , test "InProgress → Tailwind info classes with border" <|
            \_ ->
                WorkflowInstance.statusToCssClass InProgress
                    |> Expect.equal "bg-info-50 text-info-600 border-info-300"
        , test "Approved → Tailwind success classes with border" <|
            \_ ->
                WorkflowInstance.statusToCssClass Approved
                    |> Expect.equal "bg-success-50 text-success-600 border-success-200"
        , test "Rejected → Tailwind error classes with border" <|
            \_ ->
                WorkflowInstance.statusToCssClass Rejected
                    |> Expect.equal "bg-error-50 text-error-600 border-error-200"
        , test "Cancelled → Tailwind secondary classes with border" <|
            \_ ->
                WorkflowInstance.statusToCssClass Cancelled
                    |> Expect.equal "bg-secondary-100 text-secondary-500 border-secondary-200"
        , test "ChangesRequested → Tailwind warning classes with border" <|
            \_ ->
                WorkflowInstance.statusToCssClass ChangesRequested
                    |> Expect.equal "bg-warning-50 text-warning-600 border-warning-200"
        ]



-- stepStatusToCssClass


stepStatusToCssClassTests : Test
stepStatusToCssClassTests =
    describe "stepStatusToCssClass"
        [ test "StepPending → Tailwind secondary classes with border" <|
            \_ ->
                WorkflowInstance.stepStatusToCssClass StepPending
                    |> Expect.equal "bg-secondary-100 text-secondary-600 border-secondary-200"
        , test "StepActive → Tailwind warning classes with border" <|
            \_ ->
                WorkflowInstance.stepStatusToCssClass StepActive
                    |> Expect.equal "bg-warning-50 text-warning-600 border-warning-200"
        , test "StepCompleted → Tailwind success classes with border" <|
            \_ ->
                WorkflowInstance.stepStatusToCssClass StepCompleted
                    |> Expect.equal "bg-success-50 text-success-600 border-success-200"
        , test "StepSkipped → Tailwind secondary classes with border" <|
            \_ ->
                WorkflowInstance.stepStatusToCssClass StepSkipped
                    |> Expect.equal "bg-secondary-100 text-secondary-500 border-secondary-200"
        ]



-- decisionToString


decisionToStringTests : Test
decisionToStringTests =
    describe "decisionToString"
        [ test "DecisionApproved → \"Approved\"" <|
            \_ ->
                WorkflowInstance.decisionToString DecisionApproved
                    |> Expect.equal "Approved"
        , test "DecisionRejected → \"Rejected\"" <|
            \_ ->
                WorkflowInstance.decisionToString DecisionRejected
                    |> Expect.equal "Rejected"
        , test "DecisionRequestChanges → \"RequestChanges\"" <|
            \_ ->
                WorkflowInstance.decisionToString DecisionRequestChanges
                    |> Expect.equal "RequestChanges"
        ]



-- decisionFromString


decisionFromStringTests : Test
decisionFromStringTests =
    describe "decisionFromString"
        [ test "\"Approved\" → Just DecisionApproved" <|
            \_ ->
                WorkflowInstance.decisionFromString "Approved"
                    |> Expect.equal (Just DecisionApproved)
        , test "\"Rejected\" → Just DecisionRejected" <|
            \_ ->
                WorkflowInstance.decisionFromString "Rejected"
                    |> Expect.equal (Just DecisionRejected)
        , test "\"RequestChanges\" → Just DecisionRequestChanges" <|
            \_ ->
                WorkflowInstance.decisionFromString "RequestChanges"
                    |> Expect.equal (Just DecisionRequestChanges)
        , test "未知の文字列 → Nothing" <|
            \_ ->
                WorkflowInstance.decisionFromString "Unknown"
                    |> Expect.equal Nothing
        , test "decisionToString >> decisionFromString の往復" <|
            \_ ->
                [ DecisionApproved, DecisionRejected, DecisionRequestChanges ]
                    |> List.map (\d -> WorkflowInstance.decisionToString d |> WorkflowInstance.decisionFromString)
                    |> Expect.equal
                        [ Just DecisionApproved
                        , Just DecisionRejected
                        , Just DecisionRequestChanges
                        ]
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
                            "display_id": "WF-1",
                            "display_number": 1,
                            "title": "経費精算申請",
                            "definition_id": "def-001",
                            "status": "Draft",
                            "form_data": {"amount": 10000},
                            "initiated_by": {"id": "user-001", "name": "テストユーザー1"},
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
                            , displayId = i.displayId
                            , displayNumber = i.displayNumber
                            , title = i.title
                            , status = i.status
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "inst-001"
                            , displayId = "WF-1"
                            , displayNumber = 1
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
                            "display_id": "WF-2",
                            "display_number": 2,
                            "title": "休暇申請",
                            "definition_id": "def-002",
                            "status": "Pending",
                            "form_data": {},
                            "initiated_by": {"id": "user-002", "name": "テストユーザー2"},
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
                            "display_id": "WF-3",
                            "display_number": 3,
                            "title": "購買申請",
                            "definition_id": "def-003",
                            "status": "Approved",
                            "form_data": {},
                            "initiated_by": {"id": "user-003", "name": "テストユーザー3"},
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
                            "display_id": "WF-1",
                            "display_number": 1,
                            "title": "テスト",
                            "definition_id": "def-001",
                            "status": \""""
                            ++ status
                            ++ """",
                            "form_data": {},
                            "initiated_by": {"id": "user-001", "name": "テストユーザー1"},
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
                , decodeStatus "ChangesRequested"
                ]
                    |> Expect.equal
                        [ Ok Draft
                        , Ok Pending
                        , Ok InProgress
                        , Ok Approved
                        , Ok Rejected
                        , Ok Cancelled
                        , Ok ChangesRequested
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
                            "initiated_by": {"id": "user-001", "name": "テストユーザー1"},
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



-- detailDecoder


detailDecoderTests : Test
detailDecoderTests =
    describe "detailDecoder"
        [ test "data フィールドから単一インスタンスをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "id": "inst-001",
                                "display_id": "WF-1",
                                "display_number": 1,
                                "title": "経費精算申請",
                                "definition_id": "def-001",
                                "status": "Draft",
                                "form_data": {},
                                "initiated_by": {"id": "user-001", "name": "テストユーザー1"},
                                "created_at": "2026-01-01T00:00:00Z",
                                "updated_at": "2026-01-01T00:00:00Z"
                            }
                        }
                        """
                in
                Decode.decodeString WorkflowInstance.detailDecoder json
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
        , test "data フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "inst-001",
                            "display_id": "WF-1",
                            "display_number": 1,
                            "title": "経費精算申請",
                            "definition_id": "def-001",
                            "status": "Draft",
                            "form_data": {},
                            "initiated_by": {"id": "user-001", "name": "テストユーザー1"},
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowInstance.detailDecoder json
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
                                    "display_id": "WF-1",
                                    "display_number": 1,
                                    "title": "経費精算",
                                    "definition_id": "def-001",
                                    "status": "Draft",
                                    "form_data": {},
                                    "initiated_by": {"id": "user-001", "name": "テストユーザー1"},
                                    "created_at": "2026-01-01T00:00:00Z",
                                    "updated_at": "2026-01-01T00:00:00Z"
                                },
                                {
                                    "id": "inst-002",
                                    "display_id": "WF-2",
                                    "display_number": 2,
                                    "title": "休暇申請",
                                    "definition_id": "def-002",
                                    "status": "Approved",
                                    "form_data": {},
                                    "initiated_by": {"id": "user-002", "name": "テストユーザー2"},
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
