module Page.Workflow.NewTest exposing (suite)

{-| Page.Workflow.New の update ロジックテスト

init → update の Model 変更を検証する。
バリデーション、キーボード操作、dirty 状態管理をカバー。

-}

import Component.ApproverSelector as ApproverSelector exposing (ApproverSelection(..))
import Data.UserItem exposing (UserItem)
import Data.WorkflowInstance exposing (Status(..))
import Dict
import Expect
import Json.Encode as Encode
import Page.Workflow.New as New exposing (Msg(..), SaveMessage(..))
import RemoteData exposing (RemoteData(..))
import Shared
import Test exposing (..)


suite : Test
suite =
    describe "Page.Workflow.New"
        [ saveDraftTests
        , submitTests
        , approverKeyboardTests
        , dirtyStateTests
        ]



-- ────────────────────────────────────
-- テストヘルパー
-- ────────────────────────────────────


{-| テスト用の初期 Model を生成（Cmd は破棄）
-}
initialModel : New.Model
initialModel =
    let
        shared =
            Shared.init { apiBaseUrl = "", timezoneOffsetMinutes = 540 }
    in
    New.init shared |> Tuple.first


{-| テスト用ユーザー
-}
testUser1 : UserItem
testUser1 =
    { id = "u-001"
    , displayId = "U-1"
    , displayNumber = 1
    , name = "山田太郎"
    , email = "yamada@example.com"
    }


testUser2 : UserItem
testUser2 =
    { id = "u-002"
    , displayId = "U-2"
    , displayNumber = 2
    , name = "山田次郎"
    , email = "yamada2@example.com"
    }


{-| テスト用 WorkflowInstance（GotSaveResult 用）
-}
testWorkflowInstance : Data.WorkflowInstance.WorkflowInstance
testWorkflowInstance =
    { id = "wf-001"
    , displayId = "WF-1"
    , displayNumber = 1
    , title = "テスト申請"
    , definitionId = "def-001"
    , status = Draft
    , version = 1
    , formData = Encode.object []
    , initiatedBy = { id = "u-001", name = "山田太郎" }
    , currentStepId = Nothing
    , steps = []
    , submittedAt = Nothing
    , createdAt = "2026-01-01T00:00:00Z"
    , updatedAt = "2026-01-01T00:00:00Z"
    }


{-| テスト用のステップ ID
-}
testStepId : String
testStepId =
    "step-1"


{-| 承認者ステップが1つある Model を構築するヘルパー
-}
modelWithStep : New.Model -> New.Model
modelWithStep model =
    { model
        | approvers = Dict.singleton testStepId ApproverSelector.init
    }



-- ────────────────────────────────────
-- SaveDraft バリデーション
-- ────────────────────────────────────


saveDraftTests : Test
saveDraftTests =
    describe "SaveDraft"
        [ test "定義未選択でエラーメッセージ" <|
            \_ ->
                let
                    sut =
                        New.update SaveDraft initialModel |> Tuple.first
                in
                sut.saveMessage
                    |> Expect.equal (Just (SaveError "ワークフロー種類を選択してください"))
        , test "タイトル空でバリデーションエラー" <|
            \_ ->
                let
                    model =
                        { initialModel | selectedDefinitionId = Just "def-001" }

                    sut =
                        New.update SaveDraft model |> Tuple.first
                in
                Dict.member "title" sut.validationErrors
                    |> Expect.equal True
        , test "定義選択済み + タイトル入力済みで submitting = True" <|
            \_ ->
                let
                    model =
                        { initialModel
                            | selectedDefinitionId = Just "def-001"
                            , title = "テスト申請"
                        }

                    sut =
                        New.update SaveDraft model |> Tuple.first
                in
                sut.submitting
                    |> Expect.equal True
        ]



-- ────────────────────────────────────
-- Submit バリデーション
-- ────────────────────────────────────


submitTests : Test
submitTests =
    describe "Submit"
        [ test "承認者未選択でバリデーションエラー" <|
            \_ ->
                let
                    model =
                        { initialModel
                            | selectedDefinitionId = Just "def-001"
                            , title = "テスト申請"
                        }
                            |> modelWithStep

                    sut =
                        New.update Submit model |> Tuple.first
                in
                Dict.member ("approver_" ++ testStepId) sut.validationErrors
                    |> Expect.equal True
        , test "タイトル空 + 承認者未選択で複数エラー" <|
            \_ ->
                let
                    model =
                        { initialModel
                            | selectedDefinitionId = Just "def-001"
                        }
                            |> modelWithStep

                    sut =
                        New.update Submit model |> Tuple.first
                in
                Expect.all
                    [ \m -> Dict.member "title" m.validationErrors |> Expect.equal True
                    , \m -> Dict.member ("approver_" ++ testStepId) m.validationErrors |> Expect.equal True
                    ]
                    sut
        ]



-- ────────────────────────────────────
-- 承認者キーボード操作
-- ────────────────────────────────────


approverKeyboardTests : Test
approverKeyboardTests =
    let
        approverState =
            let
                s =
                    ApproverSelector.init
            in
            { s
                | search = "山田"
                , dropdownOpen = True
                , highlightIndex = 0
            }

        modelWithUsers =
            { initialModel
                | users = Success [ testUser1, testUser2 ]
                , approvers = Dict.singleton testStepId approverState
            }

        getApproverState model =
            Dict.get testStepId model.approvers
                |> Maybe.withDefault ApproverSelector.init
    in
    describe "ApproverKeyDown"
        [ test "ArrowDown でインデックス増加" <|
            \_ ->
                let
                    sut =
                        New.update (ApproverKeyDown testStepId "ArrowDown") modelWithUsers |> Tuple.first
                in
                (getApproverState sut).highlightIndex
                    |> Expect.equal 1
        , test "ArrowUp でインデックス循環（0 → 末尾）" <|
            \_ ->
                let
                    sut =
                        New.update (ApproverKeyDown testStepId "ArrowUp") modelWithUsers |> Tuple.first
                in
                -- index 0 → modBy 2 (0 - 1 + 2) = 1（末尾に循環）
                (getApproverState sut).highlightIndex
                    |> Expect.equal 1
        , test "Enter で候補選択" <|
            \_ ->
                let
                    sut =
                        New.update (ApproverKeyDown testStepId "Enter") modelWithUsers |> Tuple.first
                in
                (getApproverState sut).selection
                    |> Expect.equal (Selected testUser1)
        , test "Escape でドロップダウン閉じる" <|
            \_ ->
                let
                    sut =
                        New.update (ApproverKeyDown testStepId "Escape") modelWithUsers |> Tuple.first
                in
                (getApproverState sut).dropdownOpen
                    |> Expect.equal False
        ]



-- ────────────────────────────────────
-- Dirty 状態管理
-- ────────────────────────────────────


dirtyStateTests : Test
dirtyStateTests =
    describe "Dirty 状態"
        [ test "初期状態で isDirty = False" <|
            \_ ->
                New.isDirty initialModel
                    |> Expect.equal False
        , test "UpdateTitle で isDirty = True" <|
            \_ ->
                let
                    sut =
                        New.update (UpdateTitle "テスト") initialModel |> Tuple.first
                in
                New.isDirty sut
                    |> Expect.equal True
        , test "GotSaveResult (Ok ...) で isDirty = False" <|
            \_ ->
                let
                    dirtyModel =
                        New.update (UpdateTitle "テスト") initialModel |> Tuple.first

                    sut =
                        New.update (GotSaveResult (Ok testWorkflowInstance)) dirtyModel |> Tuple.first
                in
                New.isDirty sut
                    |> Expect.equal False
        ]
