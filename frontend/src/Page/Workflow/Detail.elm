module Page.Workflow.Detail exposing
    ( Model
    , Msg
    , init
    , subscriptions
    , update
    , updateShared
    , view
    )

{-| 申請詳細ページ

ワークフローインスタンスの詳細情報を表示する。


## 機能

  - 基本情報の表示（タイトル、ステータス、日時）
  - フォームデータの表示
  - 一覧への戻るリンク
  - 承認/却下時の確認ダイアログ


## 設計

詳細: [申請フォーム UI 設計](../../../../docs/03_詳細設計書/10_ワークフロー申請フォームUI設計.md)

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.User as UserApi
import Api.Workflow as WorkflowApi
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Component.ApproverSelector as ApproverSelector exposing (ApproverSelection(..))
import Component.Badge as Badge
import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.FormField exposing (FormField)
import Data.UserItem as UserItem exposing (UserItem)
import Data.WorkflowComment exposing (WorkflowComment)
import Data.WorkflowDefinition as WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance as WorkflowInstance exposing (WorkflowInstance, WorkflowStep)
import Dict exposing (Dict)
import Form.DynamicForm as DynamicForm
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)
import Json.Decode as Decode
import Json.Encode as Encode
import Ports
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)
import Time
import Util.DateFormat as DateFormat



-- MODEL


{-| 確認待ちの操作

承認/却下ボタンクリック後、確認ダイアログで最終確認するまで保持する。

-}
type PendingAction
    = ConfirmApprove WorkflowStep
    | ConfirmReject WorkflowStep
    | ConfirmRequestChanges WorkflowStep


{-| ページの状態
-}
type alias Model =
    { -- 共有状態
      shared : Shared

    -- パラメータ
    , workflowDisplayNumber : Int

    -- API データ
    , workflow : RemoteData ApiError WorkflowInstance
    , definition : RemoteData ApiError WorkflowDefinition

    -- 承認/却下の状態
    , comment : String
    , isSubmitting : Bool
    , pendingAction : Maybe PendingAction
    , errorMessage : Maybe String
    , successMessage : Maybe String

    -- コメントスレッド
    , comments : RemoteData ApiError (List WorkflowComment)
    , newCommentBody : String
    , isPostingComment : Bool

    -- 再提出
    , isEditing : Bool
    , editFormData : Dict String String
    , editApprovers : Dict String ApproverSelector.State
    , users : RemoteData ApiError (List UserItem)
    , resubmitValidationErrors : Dict String String
    , isResubmitting : Bool
    }


{-| 初期化
-}
init : Shared -> Int -> ( Model, Cmd Msg )
init shared workflowDisplayNumber =
    ( { shared = shared
      , workflowDisplayNumber = workflowDisplayNumber
      , workflow = Loading
      , definition = NotAsked
      , comment = ""
      , isSubmitting = False
      , pendingAction = Nothing
      , errorMessage = Nothing
      , successMessage = Nothing
      , comments = Loading
      , newCommentBody = ""
      , isPostingComment = False
      , isEditing = False
      , editFormData = Dict.empty
      , editApprovers = Dict.empty
      , users = NotAsked
      , resubmitValidationErrors = Dict.empty
      , isResubmitting = False
      }
    , Cmd.batch
        [ WorkflowApi.getWorkflow
            { config = Shared.toRequestConfig shared
            , displayNumber = workflowDisplayNumber
            , toMsg = GotWorkflow
            }
        , WorkflowApi.listComments
            { config = Shared.toRequestConfig shared
            , displayNumber = workflowDisplayNumber
            , toMsg = GotComments
            }
        ]
    )


{-| 共有状態を更新

Main.elm から新しい共有状態（CSRF トークン取得後など）を受け取る。

-}
updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


{-| メッセージ
-}
type Msg
    = GotWorkflow (Result ApiError WorkflowInstance)
    | GotDefinition (Result ApiError WorkflowDefinition)
    | Refresh
    | UpdateComment String
    | ClickApprove WorkflowStep
    | ClickReject WorkflowStep
    | ClickRequestChanges WorkflowStep
    | ConfirmAction
    | CancelAction
    | GotApproveResult (Result ApiError WorkflowInstance)
    | GotRejectResult (Result ApiError WorkflowInstance)
    | GotRequestChangesResult (Result ApiError WorkflowInstance)
    | GotComments (Result ApiError (List WorkflowComment))
    | UpdateNewComment String
    | SubmitComment
    | GotPostCommentResult (Result ApiError WorkflowComment)
      -- 再提出
    | StartEditing
    | CancelEditing
    | UpdateEditFormField String String
    | EditApproverSearchChanged String String
    | EditApproverSelected String UserItem
    | EditApproverCleared String
    | EditApproverKeyDown String String
    | EditApproverDropdownClosed String
    | SubmitResubmit
    | GotResubmitResult (Result ApiError WorkflowInstance)
    | GotUsers (Result ApiError (List UserItem))
    | DismissMessage


