module Page.Workflow.New.Update exposing (updateLoaded)

{-| New ページの Update ロジック

updateLoaded を起点に、フォーム入力・承認者選択・バリデーション・
API 呼び出し・dirty 管理を処理する。

-}

import Api exposing (ApiError)
import Api.Workflow as WorkflowApi
import Component.ApproverSelector as ApproverSelector exposing (ApproverSelection(..))
import Data.UserItem as UserItem exposing (UserItem)
import Data.WorkflowDefinition as WorkflowDefinition
import Dict exposing (Dict)
import Form.DirtyState as DirtyState
import Form.DynamicForm as DynamicForm
import Form.Validation as Validation
import List.Extra
import Page.Workflow.New.Api as NewApi
import Page.Workflow.New.Types exposing (..)
import RemoteData exposing (RemoteData(..))
import Shared exposing (Shared)


{-| Loaded 状態の更新

SelectDefinition で FormState を遷移。
それ以外は Editing 状態のときのみ updateEditing に委譲。

-}
updateLoaded : Msg -> Shared -> RemoteData ApiError (List UserItem) -> LoadedState -> ( LoadedState, Cmd Msg )
updateLoaded msg shared users loaded =
    case msg of
        SelectDefinition definitionId ->
            case List.Extra.find (\d -> d.id == definitionId) loaded.definitions of
                Just definition ->
                    let
                        previousIsDirty =
                            case loaded.formState of
                                Editing prev ->
                                    prev.isDirty_

                                SelectingDefinition ->
                                    False

                        newEditing =
                            initEditing definition

                        ( dirtyEditing, dirtyCmd ) =
                            DirtyState.markDirty { newEditing | isDirty_ = previousIsDirty }
                    in
                    ( { loaded | formState = Editing dirtyEditing }, dirtyCmd )

                Nothing ->
                    ( loaded, Cmd.none )

        _ ->
            case loaded.formState of
                Editing editing ->
                    let
                        ( newEditing, cmd ) =
                            updateEditing msg shared users editing
                    in
                    ( { loaded | formState = Editing newEditing }, cmd )

                SelectingDefinition ->
                    ( loaded, Cmd.none )


