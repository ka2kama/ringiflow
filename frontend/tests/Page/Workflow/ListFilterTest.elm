module Page.Workflow.ListFilterTest exposing (suite)

{-| 申請一覧のフィルタロジックテスト

`completedToday` フィルタと `status` フィルタの
優先順位・組み合わせ挙動を検証する（Issue #267）。

-}

import Data.WorkflowInstance exposing (Status(..), WorkflowInstance)
import Expect
import Iso8601
import Json.Encode as Encode
import Page.Workflow.List as WorkflowList
import Route
import Test exposing (..)
import Time


suite : Test
suite =
    describe "WorkflowList filter"
        [ isCompletedTodayTests
        , filterWorkflowsTests
        ]



-- isCompletedToday


isCompletedTodayTests : Test
isCompletedTodayTests =
    describe "isCompletedToday"
        [ test "Approved + 今日の updatedAt → True" <|
            \_ ->
                let
                    now =
                        unsafeToTime "2026-02-06T12:00:00Z"

                    workflow =
                        makeWorkflow Approved "2026-02-06T10:00:00Z"
                in
                WorkflowList.isCompletedToday Time.utc now workflow
                    |> Expect.equal True
        , test "Approved + 昨日の updatedAt → False" <|
            \_ ->
                let
                    now =
                        unsafeToTime "2026-02-06T12:00:00Z"

                    workflow =
                        makeWorkflow Approved "2026-02-05T23:00:00Z"
                in
                WorkflowList.isCompletedToday Time.utc now workflow
                    |> Expect.equal False
        , test "InProgress + 今日の updatedAt → False（Approved 以外）" <|
            \_ ->
                let
                    now =
                        unsafeToTime "2026-02-06T12:00:00Z"

                    workflow =
                        makeWorkflow InProgress "2026-02-06T10:00:00Z"
                in
                WorkflowList.isCompletedToday Time.utc now workflow
                    |> Expect.equal False
        ]



-- filterWorkflows


filterWorkflowsTests : Test
filterWorkflowsTests =
    describe "filterWorkflows"
        [ test "completedToday=True → 今日の Approved のみ" <|
            \_ ->
                let
                    now =
                        unsafeToTime "2026-02-06T12:00:00Z"

                    filter =
                        { status = Nothing, completedToday = True }

                    workflows =
                        [ makeWorkflow Approved "2026-02-06T10:00:00Z"
                        , makeWorkflow Approved "2026-02-05T23:00:00Z"
                        , makeWorkflow InProgress "2026-02-06T10:00:00Z"
                        ]
                in
                WorkflowList.filterWorkflows Time.utc (Just now) filter workflows
                    |> List.length
                    |> Expect.equal 1
        , test "completedToday=True, status=Draft → completedToday 優先（status 無視）" <|
            \_ ->
                let
                    now =
                        unsafeToTime "2026-02-06T12:00:00Z"

                    filter =
                        { status = Just Draft, completedToday = True }

                    workflows =
                        [ makeWorkflow Approved "2026-02-06T10:00:00Z" ]
                in
                WorkflowList.filterWorkflows Time.utc (Just now) filter workflows
                    |> List.length
                    |> Expect.equal 1
        , test "completedToday=False, status=InProgress → status フィルタ適用" <|
            \_ ->
                let
                    filter =
                        { status = Just InProgress, completedToday = False }

                    workflows =
                        [ makeWorkflow InProgress "2026-02-06T10:00:00Z"
                        , makeWorkflow Approved "2026-02-06T10:00:00Z"
                        ]
                in
                WorkflowList.filterWorkflows Time.utc Nothing filter workflows
                    |> List.length
                    |> Expect.equal 1
        , test "completedToday=False, status=Nothing → 全件" <|
            \_ ->
                let
                    filter =
                        Route.emptyWorkflowFilter

                    workflows =
                        [ makeWorkflow InProgress "2026-02-06T10:00:00Z"
                        , makeWorkflow Approved "2026-02-06T10:00:00Z"
                        ]
                in
                WorkflowList.filterWorkflows Time.utc Nothing filter workflows
                    |> List.length
                    |> Expect.equal 2
        , test "completedToday=True, now=Nothing → 全件（フィルタ不能）" <|
            \_ ->
                let
                    filter =
                        { status = Nothing, completedToday = True }

                    workflows =
                        [ makeWorkflow Approved "2026-02-06T10:00:00Z"
                        , makeWorkflow InProgress "2026-02-06T10:00:00Z"
                        ]
                in
                WorkflowList.filterWorkflows Time.utc Nothing filter workflows
                    |> List.length
                    |> Expect.equal 2
        ]



-- Helpers


{-| ISO 8601 文字列を Time.Posix に変換（テスト専用）

テストデータ作成用。パース失敗時は epoch 0 にフォールバック。

-}
unsafeToTime : String -> Time.Posix
unsafeToTime isoString =
    case Iso8601.toTime isoString of
        Ok posix ->
            posix

        Err _ ->
            Time.millisToPosix 0


{-| テスト用の最小限の WorkflowInstance を作成

フィルタロジックに必要な `status` と `updatedAt` のみ可変。

-}
makeWorkflow : Status -> String -> WorkflowInstance
makeWorkflow status updatedAt =
    { id = "test-id"
    , displayId = "WF-1"
    , displayNumber = 1
    , title = "Test Workflow"
    , definitionId = "def-1"
    , status = status
    , version = 1
    , formData = Encode.object []
    , initiatedBy = { id = "user-1", name = "Test User" }
    , currentStepId = Nothing
    , steps = []
    , submittedAt = Nothing
    , createdAt = "2026-01-01T00:00:00Z"
    , updatedAt = updatedAt
    }
