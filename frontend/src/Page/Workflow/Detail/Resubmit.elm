module Page.Workflow.Detail.Resubmit exposing (updateResubmit, viewEditableFormData, viewResubmitSection)

{-| 再提出・編集

差し戻しされたワークフローの編集モード、フォーム入力、
承認者選択管理、バリデーション、再申請の送信を管理する。

-}

import Api.ErrorMessage as ErrorMessage
import Api.User as UserApi
import Api.Workflow as WorkflowApi
import Component.ApproverSelector as ApproverSelector exposing (ApproverSelection(..))
import Component.Button as Button
import Component.LoadingSpinner as LoadingSpinner
import Data.FormField exposing (FormField)
import Data.UserItem as UserItem
import Data.WorkflowDefinition as WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance as WorkflowInstance
import Dict exposing (Dict)
import Form.DynamicForm as DynamicForm
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)
import Json.Decode as Decode
import Json.Encode as Encode
import Page.Workflow.Detail.Types exposing (EditState(..), EditingState, LoadedState, Msg(..))
import RemoteData exposing (RemoteData(..))
import Shared exposing (Shared)



-- UPDATE


{-| 再提出関連メッセージの処理
-}
updateResubmit : Msg -> Shared -> Int -> LoadedState -> ( LoadedState, Cmd Msg )
updateResubmit msg shared workflowDisplayNumber loaded =
    case msg of
        StartEditing ->
            let
                formDataDict =
                    case Decode.decodeValue (Decode.keyValuePairs Decode.string) loaded.workflow.formData of
                        Ok pairs ->
                            Dict.fromList pairs

                        Err _ ->
                            Dict.empty

                approverStates =
                    case loaded.definition of
                        Success def ->
                            WorkflowDefinition.approvalStepInfos def
                                |> List.map
                                    (\info ->
                                        let
                                            existingApprover =
                                                loaded.workflow.steps
                                                    |> List.filter (\s -> s.stepName == info.name)
                                                    |> List.head
                                                    |> Maybe.andThen .assignedTo

                                            state =
                                                case existingApprover of
                                                    Just ref ->
                                                        { selection = Preselected ref
                                                        , search = ""
                                                        , dropdownOpen = False
                                                        , highlightIndex = -1
                                                        }

                                                    Nothing ->
                                                        ApproverSelector.init
                                        in
                                        ( info.id, state )
                                    )
                                |> Dict.fromList

                        _ ->
                            Dict.empty
            in
            ( { loaded
                | editState =
                    Editing
                        { editFormData = formDataDict
                        , editApprovers = approverStates
                        , resubmitValidationErrors = Dict.empty
                        , isResubmitting = False
                        }
                , users = RemoteData.Loading
              }
            , UserApi.listUsers
                { config = Shared.toRequestConfig shared
                , toMsg = GotUsers
                }
            )

        CancelEditing ->
            ( { loaded | editState = Viewing }
            , Cmd.none
            )

        UpdateEditFormField fieldId fieldValue ->
            case loaded.editState of
                Editing editing ->
                    ( { loaded
                        | editState =
                            Editing { editing | editFormData = Dict.insert fieldId fieldValue editing.editFormData }
                      }
                    , Cmd.none
                    )

                Viewing ->
                    ( loaded, Cmd.none )

        EditApproverSearchChanged stepId search ->
            updateEditing loaded
                (\editing ->
                    { editing | editApprovers = updateApproverState stepId (\s -> { s | search = search, dropdownOpen = True, highlightIndex = 0 }) editing.editApprovers }
                )

        EditApproverSelected stepId user ->
            updateEditing loaded
                (\editing ->
                    { editing | editApprovers = updateApproverState stepId (\s -> { s | selection = Selected user, search = "", dropdownOpen = False }) editing.editApprovers }
                )

        EditApproverCleared stepId ->
            updateEditing loaded
                (\editing ->
                    { editing | editApprovers = updateApproverState stepId (\_ -> ApproverSelector.init) editing.editApprovers }
                )

        EditApproverKeyDown stepId key ->
            case loaded.editState of
                Editing editing ->
                    case Dict.get stepId editing.editApprovers of
                        Just state ->
                            let
                                candidates =
                                    RemoteData.withDefault [] loaded.users
                                        |> UserItem.filterUsers state.search

                                result =
                                    ApproverSelector.handleKeyDown
                                        { key = key
                                        , candidates = candidates
                                        , highlightIndex = state.highlightIndex
                                        }
                            in
                            case result of
                                ApproverSelector.Navigate newIndex ->
                                    updateEditing loaded
                                        (\e -> { e | editApprovers = updateApproverState stepId (\s -> { s | highlightIndex = newIndex }) e.editApprovers })

                                ApproverSelector.Select user ->
                                    updateEditing loaded
                                        (\e -> { e | editApprovers = updateApproverState stepId (\s -> { s | selection = Selected user, search = "", dropdownOpen = False }) e.editApprovers })

                                ApproverSelector.Close ->
                                    updateEditing loaded
                                        (\e -> { e | editApprovers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) e.editApprovers })

                                ApproverSelector.NoChange ->
                                    ( loaded, Cmd.none )

                        Nothing ->
                            ( loaded, Cmd.none )

                Viewing ->
                    ( loaded, Cmd.none )

        EditApproverDropdownClosed stepId ->
            updateEditing loaded
                (\editing ->
                    { editing | editApprovers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) editing.editApprovers }
                )

        SubmitResubmit ->
            case loaded.editState of
                Editing editing ->
                    let
                        validationErrors =
                            validateResubmit editing

                        approvers =
                            buildResubmitApprovers editing
                    in
                    if Dict.isEmpty validationErrors then
                        ( { loaded
                            | editState = Editing { editing | isResubmitting = True, resubmitValidationErrors = Dict.empty }
                            , errorMessage = Nothing
                          }
                        , WorkflowApi.resubmitWorkflow
                            { config = Shared.toRequestConfig shared
                            , displayNumber = workflowDisplayNumber
                            , body =
                                { version = loaded.workflow.version
                                , formData = encodeFormValues editing.editFormData
                                , approvers = approvers
                                }
                            , toMsg = GotResubmitResult
                            }
                        )

                    else
                        ( { loaded | editState = Editing { editing | resubmitValidationErrors = validationErrors } }
                        , Cmd.none
                        )

                Viewing ->
                    ( loaded, Cmd.none )

        GotResubmitResult result ->
            case result of
                Ok workflow ->
                    ( { loaded
                        | workflow = workflow
                        , editState = Viewing
                        , successMessage = Just "再申請しました"
                        , errorMessage = Nothing
                      }
                    , WorkflowApi.listComments
                        { config = Shared.toRequestConfig shared
                        , displayNumber = workflowDisplayNumber
                        , toMsg = GotComments
                        }
                    )

                Err error ->
                    case loaded.editState of
                        Editing editing ->
                            ( { loaded
                                | editState = Editing { editing | isResubmitting = False }
                                , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー" } error)
                              }
                            , Cmd.none
                            )

                        Viewing ->
                            ( loaded, Cmd.none )

        GotUsers result ->
            case result of
                Ok users ->
                    ( { loaded | users = Success users }, Cmd.none )

                Err err ->
                    ( { loaded | users = Failure err }, Cmd.none )

        _ ->
            ( loaded, Cmd.none )