{-| Editing 状態の更新

フォーム入力、承認者選択、保存、申請の処理を行う。
定義が選択済みであることが型で保証されているため、
定義未選択チェックが不要。

-}
updateEditing : Msg -> Shared -> RemoteData ApiError (List UserItem) -> EditingState -> ( EditingState, Cmd Msg )
updateEditing msg shared users editing =
    case msg of
        UpdateTitle newTitle ->
            let
                ( dirtyEditing, dirtyCmd ) =
                    DirtyState.markDirty editing
            in
            ( { dirtyEditing | title = newTitle }
            , dirtyCmd
            )

        UpdateField fieldId value ->
            let
                ( dirtyEditing, dirtyCmd ) =
                    DirtyState.markDirty editing
            in
            ( { dirtyEditing | formValues = Dict.insert fieldId value editing.formValues }
            , dirtyCmd
            )

        UpdateApproverSearch stepId query ->
            ( { editing
                | approvers =
                    updateApproverState stepId
                        (\s ->
                            { s
                                | search = query
                                , dropdownOpen = not (String.isEmpty (String.trim query))
                                , highlightIndex = 0
                            }
                        )
                        editing.approvers
              }
            , Cmd.none
            )

        SelectApprover stepId user ->
            let
                ( dirtyEditing, dirtyCmd ) =
                    DirtyState.markDirty editing
            in
            ( { dirtyEditing
                | approvers =
                    updateApproverState stepId
                        (\s ->
                            { s
                                | selection = Selected user
                                , search = ""
                                , dropdownOpen = False
                                , highlightIndex = 0
                            }
                        )
                        dirtyEditing.approvers
                , validationErrors = Dict.remove ("approver_" ++ stepId) dirtyEditing.validationErrors
              }
            , dirtyCmd
            )

        ClearApprover stepId ->
            let
                ( dirtyEditing, dirtyCmd ) =
                    DirtyState.markDirty editing
            in
            ( { dirtyEditing | approvers = Dict.insert stepId ApproverSelector.init dirtyEditing.approvers }
            , dirtyCmd
            )

        ApproverKeyDown stepId key ->
            handleApproverKeyDown stepId key users editing

        CloseApproverDropdown stepId ->
            ( { editing
                | approvers =
                    updateApproverState stepId
                        (\s -> { s | dropdownOpen = False })
                        editing.approvers
              }
            , Cmd.none
            )

        SaveDraft ->
            case Validation.validateTitle editing.title of
                Err errorMsg ->
                    ( { editing
                        | validationErrors = Dict.singleton "title" errorMsg
                        , saveMessage = Nothing
                      }
                    , Cmd.none
                    )

                Ok _ ->
                    ( { editing
                        | submitting = True
                        , saveMessage = Nothing
                        , validationErrors = Dict.empty
                      }
                    , NewApi.saveDraft shared editing.selectedDefinition.id editing.title editing.formValues
                    )

        GotSaveResult result ->
            case result of
                Ok workflow ->
                    let
                        ( cleanEditing, cleanCmd ) =
                            DirtyState.clearDirty editing
                    in
                    ( { cleanEditing
                        | submitting = False
                        , savedWorkflow = Just workflow
                        , saveMessage = Just (SaveSuccess "下書きを保存しました")
                      }
                    , cleanCmd
                    )

                Err _ ->
                    ( { editing
                        | submitting = False
                        , saveMessage = Just (SaveError "保存に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        Submit ->
            let
                validationErrors =
                    validateFormWithApprover editing
            in
            if Dict.isEmpty validationErrors then
                let
                    approvers =
                        buildApprovers editing
                in
                case editing.savedWorkflow of
                    Just workflow ->
                        let
                            ( cleanEditing, cleanCmd ) =
                                DirtyState.clearDirty editing
                        in
                        ( { cleanEditing
                            | submitting = True
                            , saveMessage = Nothing
                          }
                        , Cmd.batch
                            [ NewApi.submitWorkflow shared workflow.displayNumber approvers
                            , cleanCmd
                            ]
                        )

                    Nothing ->
                        ( { editing
                            | submitting = True
                            , saveMessage = Nothing
                          }
                        , NewApi.saveAndSubmit shared
                            editing.selectedDefinition.id
                            editing.title
                            editing.formValues
                            approvers
                        )

            else
                ( { editing
                    | validationErrors = validationErrors
                    , saveMessage = Nothing
                  }
                , Cmd.none
                )

        GotSaveAndSubmitResult approvers result ->
            case result of
                Ok workflow ->
                    let
                        ( cleanEditing, cleanCmd ) =
                            DirtyState.clearDirty editing
                    in
                    ( { cleanEditing | savedWorkflow = Just workflow }
                    , Cmd.batch
                        [ NewApi.submitWorkflow shared workflow.displayNumber approvers
                        , cleanCmd
                        ]
                    )

                Err _ ->
                    ( { editing
                        | submitting = False
                        , saveMessage = Just (SaveError "保存に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        GotSubmitResult result ->
            case result of
                Ok workflow ->
                    let
                        ( cleanEditing, cleanCmd ) =
                            DirtyState.clearDirty editing
                    in
                    ( { cleanEditing
                        | submitting = False
                        , savedWorkflow = Just workflow
                        , saveMessage = Just (SaveSuccess "申請が完了しました")
                      }
                    , cleanCmd
                    )

                Err _ ->
                    ( { editing
                        | submitting = False
                        , saveMessage = Just (SaveError "申請に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        ClearMessage ->
            ( { editing | saveMessage = Nothing }
            , Cmd.none
            )

        -- 外側レベルで処理済みのメッセージ（GotDefinitions, GotUsers, SelectDefinition）
        _ ->
            ( editing, Cmd.none )



-- ヘルパー関数


{-| フォーム全体のバリデーション

タイトルと動的フォームフィールドを検証する。
selectedDefinition により定義検索が不要。

-}
validateForm : EditingState -> Dict String String
validateForm editing =
    let
        titleErrors =
            case Validation.validateTitle editing.title of
                Err errorMsg ->
                    Dict.singleton "title" errorMsg

                Ok _ ->
                    Dict.empty

        fieldErrors =
            case DynamicForm.extractFormFields editing.selectedDefinition.definition of
                Ok fields ->
                    Validation.validateAllFields fields editing.formValues

                Err _ ->
                    Dict.empty
    in
    Dict.union titleErrors fieldErrors


{-| フォーム全体 + 承認者のバリデーション（申請用）
-}
validateFormWithApprover : EditingState -> Dict String String
validateFormWithApprover editing =
    let
        formErrors =
            validateForm editing

        approverErrors =
            editing.approvers
                |> Dict.toList
                |> List.filterMap
                    (\( stepId, state ) ->
                        if ApproverSelector.selectedUserId state.selection == Nothing then
                            Just ( "approver_" ++ stepId, "承認者を選択してください" )

                        else
                            Nothing
                    )
                |> Dict.fromList
    in
    Dict.union formErrors approverErrors


{-| 承認者検索のキーボードイベントを処理
-}
handleApproverKeyDown : String -> String -> RemoteData ApiError (List UserItem) -> EditingState -> ( EditingState, Cmd Msg )
handleApproverKeyDown stepId key users editing =
    case Dict.get stepId editing.approvers of
        Just state ->
            let
                candidates =
                    case users of
                        Success userList ->
                            UserItem.filterUsers state.search userList

                        _ ->
                            []

                result =
                    ApproverSelector.handleKeyDown
                        { key = key
                        , candidates = candidates
                        , highlightIndex = state.highlightIndex
                        }
            in
            case result of
                ApproverSelector.NoChange ->
                    ( editing, Cmd.none )

                ApproverSelector.Navigate newIndex ->
                    ( { editing | approvers = updateApproverState stepId (\s -> { s | highlightIndex = newIndex }) editing.approvers }
                    , Cmd.none
                    )

                ApproverSelector.Select user ->
                    let
                        ( dirtyEditing, dirtyCmd ) =
                            DirtyState.markDirty editing
                    in
                    ( { dirtyEditing
                        | approvers =
                            updateApproverState stepId
                                (\s ->
                                    { s
                                        | selection = Selected user
                                        , search = ""
                                        , dropdownOpen = False
                                        , highlightIndex = 0
                                    }
                                )
                                dirtyEditing.approvers
                        , validationErrors = Dict.remove ("approver_" ++ stepId) dirtyEditing.validationErrors
                      }
                    , dirtyCmd
                    )

                ApproverSelector.Close ->
                    ( { editing | approvers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) editing.approvers }
                    , Cmd.none
                    )

        Nothing ->
            ( editing, Cmd.none )


{-| ApproverSelector.State を更新するヘルパー
-}
updateApproverState : String -> (ApproverSelector.State -> ApproverSelector.State) -> Dict String ApproverSelector.State -> Dict String ApproverSelector.State
updateApproverState stepId updater dict =
    Dict.update stepId (Maybe.map updater) dict


{-| 各ステップの承認者選択から承認者リストを構築する

定義の承認ステップ順序に従って構築する。
selectedDefinition により直接ステップ情報にアクセスできるため、
Dict のキー順へのフォールバックが不要。

-}
buildApprovers : EditingState -> List WorkflowApi.StepApproverRequest
buildApprovers editing =
    let
        stepIds =
            WorkflowDefinition.approvalStepInfos editing.selectedDefinition
                |> List.map .id
    in
    stepIds
        |> List.filterMap
            (\stepId ->
                Dict.get stepId editing.approvers
                    |> Maybe.andThen (\state -> ApproverSelector.selectedUserId state.selection)
                    |> Maybe.map (\userId -> { stepId = stepId, assignedTo = userId })
            )
