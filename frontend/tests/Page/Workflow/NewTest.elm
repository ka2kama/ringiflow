module Page.Workflow.NewTest exposing (suite)

{-| Page.Workflow.New の update ロジックテスト

メッセージ経由のモデル構築（行動的アプローチ）で検証する。
ADT ステートマシン化により、状態遷移・バリデーション・dirty 管理をカバー。

-}

import Api exposing (ApiError(..))
import Component.ApproverSelector exposing (ApproverSelection(..))
import Data.UserItem exposing (UserItem)
import Data.WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance exposing (Status(..))
import Dict
import Expect exposing (Expectation)
import Json.Encode as Encode
import Page.Workflow.New as New exposing (FormState(..), Msg(..), PageState(..))
import Shared
import Test exposing (..)


suite : Test
suite =
    describe "Page.Workflow.New"
        [ stateTransitionTests
        , saveDraftTests
        , submitTests
        , approverKeyboardTests
        , dirtyStateTests
        ]



-- ────────────────────────────────────
-- テストヘルパー
-- ────────────────────────────────────


{-| メッセージを送信し、結果の Model を返す（Cmd は破棄）
-}
sendMsg : Msg -> New.Model -> New.Model
sendMsg msg model =
    New.update msg model |> Tuple.first


{-| テスト用の初期 Model を生成（Cmd は破棄）
-}
initialModel : New.Model
initialModel =
    let
        shared =
            Shared.init { apiBaseUrl = "", timezoneOffsetMinutes = 540 }
    in
    New.init shared |> Tuple.first


{-| テスト用ワークフロー定義（承認ステップ付き）
-}
testDefinition : WorkflowDefinition
testDefinition =
    { id = "def-001"
    , name = "テスト定義"
    , description = Nothing
    , version = 1
    , definition =
        Encode.object
            [ ( "steps"
              , Encode.list identity
                    [ Encode.object
                        [ ( "id", Encode.string "step-1" )
                        , ( "name", Encode.string "承認" )
                        , ( "type", Encode.string "approval" )
                        ]
                    ]
              )
            ]
    , status = "published"
    , createdBy = "u-001"
    , createdAt = "2026-01-01T00:00:00Z"
    , updatedAt = "2026-01-01T00:00:00Z"
    }


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


{-| ロード完了モデル（定義一覧表示、定義未選択）
-}
loadedModel : New.Model
loadedModel =
    initialModel |> sendMsg (GotDefinitions (Ok [ testDefinition ]))


{-| 編集中モデル（定義選択済み、承認ステップ初期化済み）
-}
editingModel : New.Model
editingModel =
    loadedModel |> sendMsg (SelectDefinition "def-001")


{-| Editing 状態を検証するアサーションヘルパー
-}
expectEditing : (New.EditingState -> Expectation) -> New.Model -> Expectation
expectEditing check model =
    case model.state of
        Loaded loaded ->
            case loaded.formState of
                Editing editing ->
                    check editing

                SelectingDefinition ->
                    Expect.fail "Expected Editing state, got SelectingDefinition"

        Failed _ ->
            Expect.fail "Expected Loaded state, got Failed"

        Loading ->
            Expect.fail "Expected Loaded state, got Loading"



-- ────────────────────────────────────
-- 状態遷移
-- ────────────────────────────────────


stateTransitionTests : Test
stateTransitionTests =
    describe "状態遷移"
        [ test "GotDefinitions Ok で Loaded.SelectingDefinition になる" <|
            \_ ->
                case loadedModel.state of
                    Loaded loaded ->
                        case loaded.formState of
                            SelectingDefinition ->
                                Expect.pass

                            Editing _ ->
                                Expect.fail "Expected SelectingDefinition, got Editing"

                    _ ->
                        Expect.fail "Expected Loaded state"
        , test "GotDefinitions Err で Failed になる" <|
            \_ ->
                let
                    sut =
                        initialModel
                            |> sendMsg (GotDefinitions (Err NetworkError))
                in
                case sut.state of
                    Failed _ ->
                        Expect.pass

                    _ ->
                        Expect.fail "Expected Failed state"
        , test "SelectDefinition で Editing になり approvers が初期化される" <|
            \_ ->
                editingModel
                    |> expectEditing
                        (\editing ->
                            Expect.all
                                [ \e -> e.selectedDefinition.id |> Expect.equal "def-001"
                                , \e -> Dict.member testStepId e.approvers |> Expect.equal True
                                ]
                                editing
                        )
        ]