-- HELPERS


{-| EditingState を更新するヘルパー

Editing 状態のときのみ更新を適用し、Viewing 状態では何もしない。

-}
updateEditing : LoadedState -> (EditingState -> EditingState) -> ( LoadedState, Cmd Msg )
updateEditing loaded updater =
    case loaded.editState of
        Editing editing ->
            ( { loaded | editState = Editing (updater editing) }, Cmd.none )

        Viewing ->
            ( loaded, Cmd.none )


{-| ApproverSelector.State を更新するヘルパー
-}
updateApproverState : String -> (ApproverSelector.State -> ApproverSelector.State) -> Dict String ApproverSelector.State -> Dict String ApproverSelector.State
updateApproverState stepId updater dict =
    Dict.update stepId (Maybe.map updater) dict


{-| 再提出のバリデーション
-}
validateResubmit : EditingState -> Dict String String
validateResubmit editing =
    let
        approverErrors =
            editing.editApprovers
                |> Dict.toList
                |> List.filterMap
                    (\( stepId, state ) ->
                        if ApproverSelector.selectedUserId state.selection == Nothing then
                            Just ( "approver_" ++ stepId, "承認者を選択してください" )

                        else
                            Nothing
                    )
    in
    Dict.fromList approverErrors


{-| 再提出用の承認者リストを構築
-}
buildResubmitApprovers : EditingState -> List WorkflowApi.StepApproverRequest
buildResubmitApprovers editing =
    editing.editApprovers
        |> Dict.toList
        |> List.filterMap
            (\( stepId, state ) ->
                ApproverSelector.selectedUserId state.selection
                    |> Maybe.map (\userId -> { stepId = stepId, assignedTo = userId })
            )


