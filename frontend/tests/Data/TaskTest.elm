module Data.TaskTest exposing (suite)

{-| Data.Task のデコーダテスト

タスクデータ型（TaskItem, TaskDetail, WorkflowSummary）の
JSON デコーダが正しく動作することを検証する。

-}

import Data.Task as Task
import Data.WorkflowInstance exposing (StepStatus(..))
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.Task"
        [ workflowSummaryTests
        , taskItemTests
        , listDecoderTests
        , detailDecoderTests
        ]



-- ────────────────────────────────────
-- WorkflowSummary
-- ────────────────────────────────────


workflowSummaryTests : Test
workflowSummaryTests =
    describe "workflowSummaryDecoder"
        [ test "全フィールドをデコード（initiatedBy のネスト含む）" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "wf-001",
                            "display_id": "WF-1",
                            "display_number": 1,
                            "title": "テスト申請",
                            "status": "Pending",
                            "initiated_by": {
                                "id": "user-001",
                                "name": "山田太郎"
                            },
                            "submitted_at": "2026-01-15T10:00:00Z"
                        }
                        """
                in
                Decode.decodeString Task.workflowSummaryDecoder json
                    |> Result.map
                        (\s ->
                            { id = s.id
                            , displayId = s.displayId
                            , displayNumber = s.displayNumber
                            , title = s.title
                            , status = s.status
                            , initiatedByName = s.initiatedBy.name
                            , submittedAt = s.submittedAt
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "wf-001"
                            , displayId = "WF-1"
                            , displayNumber = 1
                            , title = "テスト申請"
                            , status = "Pending"
                            , initiatedByName = "山田太郎"
                            , submittedAt = Just "2026-01-15T10:00:00Z"
                            }
                        )
        , test "submitted_at が null の場合 Nothing" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "wf-001",
                            "display_id": "WF-1",
                            "display_number": 1,
                            "title": "下書き",
                            "status": "Draft",
                            "initiated_by": {
                                "id": "user-001",
                                "name": "山田太郎"
                            },
                            "submitted_at": null
                        }
                        """
                in
                Decode.decodeString Task.workflowSummaryDecoder json
                    |> Result.map .submittedAt
                    |> Expect.equal (Ok Nothing)
        , test "必須フィールド欠落でエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "wf-001",
                            "display_id": "WF-1"
                        }
                        """
                in
                Decode.decodeString Task.workflowSummaryDecoder json
                    |> Expect.err
        ]



-- ────────────────────────────────────
-- TaskItem
-- ────────────────────────────────────


taskItemTests : Test
taskItemTests =
    describe "taskItemDecoder"
        [ test "全フィールドをデコード（ネストされた WorkflowSummary 含む）" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "step-001",
                            "display_number": 1,
                            "step_name": "部長承認",
                            "status": "Pending",
                            "version": 2,
                            "assigned_to": {
                                "id": "user-002",
                                "name": "鈴木一郎"
                            },
                            "due_date": "2026-02-01",
                            "started_at": "2026-01-20T09:00:00Z",
                            "created_at": "2026-01-15T10:00:00Z",
                            "workflow": {
                                "id": "wf-001",
                                "display_id": "WF-1",
                                "display_number": 1,
                                "title": "テスト申請",
                                "status": "InProgress",
                                "initiated_by": {
                                    "id": "user-001",
                                    "name": "山田太郎"
                                },
                                "submitted_at": "2026-01-15T10:00:00Z"
                            }
                        }
                        """
                in
                Decode.decodeString Task.taskItemDecoder json
                    |> Result.map
                        (\t ->
                            { id = t.id
                            , stepName = t.stepName
                            , status = t.status
                            , version = t.version
                            , assignedToName = Maybe.map .name t.assignedTo
                            , workflowTitle = t.workflow.title
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "step-001"
                            , stepName = "部長承認"
                            , status = StepPending
                            , version = 2
                            , assignedToName = Just "鈴木一郎"
                            , workflowTitle = "テスト申請"
                            }
                        )
        , test "optional フィールドが null の場合" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "step-001",
                            "display_number": 1,
                            "step_name": "承認",
                            "status": "Pending",
                            "version": 1,
                            "assigned_to": null,
                            "due_date": null,
                            "started_at": null,
                            "created_at": "2026-01-15T10:00:00Z",
                            "workflow": {
                                "id": "wf-001",
                                "display_id": "WF-1",
                                "display_number": 1,
                                "title": "テスト",
                                "status": "Pending",
                                "initiated_by": {
                                    "id": "user-001",
                                    "name": "山田太郎"
                                }
                            }
                        }
                        """
                in
                Decode.decodeString Task.taskItemDecoder json
                    |> Result.map
                        (\t ->
                            { assignedTo = t.assignedTo
                            , dueDate = t.dueDate
                            , startedAt = t.startedAt
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { assignedTo = Nothing
                            , dueDate = Nothing
                            , startedAt = Nothing
                            }
                        )
        , test "version 省略時のデフォルト値" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "step-001",
                            "display_number": 1,
                            "step_name": "承認",
                            "status": "Pending",
                            "created_at": "2026-01-15T10:00:00Z",
                            "workflow": {
                                "id": "wf-001",
                                "display_id": "WF-1",
                                "display_number": 1,
                                "title": "テスト",
                                "status": "Pending",
                                "initiated_by": {
                                    "id": "user-001",
                                    "name": "山田太郎"
                                }
                            }
                        }
                        """
                in
                Decode.decodeString Task.taskItemDecoder json
                    |> Result.map .version
                    |> Expect.equal (Ok 1)
        ]