{-| 状態更新
-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotWorkflow result ->
            case result of
                Ok workflow ->
                    ( { model | workflow = Success workflow }
                    , WorkflowDefinitionApi.getDefinition
                        { config = Shared.toRequestConfig model.shared
                        , id = workflow.definitionId
                        , toMsg = GotDefinition
                        }
                    )

                Err err ->
                    ( { model | workflow = Failure err }
                    , Cmd.none
                    )

        GotDefinition result ->
            case result of
                Ok definition ->
                    ( { model | definition = Success definition }
                    , Cmd.none
                    )

                Err err ->
                    ( { model | definition = Failure err }
                    , Cmd.none
                    )

        Refresh ->
            ( { model
                | workflow = Loading
                , definition = NotAsked
                , comments = Loading
                , errorMessage = Nothing
                , successMessage = Nothing
              }
            , Cmd.batch
                [ WorkflowApi.getWorkflow
                    { config = Shared.toRequestConfig model.shared
                    , displayNumber = model.workflowDisplayNumber
                    , toMsg = GotWorkflow
                    }
                , WorkflowApi.listComments
                    { config = Shared.toRequestConfig model.shared
                    , displayNumber = model.workflowDisplayNumber
                    , toMsg = GotComments
                    }
                ]
            )

        UpdateComment newComment ->
            ( { model | comment = newComment }, Cmd.none )

        ClickApprove step ->
            ( { model | pendingAction = Just (ConfirmApprove step) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ClickReject step ->
            ( { model | pendingAction = Just (ConfirmReject step) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ClickRequestChanges step ->
            ( { model | pendingAction = Just (ConfirmRequestChanges step) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ConfirmAction ->
            case model.pendingAction of
                Just (ConfirmApprove step) ->
                    ( { model | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , WorkflowApi.approveStep
                        { config = Shared.toRequestConfig model.shared
                        , workflowDisplayNumber = model.workflowDisplayNumber
                        , stepDisplayNumber = step.displayNumber
                        , body = { version = step.version, comment = nonEmptyComment model.comment }
                        , toMsg = GotApproveResult
                        }
                    )

                Just (ConfirmReject step) ->
                    ( { model | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , WorkflowApi.rejectStep
                        { config = Shared.toRequestConfig model.shared
                        , workflowDisplayNumber = model.workflowDisplayNumber
                        , stepDisplayNumber = step.displayNumber
                        , body = { version = step.version, comment = nonEmptyComment model.comment }
                        , toMsg = GotRejectResult
                        }
                    )

                Just (ConfirmRequestChanges step) ->
                    ( { model | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , WorkflowApi.requestChangesStep
                        { config = Shared.toRequestConfig model.shared
                        , workflowDisplayNumber = model.workflowDisplayNumber
                        , stepDisplayNumber = step.displayNumber
                        , body = { version = step.version, comment = nonEmptyComment model.comment }
                        , toMsg = GotRequestChangesResult
                        }
                    )

                Nothing ->
                    ( model, Cmd.none )

        CancelAction ->
            ( { model | pendingAction = Nothing }
            , Cmd.none
            )

        GotApproveResult result ->
            handleApprovalResult "承認しました" result model

        GotRejectResult result ->
            handleApprovalResult "却下しました" result model

        GotRequestChangesResult result ->
            handleApprovalResult "差し戻しました" result model

        GotComments result ->
            case result of
                Ok comments ->
                    ( { model | comments = Success comments }, Cmd.none )

                Err err ->
                    ( { model | comments = Failure err }, Cmd.none )

        UpdateNewComment body ->
            ( { model | newCommentBody = body }, Cmd.none )

        SubmitComment ->
            if String.isEmpty (String.trim model.newCommentBody) then
                ( model, Cmd.none )

            else
                ( { model | isPostingComment = True }
                , WorkflowApi.postComment
                    { config = Shared.toRequestConfig model.shared
                    , displayNumber = model.workflowDisplayNumber
                    , body = { body = String.trim model.newCommentBody }
                    , toMsg = GotPostCommentResult
                    }
                )

        GotPostCommentResult result ->
            case result of
                Ok newComment ->
                    let
                        updatedComments =
                            case model.comments of
                                Success existing ->
                                    Success (existing ++ [ newComment ])

                                _ ->
                                    Success [ newComment ]
                    in
                    ( { model
                        | comments = updatedComments
                        , newCommentBody = ""
                        , isPostingComment = False
                      }
                    , Cmd.none
                    )

                Err _ ->
                    ( { model
                        | isPostingComment = False
                        , errorMessage = Just "コメントの投稿に失敗しました。"
                      }
                    , Cmd.none
                    )

        StartEditing ->
            case model.workflow of
                Success workflow ->
                    let
                        formDataDict =
                            case Decode.decodeValue (Decode.keyValuePairs Decode.string) workflow.formData of
                                Ok pairs ->
                                    Dict.fromList pairs

                                Err _ ->
                                    Dict.empty

                        approverStates =
                            case model.definition of
                                Success def ->
                                    WorkflowDefinition.approvalStepInfos def
                                        |> List.map
                                            (\info ->
                                                let
                                                    existingApprover =
                                                        workflow.steps
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
                    ( { model
                        | isEditing = True
                        , editFormData = formDataDict
                        , editApprovers = approverStates
                        , resubmitValidationErrors = Dict.empty
                        , users = Loading
                      }
                    , UserApi.listUsers
                        { config = Shared.toRequestConfig model.shared
                        , toMsg = GotUsers
                        }
                    )

                _ ->
                    ( model, Cmd.none )

        CancelEditing ->
            ( { model
                | isEditing = False
                , editFormData = Dict.empty
                , editApprovers = Dict.empty
                , resubmitValidationErrors = Dict.empty
              }
            , Cmd.none
            )

        UpdateEditFormField fieldId fieldValue ->
            ( { model | editFormData = Dict.insert fieldId fieldValue model.editFormData }
            , Cmd.none
            )

        EditApproverSearchChanged stepId search ->
            ( { model | editApprovers = updateApproverState stepId (\s -> { s | search = search, dropdownOpen = True, highlightIndex = 0 }) model.editApprovers }
            , Cmd.none
            )

        EditApproverSelected stepId user ->
            ( { model | editApprovers = updateApproverState stepId (\s -> { s | selection = Selected user, search = "", dropdownOpen = False }) model.editApprovers }
            , Cmd.none
            )

        EditApproverCleared stepId ->
            ( { model | editApprovers = updateApproverState stepId (\_ -> ApproverSelector.init) model.editApprovers }
            , Cmd.none
            )

        EditApproverKeyDown stepId key ->
            case Dict.get stepId model.editApprovers of
                Just state ->
                    let
                        candidates =
                            RemoteData.withDefault [] model.users
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
                            ( { model | editApprovers = updateApproverState stepId (\s -> { s | highlightIndex = newIndex }) model.editApprovers }, Cmd.none )

                        ApproverSelector.Select user ->
                            ( { model | editApprovers = updateApproverState stepId (\s -> { s | selection = Selected user, search = "", dropdownOpen = False }) model.editApprovers }, Cmd.none )

                        ApproverSelector.Close ->
                            ( { model | editApprovers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) model.editApprovers }, Cmd.none )

                        ApproverSelector.NoChange ->
                            ( model, Cmd.none )

                Nothing ->
                    ( model, Cmd.none )

        EditApproverDropdownClosed stepId ->
            ( { model | editApprovers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) model.editApprovers }
            , Cmd.none
            )

        SubmitResubmit ->
            case model.workflow of
                Success workflow ->
                    let
                        validationErrors =
                            validateResubmit model

                        approvers =
                            buildResubmitApprovers model
                    in
                    if Dict.isEmpty validationErrors then
                        ( { model | isResubmitting = True, resubmitValidationErrors = Dict.empty, errorMessage = Nothing }
                        , WorkflowApi.resubmitWorkflow
                            { config = Shared.toRequestConfig model.shared
                            , displayNumber = model.workflowDisplayNumber
                            , body =
                                { version = workflow.version
                                , formData = encodeFormValues model.editFormData
                                , approvers = approvers
                                }
                            , toMsg = GotResubmitResult
                            }
                        )

                    else
                        ( { model | resubmitValidationErrors = validationErrors }, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        GotResubmitResult result ->
            case result of
                Ok workflow ->
                    ( { model
                        | workflow = Success workflow
                        , isEditing = False
                        , isResubmitting = False
                        , editFormData = Dict.empty
                        , editApprovers = Dict.empty
                        , successMessage = Just "再申請しました"
                        , errorMessage = Nothing
                      }
                    , WorkflowApi.listComments
                        { config = Shared.toRequestConfig model.shared
                        , displayNumber = model.workflowDisplayNumber
                        , toMsg = GotComments
                        }
                    )

                Err error ->
                    ( { model
                        | isResubmitting = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー" } error)
                      }
                    , Cmd.none
                    )

        GotUsers result ->
            case result of
                Ok users ->
                    ( { model | users = Success users }, Cmd.none )

                Err err ->
                    ( { model | users = Failure err }, Cmd.none )

        DismissMessage ->
            ( { model | errorMessage = Nothing, successMessage = Nothing }
            , Cmd.none
            )


{-| 空文字列を Nothing に変換
-}
nonEmptyComment : String -> Maybe String
nonEmptyComment comment =
    if String.isEmpty (String.trim comment) then
        Nothing

    else
        Just (String.trim comment)


{-| 承認/却下結果のハンドリング
-}
handleApprovalResult : String -> Result ApiError WorkflowInstance -> Model -> ( Model, Cmd Msg )
handleApprovalResult successMsg result model =
    case result of
        Ok workflow ->
            ( { model
                | workflow = Success workflow
                , isSubmitting = False
                , successMessage = Just successMsg
                , errorMessage = Nothing
                , comment = ""
              }
            , Cmd.none
            )

        Err error ->
            ( { model
                | isSubmitting = False
                , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー" } error)
              }
            , Cmd.none
            )



-- RESUBMIT HELPERS


{-| ApproverSelector.State を更新するヘルパー
-}
updateApproverState : String -> (ApproverSelector.State -> ApproverSelector.State) -> Dict String ApproverSelector.State -> Dict String ApproverSelector.State
updateApproverState stepId updater dict =
    Dict.update stepId (Maybe.map updater) dict


{-| 再提出のバリデーション
-}
validateResubmit : Model -> Dict String String
validateResubmit model =
    let
        approverErrors =
            model.editApprovers
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
buildResubmitApprovers : Model -> List WorkflowApi.StepApproverRequest
buildResubmitApprovers model =
    model.editApprovers
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



-- SUBSCRIPTIONS


{-| 外部イベントの購読
-}
subscriptions : Sub Msg
subscriptions =
    Sub.none



-- VIEW


{-| ビュー
-}
view : Model -> Html Msg
view model =
    div []
        [ viewHeader
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = model.successMessage
            , errorMessage = model.errorMessage
            }
        , viewContent model
        , viewConfirmDialog model.pendingAction
        ]


viewHeader : Html Msg
viewHeader =
    nav [ class "mb-6 flex items-center gap-2 text-sm" ]
        [ a [ href (Route.toString (Route.Workflows Route.emptyWorkflowFilter)), class "text-secondary-500 hover:text-primary-600 transition-colors" ] [ text "申請一覧" ]
        , span [ class "text-secondary-400" ] [ text "/" ]
        , span [ class "text-secondary-900 font-medium" ] [ text "申請詳細" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.workflow of
        NotAsked ->
            div [] []

        Loading ->
            LoadingSpinner.view

        Failure _ ->
            viewError

        Success workflow ->
            viewWorkflowDetail model workflow


viewError : Html Msg
viewError =
    div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , Button.view
            { variant = Button.Outline
            , disabled = False
            , onClick = Refresh
            }
            [ text "再読み込み" ]
        ]


viewWorkflowDetail : Model -> WorkflowInstance -> Html Msg
viewWorkflowDetail model workflow =
    div [ class "space-y-6" ]
        [ viewTitle workflow
        , viewStatus workflow
        , viewStepProgress workflow
        , viewApprovalSection workflow model.comment model.isSubmitting model.shared
        , viewResubmitSection model workflow
        , viewSteps workflow
        , viewBasicInfo (Shared.zone model.shared) workflow
        , if model.isEditing then
            viewEditableFormData model

          else
            viewFormData workflow model.definition
        , viewCommentSection model
        ]


viewTitle : WorkflowInstance -> Html Msg
viewTitle workflow =
    h1 [ class "text-2xl font-bold text-secondary-900" ]
        [ span [ class "text-secondary-400 mr-2" ] [ text workflow.displayId ]
        , text workflow.title
        ]


viewStatus : WorkflowInstance -> Html Msg
viewStatus workflow =
    div [ class "text-secondary-700" ]
        [ text "ステータス: "
        , Badge.view
            { colorClass = WorkflowInstance.statusToCssClass workflow.status
            , label = WorkflowInstance.statusToJapanese workflow.status
            }
        ]


{-| ステップ進行状況の水平プログレス表示

全ステップを水平に並べ、ステータスに応じた色分けで進行状況を可視化する。
ステップが1つ以下の場合は表示しない（単一ステップの場合はプログレス表示の意味がない）。

-}
viewStepProgress : WorkflowInstance -> Html Msg
viewStepProgress workflow =
    if List.length workflow.steps <= 1 then
        text ""

    else
        div [ class "rounded-lg border border-secondary-200 bg-white p-4 shadow-sm" ]
            [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "進行状況" ]
            , let
                totalSteps =
                    List.length workflow.steps
              in
              div [ class "flex items-center gap-1" ]
                (workflow.steps
                    |> List.indexedMap
                        (\index step ->
                            let
                                isLast =
                                    index == totalSteps - 1
                            in
                            div [ class "flex items-center gap-1 flex-1 min-w-0" ]
                                (viewStepProgressItem step
                                    :: (if isLast then
                                            []

                                        else
                                            [ viewStepConnector step ]
                                       )
                                )
                        )
                )
            ]


viewStepProgressItem : WorkflowStep -> Html Msg
viewStepProgressItem step =
    let
        ( bgClass, textClass, borderClass ) =
            stepProgressStyle step
    in
    div [ class ("flex flex-col items-center min-w-0 flex-1 " ++ borderClass) ]
        [ div [ class ("flex h-8 w-8 items-center justify-center rounded-full text-xs font-bold " ++ bgClass ++ " " ++ textClass) ]
            [ text (String.fromInt step.displayNumber) ]
        , span [ class "mt-1 text-xs text-secondary-600 truncate max-w-full text-center" ]
            [ text step.stepName ]
        , case step.assignedTo of
            Just assignee ->
                span [ class "text-xs text-secondary-400 truncate max-w-full" ] [ text assignee.name ]

            Nothing ->
                text ""
        ]


viewStepConnector : WorkflowStep -> Html Msg
viewStepConnector step =
    let
        connectorClass =
            if step.status == WorkflowInstance.StepCompleted then
                "bg-success-400"

            else
                "bg-secondary-200"
    in
    div [ class ("h-0.5 flex-1 " ++ connectorClass) ] []


{-| ステップの進行状況スタイル
-}
stepProgressStyle : WorkflowStep -> ( String, String, String )
stepProgressStyle step =
    case ( step.status, step.decision ) of
        ( WorkflowInstance.StepCompleted, Just WorkflowInstance.DecisionApproved ) ->
            ( "bg-success-100", "text-success-700", "" )

        ( WorkflowInstance.StepCompleted, Just WorkflowInstance.DecisionRejected ) ->
            ( "bg-error-100", "text-error-700", "" )

        ( WorkflowInstance.StepCompleted, Just WorkflowInstance.DecisionRequestChanges ) ->
            ( "bg-warning-100", "text-warning-700", "" )

        ( WorkflowInstance.StepActive, _ ) ->
            ( "bg-info-100", "text-info-700", "ring-2 ring-info-300" )

        _ ->
            ( "bg-secondary-100", "text-secondary-500", "" )


viewBasicInfo : Time.Zone -> WorkflowInstance -> Html Msg
viewBasicInfo zone workflow =
    div []
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "基本情報" ]
        , dl [ class "grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm" ]
            [ dt [ class "text-secondary-500" ] [ text "申請者" ]
            , dd [ class "text-secondary-900" ] [ text workflow.initiatedBy.name ]
            , dt [ class "text-secondary-500" ] [ text "申請日" ]
            , dd [ class "text-secondary-900" ] [ text (DateFormat.formatMaybeDateTime zone workflow.submittedAt) ]
            , dt [ class "text-secondary-500" ] [ text "作成日" ]
            , dd [ class "text-secondary-900" ] [ text (DateFormat.formatDateTime zone workflow.createdAt) ]
            , dt [ class "text-secondary-500" ] [ text "更新日" ]
            , dd [ class "text-secondary-900" ] [ text (DateFormat.formatDateTime zone workflow.updatedAt) ]
            ]
        ]


{-| 再提出セクション

ステータスが ChangesRequested かつ現在のユーザーが起案者の場合のみ表示。

-}
viewResubmitSection : Model -> WorkflowInstance -> Html Msg
viewResubmitSection model workflow =
    let
        currentUserId =
            Shared.getUserId model.shared

        isInitiator =
            currentUserId == Just workflow.initiatedBy.id

        isChangesRequested =
            workflow.status == WorkflowInstance.ChangesRequested
    in
    if isChangesRequested && isInitiator && not model.isEditing then
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
viewEditableFormData : Model -> Html Msg
viewEditableFormData model =
    div []
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "フォームデータ（編集中）" ]
        , case model.definition of
            Success definition ->
                case DynamicForm.extractFormFields definition.definition of
                    Ok fields ->
                        div [ class "space-y-4" ]
                            (List.map (viewEditableFormField model.editFormData) fields
                                ++ [ viewEditableApprovers model definition
                                   , viewEditActions model
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


viewEditableApprovers : Model -> WorkflowDefinition -> Html Msg
viewEditableApprovers model definition =
    let
        stepInfos =
            WorkflowDefinition.approvalStepInfos definition
    in
    div [ class "space-y-3" ]
        [ h3 [ class "text-sm font-semibold text-secondary-700" ] [ text "承認者" ]
        , div [ class "space-y-3" ]
            (List.map (viewEditableApproverStep model) stepInfos)
        ]


viewEditableApproverStep : Model -> WorkflowDefinition.ApprovalStepInfo -> Html Msg
viewEditableApproverStep model stepInfo =
    let
        state =
            Dict.get stepInfo.id model.editApprovers
                |> Maybe.withDefault ApproverSelector.init
    in
    div [ class "space-y-1" ]
        [ label [ class "block text-sm font-medium text-secondary-600" ] [ text stepInfo.name ]
        , ApproverSelector.view
            { state = state
            , users = model.users
            , validationError = Dict.get ("approver_" ++ stepInfo.id) model.resubmitValidationErrors
            , onSearch = EditApproverSearchChanged stepInfo.id
            , onSelect = EditApproverSelected stepInfo.id
            , onClear = EditApproverCleared stepInfo.id
            , onKeyDown = EditApproverKeyDown stepInfo.id
            , onCloseDropdown = EditApproverDropdownClosed stepInfo.id
            }
        ]


viewEditActions : Model -> Html Msg
viewEditActions model =
    div [ class "flex gap-3 pt-4 border-t border-secondary-100" ]
        [ Button.view
            { variant = Button.Primary
            , disabled = model.isResubmitting
            , onClick = SubmitResubmit
            }
            [ text
                (if model.isResubmitting then
                    "再申請中..."

                 else
                    "再申請する"
                )
            ]
        , Button.view
            { variant = Button.Outline
            , disabled = model.isResubmitting
            , onClick = CancelEditing
            }
            [ text "キャンセル" ]
        ]


viewFormData : WorkflowInstance -> RemoteData ApiError WorkflowDefinition -> Html Msg
viewFormData workflow maybeDefinition =
    div []
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "フォームデータ" ]
        , case maybeDefinition of
            NotAsked ->
                text ""

            Loading ->
                div [ class "flex flex-col items-center justify-center py-8" ]
                    [ div [ class "h-8 w-8 animate-spin rounded-full border-4 border-secondary-100 border-t-primary-600" ] []
                    , p [ class "mt-4 text-secondary-500" ] [ text "読み込み中..." ]
                    ]

            Failure _ ->
                viewRawFormData workflow.formData

            Success definition ->
                viewFormDataWithLabels definition workflow.formData
        ]


viewFormDataWithLabels : WorkflowDefinition -> Decode.Value -> Html Msg
viewFormDataWithLabels definition formData =
    case DynamicForm.extractFormFields definition.definition of
        Ok fields ->
            dl [ class "grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm" ]
                (List.concatMap (viewFormField formData) fields)

        Err _ ->
            viewRawFormData formData


viewFormField : Decode.Value -> FormField -> List (Html Msg)
viewFormField formData field =
    let
        value =
            Decode.decodeValue (Decode.field field.id Decode.string) formData
                |> Result.withDefault ""
    in
    [ dt [ class "text-secondary-500" ] [ text field.label ]
    , dd [ class "text-secondary-900" ]
        [ text
            (if String.isEmpty value then
                "-"

             else
                value
            )
        ]
    ]


viewRawFormData : Decode.Value -> Html Msg
viewRawFormData formData =
    pre [ class "overflow-x-auto rounded-lg bg-secondary-50 p-4 text-sm font-mono" ]
        [ text
            (Decode.decodeValue (Decode.keyValuePairs Decode.string) formData
                |> Result.map (List.map (\( k, v ) -> k ++ ": " ++ v) >> String.join "\n")
                |> Result.withDefault "（データなし）"
            )
        ]



-- COMMENT SECTION


{-| コメントセクション

ワークフローに紐づくコメントスレッドを表示し、新規コメントの投稿を提供する。

-}
viewCommentSection : Model -> Html Msg
viewCommentSection model =
    div []
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "コメント" ]
        , case model.comments of
            NotAsked ->
                text ""

            Loading ->
                div [ class "flex items-center gap-2 py-4 text-sm text-secondary-500" ]
                    [ div [ class "h-4 w-4 animate-spin rounded-full border-2 border-secondary-200 border-t-primary-600" ] []
                    , text "読み込み中..."
                    ]

            Failure _ ->
                div [ class "rounded-lg bg-error-50 p-3 text-sm text-error-700" ]
                    [ text "コメントの取得に失敗しました。" ]

            Success comments ->
                div [ class "space-y-4" ]
                    [ viewCommentList comments
                    , viewCommentForm model.newCommentBody model.isPostingComment
                    ]
        ]


viewCommentList : List WorkflowComment -> Html Msg
viewCommentList comments =
    if List.isEmpty comments then
        p [ class "text-sm text-secondary-500" ] [ text "コメントはまだありません。" ]

    else
        div [ class "space-y-3" ]
            (List.map viewCommentItem comments)


viewCommentItem : WorkflowComment -> Html Msg
viewCommentItem commentData =
    div [ class "rounded-lg border border-secondary-200 bg-white p-3" ]
        [ div [ class "flex items-center justify-between text-xs text-secondary-500" ]
            [ span [ class "font-medium text-secondary-700" ] [ text commentData.postedBy.name ]
            , span [] [ text commentData.createdAt ]
            ]
        , p [ class "mt-1 text-sm text-secondary-900 whitespace-pre-wrap" ] [ text commentData.body ]
        ]


viewCommentForm : String -> Bool -> Html Msg
viewCommentForm body isPosting =
    div [ class "space-y-2" ]
        [ textarea
            [ class "w-full rounded-lg border border-secondary-300 bg-white px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
            , value body
            , onInput UpdateNewComment
            , placeholder "コメントを入力..."
            , rows 3
            , disabled isPosting
            ]
            []
        , div [ class "flex justify-end" ]
            [ Button.view
                { variant = Button.Primary
                , disabled = isPosting || String.isEmpty (String.trim body)
                , onClick = SubmitComment
                }
                [ text
                    (if isPosting then
                        "投稿中..."

                     else
                        "コメントを投稿"
                    )
                ]
            ]
        ]



-- CONFIRM DIALOG


{-| 確認ダイアログの描画

pendingAction が Nothing の場合は何も表示しない。

-}
viewConfirmDialog : Maybe PendingAction -> Html Msg
viewConfirmDialog maybePending =
    case maybePending of
        Just (ConfirmApprove _) ->
            ConfirmDialog.view
                { title = "承認の確認"
                , message = "この申請を承認しますか？"
                , confirmLabel = "承認する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmAction
                , onCancel = CancelAction
                , actionStyle = ConfirmDialog.Positive
                }

        Just (ConfirmReject _) ->
            ConfirmDialog.view
                { title = "却下の確認"
                , message = "この申請を却下しますか？"
                , confirmLabel = "却下する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmAction
                , onCancel = CancelAction
                , actionStyle = ConfirmDialog.Destructive
                }

        Just (ConfirmRequestChanges step) ->
            ConfirmDialog.view
                { title = "差し戻しの確認"
                , message = "ステップ「" ++ step.stepName ++ "」を差し戻しますか？"
                , confirmLabel = "差し戻す"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmAction
                , onCancel = CancelAction
                , actionStyle = ConfirmDialog.Caution
                }

        Nothing ->
            text ""



-- APPROVAL VIEWS


{-| 承認/却下セクション

現在のユーザーが担当者に割り当てられているアクティブなステップがある場合のみ表示。

-}
viewApprovalSection : WorkflowInstance -> String -> Bool -> Shared -> Html Msg
viewApprovalSection workflow comment isSubmitting shared =
    let
        currentUserId =
            Shared.getUserId shared
    in
    case findActiveStepForUser workflow.steps currentUserId of
        Just step ->
            div [ class "space-y-4 rounded-lg border border-secondary-200 bg-white p-4 shadow-sm" ]
                [ viewCommentInput comment
                , viewApprovalButtons step isSubmitting
                ]

        Nothing ->
            text ""


{-| コメント入力欄
-}
viewCommentInput : String -> Html Msg
viewCommentInput comment =
    div [ class "space-y-2" ]
        [ label [ for "approval-comment", class "block text-sm font-medium text-secondary-700" ] [ text "コメント（任意）" ]
        , textarea
            [ id "approval-comment"
            , class "w-full rounded-lg border border-secondary-300 bg-white px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
            , value comment
            , onInput UpdateComment
            , placeholder "承認/却下の理由を入力..."
            , rows 3
            ]
            []
        ]


{-| 承認/却下/差し戻しボタン
-}
viewApprovalButtons : WorkflowStep -> Bool -> Html Msg
viewApprovalButtons step isSubmitting =
    div [ class "flex gap-3" ]
        [ Button.view
            { variant = Button.Success
            , disabled = isSubmitting
            , onClick = ClickApprove step
            }
            [ text
                (if isSubmitting then
                    "処理中..."

                 else
                    "承認"
                )
            ]
        , Button.view
            { variant = Button.Warning
            , disabled = isSubmitting
            , onClick = ClickRequestChanges step
            }
            [ text
                (if isSubmitting then
                    "処理中..."

                 else
                    "差し戻し"
                )
            ]
        , Button.view
            { variant = Button.Error
            , disabled = isSubmitting
            , onClick = ClickReject step
            }
            [ text
                (if isSubmitting then
                    "処理中..."

                 else
                    "却下"
                )
            ]
        ]


{-| 現在のユーザーが担当のアクティブなステップを探す
-}
findActiveStepForUser : List WorkflowStep -> Maybe String -> Maybe WorkflowStep
findActiveStepForUser steps maybeUserId =
    case maybeUserId of
        Nothing ->
            Nothing

        Just userId ->
            steps
                |> List.filter
                    (\step ->
                        step.status == WorkflowInstance.StepActive && Maybe.map .id step.assignedTo == Just userId
                    )
                |> List.head


{-| ワークフローステップの一覧を表示
-}
viewSteps : WorkflowInstance -> Html Msg
viewSteps workflow =
    if List.isEmpty workflow.steps then
        text ""

    else
        div []
            [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "承認ステップ" ]
            , ul [ class "space-y-3 list-none pl-0" ]
                (List.map viewStep workflow.steps)
            ]


viewStep : WorkflowStep -> Html Msg
viewStep step =
    li [ class "rounded-lg border border-secondary-200 bg-white p-4" ]
        [ div [ class "flex items-center justify-between" ]
            [ span [ class "font-medium text-secondary-900" ]
                [ span [ class "text-secondary-400 mr-2" ] [ text step.displayId ]
                , text step.stepName
                ]
            , span [ class "text-sm text-secondary-500" ] [ text (WorkflowInstance.stepStatusToJapanese step.status) ]
            ]
        , div [ class "mt-2 flex flex-wrap gap-3 text-sm text-secondary-500" ]
            [ case step.assignedTo of
                Just assignee ->
                    span [] [ text ("担当: " ++ assignee.name) ]

                Nothing ->
                    text ""
            , case step.decision of
                Just decision ->
                    span [] [ text (WorkflowInstance.decisionToJapanese decision) ]

                Nothing ->
                    text ""
            , case step.comment of
                Just comment ->
                    span [] [ text ("コメント: " ++ comment) ]

                Nothing ->
                    text ""
            ]
        ]