-- ────────────────────────────────────
-- SaveDraft バリデーション
-- ────────────────────────────────────


saveDraftTests : Test
saveDraftTests =
    describe "SaveDraft"
        [ test "タイトル空でバリデーションエラー" <|
            \_ ->
                editingModel
                    |> sendMsg SaveDraft
                    |> expectEditing
                        (\editing ->
                            Dict.member "title" editing.validationErrors
                                |> Expect.equal True
                        )
        , test "タイトル入力済みで submitting = True" <|
            \_ ->
                editingModel
                    |> sendMsg (UpdateTitle "テスト申請")
                    |> sendMsg SaveDraft
                    |> expectEditing
                        (\editing ->
                            editing.submitting
                                |> Expect.equal True
                        )
        ]



-- ────────────────────────────────────
-- Submit バリデーション
-- ────────────────────────────────────


submitTests : Test
submitTests =
    describe "Submit"
        [ test "承認者未選択でバリデーションエラー" <|
            \_ ->
                editingModel
                    |> sendMsg (UpdateTitle "テスト申請")
                    |> sendMsg Submit
                    |> expectEditing
                        (\editing ->
                            Dict.member ("approver_" ++ testStepId) editing.validationErrors
                                |> Expect.equal True
                        )
        , test "タイトル空 + 承認者未選択で複数エラー" <|
            \_ ->
                editingModel
                    |> sendMsg Submit
                    |> expectEditing
                        (\editing ->
                            Expect.all
                                [ \e -> Dict.member "title" e.validationErrors |> Expect.equal True
                                , \e -> Dict.member ("approver_" ++ testStepId) e.validationErrors |> Expect.equal True
                                ]
                                editing
                        )
        ]



-- ────────────────────────────────────
-- 承認者キーボード操作
-- ────────────────────────────────────


approverKeyboardTests : Test
approverKeyboardTests =
    let
        -- ユーザー読み込み + 定義選択 + 承認者検索で準備完了
        modelForKeyboard =
            initialModel
                |> sendMsg (GotUsers (Ok [ testUser1, testUser2 ]))
                |> sendMsg (GotDefinitions (Ok [ testDefinition ]))
                |> sendMsg (SelectDefinition "def-001")
                |> sendMsg (UpdateApproverSearch testStepId "山田")

        getHighlightIndex editing =
            Dict.get testStepId editing.approvers
                |> Maybe.map .highlightIndex
                |> Maybe.withDefault -1

        getSelection editing =
            Dict.get testStepId editing.approvers
                |> Maybe.map .selection

        getDropdownOpen editing =
            Dict.get testStepId editing.approvers
                |> Maybe.map .dropdownOpen
                |> Maybe.withDefault False
    in
    describe "ApproverKeyDown"
        [ test "ArrowDown でインデックス増加" <|
            \_ ->
                modelForKeyboard
                    |> sendMsg (ApproverKeyDown testStepId "ArrowDown")
                    |> expectEditing
                        (\editing ->
                            getHighlightIndex editing
                                |> Expect.equal 1
                        )
        , test "ArrowUp でインデックス循環（0 → 末尾）" <|
            \_ ->
                modelForKeyboard
                    |> sendMsg (ApproverKeyDown testStepId "ArrowUp")
                    |> expectEditing
                        (\editing ->
                            -- index 0 → modBy 2 (0 - 1 + 2) = 1（末尾に循環）
                            getHighlightIndex editing
                                |> Expect.equal 1
                        )
        , test "Enter で候補選択" <|
            \_ ->
                modelForKeyboard
                    |> sendMsg (ApproverKeyDown testStepId "Enter")
                    |> expectEditing
                        (\editing ->
                            getSelection editing
                                |> Expect.equal (Just (Selected testUser1))
                        )
        , test "Escape でドロップダウン閉じる" <|
            \_ ->
                modelForKeyboard
                    |> sendMsg (ApproverKeyDown testStepId "Escape")
                    |> expectEditing
                        (\editing ->
                            getDropdownOpen editing
                                |> Expect.equal False
                        )
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
                editingModel
                    |> sendMsg (GotSaveResult (Ok testWorkflowInstance))
                    |> sendMsg (UpdateTitle "テスト")
                    |> New.isDirty
                    |> Expect.equal True
        , test "GotSaveResult (Ok ...) で isDirty = False" <|
            \_ ->
                editingModel
                    |> sendMsg (GotSaveResult (Ok testWorkflowInstance))
                    |> New.isDirty
                    |> Expect.equal False
        ]