{-| フォーム値を JSON にエンコード
-}
encodeFormValues : Dict String String -> Encode.Value
encodeFormValues values =
    Dict.toList values
        |> List.map (\( k, v ) -> ( k, Encode.string v ))
        |> Encode.object



-- VIEW


{-| 再提出セクション

ステータスが ChangesRequested かつ現在のユーザーが起案者の場合のみ表示。

-}
viewResubmitSection : Shared -> LoadedState -> Html Msg
viewResubmitSection shared loaded =
    let
        currentUserId =
            Shared.getUserId shared

        isInitiator =
            currentUserId == Just loaded.workflow.initiatedBy.id

        isChangesRequested =
            loaded.workflow.status == WorkflowInstance.ChangesRequested
    in
    if isChangesRequested && isInitiator && loaded.editState == Viewing then
        div [ class "rounded-lg border border-warning-200 bg-warning-50 p-4" ]
            [ p [ class "mb-3 text-sm text-warning-700" ] [ text "この申請は差し戻されました。内容を修正して再申請できます。" ]
            , Button.view
                { variant = Button.Primary
                , disabled = False
                , onClick = StartEditing
                }
                [ text "再申請する" ]
            ]

    else
        text ""


{-| 編集可能なフォームデータ表示
-}
viewEditableFormData : LoadedState -> EditingState -> Html Msg
viewEditableFormData loaded editing =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "フォームデータ（編集中）" ]
        , case loaded.definition of
            Success definition ->
                case DynamicForm.extractFormFields definition.definition of
                    Ok fields ->
                        div [ class "space-y-4" ]
                            (List.map (viewEditableFormField editing.editFormData) fields
                                ++ [ viewEditableApprovers loaded editing definition
                                   , viewEditActions editing
                                   ]
                            )

                    Err _ ->
                        p [ class "text-sm text-secondary-500" ] [ text "フォーム定義の読み込みに失敗しました。" ]

            _ ->
                LoadingSpinner.view
        ]


viewEditableFormField : Dict String String -> FormField -> Html Msg
viewEditableFormField formData field =
    div [ class "space-y-1" ]
        [ label [ class "block text-sm font-medium text-secondary-700" ] [ text field.label ]
        , input
            [ type_ "text"
            , class "w-full rounded-lg border border-secondary-300 bg-white px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
            , value (Dict.get field.id formData |> Maybe.withDefault "")
            , onInput (UpdateEditFormField field.id)
            ]
            []
        ]


viewEditableApprovers : LoadedState -> EditingState -> WorkflowDefinition -> Html Msg
viewEditableApprovers loaded editing definition =
    let
        stepInfos =
            WorkflowDefinition.approvalStepInfos definition
    in
    div [ class "space-y-3" ]
        [ h3 [ class "text-sm font-semibold text-secondary-700" ] [ text "承認者" ]
        , div [ class "space-y-3" ]
            (List.map (viewEditableApproverStep loaded editing) stepInfos)
        ]


viewEditableApproverStep : LoadedState -> EditingState -> WorkflowDefinition.ApprovalStepInfo -> Html Msg
viewEditableApproverStep loaded editing stepInfo =
    let
        state =
            Dict.get stepInfo.id editing.editApprovers
                |> Maybe.withDefault ApproverSelector.init
    in
    div [ class "space-y-1" ]
        [ label [ class "block text-sm font-medium text-secondary-600" ] [ text stepInfo.name ]
        , ApproverSelector.view
            { state = state
            , users = loaded.users
            , validationError = Dict.get ("approver_" ++ stepInfo.id) editing.resubmitValidationErrors
            , onSearch = EditApproverSearchChanged stepInfo.id
            , onSelect = EditApproverSelected stepInfo.id
            , onClear = EditApproverCleared stepInfo.id
            , onKeyDown = EditApproverKeyDown stepInfo.id
            , onCloseDropdown = EditApproverDropdownClosed stepInfo.id
            }
        ]


viewEditActions : EditingState -> Html Msg
viewEditActions editing =
    div [ class "flex gap-3 pt-4 border-t border-secondary-100" ]
        [ Button.view
            { variant = Button.Primary
            , disabled = editing.isResubmitting
            , onClick = SubmitResubmit
            }
            [ text
                (if editing.isResubmitting then
                    "再申請中..."

                 else
                    "再申請する"
                )
            ]
        , Button.view
            { variant = Button.Outline
            , disabled = editing.isResubmitting
            , onClick = CancelEditing
            }
            [ text "キャンセル" ]
        ]