-- ────────────────────────────────────
-- listDecoder
-- ────────────────────────────────────


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
                                    "id": "step-001",
                                    "display_number": 1,
                                    "step_name": "承認",
                                    "status": "Pending",
                                    "version": 1,
                                    "created_at": "2026-01-15T10:00:00Z",
                                    "workflow": {
                                        "id": "wf-001",
                                        "display_id": "WF-1",
                                        "display_number": 1,
                                        "title": "テスト",
                                        "status": "Pending",
                                        "initiated_by": {
                                            "id": "user-001",
                                            "name": "山田太郎"
                                        }
                                    }
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString Task.listDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 1)
        , test "空の一覧をデコード" <|
            \_ ->
                Decode.decodeString Task.listDecoder """{ "data": [] }"""
                    |> Result.map List.length
                    |> Expect.equal (Ok 0)
        ]



-- ────────────────────────────────────
-- detailDecoder
-- ────────────────────────────────────


detailDecoderTests : Test
detailDecoderTests =
    describe "detailDecoder"
        [ test "step と workflow をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "step": {
                                    "id": "step-001",
                                    "display_id": "STEP-1",
                                    "display_number": 1,
                                    "step_name": "部長承認",
                                    "status": "Pending",
                                    "version": 1,
                                    "assigned_to": {
                                        "id": "user-002",
                                        "name": "鈴木一郎"
                                    },
                                    "decision": null,
                                    "comment": null
                                },
                                "workflow": {
                                    "id": "wf-001",
                                    "display_id": "WF-1",
                                    "display_number": 1,
                                    "title": "テスト申請",
                                    "definition_id": "def-001",
                                    "status": "InProgress",
                                    "version": 1,
                                    "form_data": {},
                                    "initiated_by": {
                                        "id": "user-001",
                                        "name": "山田太郎"
                                    },
                                    "steps": [],
                                    "submitted_at": "2026-01-15T10:00:00Z",
                                    "created_at": "2026-01-15T10:00:00Z",
                                    "updated_at": "2026-01-15T10:00:00Z"
                                }
                            }
                        }
                        """
                in
                Decode.decodeString Task.detailDecoder json
                    |> Result.map
                        (\d ->
                            { stepName = d.step.stepName
                            , workflowTitle = d.workflow.title
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { stepName = "部長承認"
                            , workflowTitle = "テスト申請"
                            }
                        )
        ]
