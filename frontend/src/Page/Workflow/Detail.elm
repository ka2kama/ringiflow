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

Model は型安全ステートマシン（[ADR-054](../../../../docs/05_ADR/054_型安全ステートマシンパターンの標準化.md)
パターン A）で Loading/Failed/Loaded を分離し、Loaded 時のみ操作フィールドが存在する。

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
import Component.ErrorState as ErrorState
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


{-| ページの状態（ADR-054 パターン A: 外側に共通フィールド）
-}
type alias Model =
    { shared : Shared
    , workflowDisplayNumber : Int
    , state : PageState
    }


{-| ページの状態遷移
-}
type PageState
    = Loading
    | Failed ApiError
    | Loaded LoadedState


{-| Loaded 時のみ存在するフィールド

workflow 取得完了後の状態でのみ有効なフィールドを集約する。
definition/comments は Loaded 後も非同期ロード中のため RemoteData を維持。

-}
type alias LoadedState =
    { workflow : WorkflowInstance
    , definition : RemoteData ApiError WorkflowDefinition

    -- 承認/却下/差し戻し
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


{-| LoadedState の初期値を構築

GotWorkflow Ok 受信時、workflow から LoadedState を生成する。
definition は後続の GotDefinition で、comments は後続の GotComments で更新される。

-}
initLoaded : WorkflowInstance -> LoadedState
initLoaded workflow =
    { workflow = workflow
    , definition = RemoteData.Loading
    , comment = ""
    , isSubmitting = False
    , pendingAction = Nothing
    , errorMessage = Nothing
    , successMessage = Nothing
    , comments = RemoteData.Loading
    , newCommentBody = ""
    , isPostingComment = False
    , isEditing = False
    , editFormData = Dict.empty
    , editApprovers = Dict.empty
    , users = NotAsked
    , resubmitValidationErrors = Dict.empty
    , isResubmitting = False
    }


{-| 初期化
-}
init : Shared -> Int -> ( Model, Cmd Msg )
init shared workflowDisplayNumber =
    ( { shared = shared
      , workflowDisplayNumber = workflowDisplayNumber
      , state = Loading
      }
    , WorkflowApi.getWorkflow
        { config = Shared.toRequestConfig shared
        , displayNumber = workflowDisplayNumber
        , toMsg = GotWorkflow
        }
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

状態遷移メッセージ（GotWorkflow, Refresh）は外側で処理し、
操作メッセージは updateLoaded に委譲する。

-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotWorkflow result ->
            handleGotWorkflow result model

        Refresh ->
            ( { model | state = Loading }
            , WorkflowApi.getWorkflow
                { config = Shared.toRequestConfig model.shared
                , displayNumber = model.workflowDisplayNumber
                , toMsg = GotWorkflow
                }
            )

        _ ->
            case model.state of
                Loaded loaded ->
                    let
                        ( newLoaded, cmd ) =
                            updateLoaded msg model.shared model.workflowDisplayNumber loaded
                    in
                    ( { model | state = Loaded newLoaded }, cmd )

                _ ->
                    ( model, Cmd.none )


{-| ワークフロー取得結果のハンドリング

初回ロード時（Loading/Failed → Loaded）は新しい LoadedState を構築し、
definition と comments の fetch を並列発行する。
Refresh 後の再取得もこのパスを通る（Refresh で Loading に遷移するため）。

-}
handleGotWorkflow : Result ApiError WorkflowInstance -> Model -> ( Model, Cmd Msg )
handleGotWorkflow result model =
    case result of
        Ok workflow ->
            case model.state of
                Loaded loaded ->
                    ( { model | state = Loaded { loaded | workflow = workflow } }
                    , Cmd.none
                    )

                _ ->
                    ( { model | state = Loaded (initLoaded workflow) }
                    , Cmd.batch
                        [ WorkflowDefinitionApi.getDefinition
                            { config = Shared.toRequestConfig model.shared
                            , id = workflow.definitionId
                            , toMsg = GotDefinition
                            }
                        , WorkflowApi.listComments
                            { config = Shared.toRequestConfig model.shared
                            , displayNumber = model.workflowDisplayNumber
                            , toMsg = GotComments
                            }
                        ]
                    )

        Err err ->
            ( { model | state = Failed err }
            , Cmd.none
            )


{-| Loaded 状態専用の状態更新

GotWorkflow/Refresh 以外のすべてのメッセージを処理する。

-}
updateLoaded : Msg -> Shared -> Int -> LoadedState -> ( LoadedState, Cmd Msg )
updateLoaded msg shared workflowDisplayNumber loaded =
    case msg of
        GotDefinition result ->
            case result of
                Ok definition ->
                    ( { loaded | definition = Success definition }, Cmd.none )

                Err err ->
                    ( { loaded | definition = Failure err }, Cmd.none )

        GotComments result ->
            case result of
                Ok comments ->
                    ( { loaded | comments = Success comments }, Cmd.none )

                Err err ->
                    ( { loaded | comments = Failure err }, Cmd.none )

        UpdateComment newComment ->
            ( { loaded | comment = newComment }, Cmd.none )

        ClickApprove step ->
            ( { loaded | pendingAction = Just (ConfirmApprove step) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ClickReject step ->
            ( { loaded | pendingAction = Just (ConfirmReject step) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ClickRequestChanges step ->
            ( { loaded | pendingAction = Just (ConfirmRequestChanges step) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ConfirmAction ->
            case loaded.pendingAction of
                Just (ConfirmApprove step) ->
                    ( { loaded | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , WorkflowApi.approveStep
                        { config = Shared.toRequestConfig shared
                        , workflowDisplayNumber = workflowDisplayNumber
                        , stepDisplayNumber = step.displayNumber
                        , body = { version = step.version, comment = nonEmptyComment loaded.comment }
                        , toMsg = GotApproveResult
                        }
                    )

                Just (ConfirmReject step) ->
                    ( { loaded | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , WorkflowApi.rejectStep
                        { config = Shared.toRequestConfig shared
                        , workflowDisplayNumber = workflowDisplayNumber
                        , stepDisplayNumber = step.displayNumber
                        , body = { version = step.version, comment = nonEmptyComment loaded.comment }
                        , toMsg = GotRejectResult
                        }
                    )

                Just (ConfirmRequestChanges step) ->
                    ( { loaded | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , WorkflowApi.requestChangesStep
                        { config = Shared.toRequestConfig shared
                        , workflowDisplayNumber = workflowDisplayNumber
                        , stepDisplayNumber = step.displayNumber
                        , body = { version = step.version, comment = nonEmptyComment loaded.comment }
                        , toMsg = GotRequestChangesResult
                        }
                    )

                Nothing ->
                    ( loaded, Cmd.none )

        CancelAction ->
            ( { loaded | pendingAction = Nothing }
            , Cmd.none
            )

        GotApproveResult result ->
            handleApprovalResult "承認しました" result loaded

        GotRejectResult result ->
            handleApprovalResult "却下しました" result loaded

        GotRequestChangesResult result ->
            handleApprovalResult "差し戻しました" result loaded

        UpdateNewComment body ->
            ( { loaded | newCommentBody = body }, Cmd.none )

        SubmitComment ->
            if String.isEmpty (String.trim loaded.newCommentBody) then
                ( loaded, Cmd.none )

            else
                ( { loaded | isPostingComment = True }
                , WorkflowApi.postComment
                    { config = Shared.toRequestConfig shared
                    , displayNumber = workflowDisplayNumber
                    , body = { body = String.trim loaded.newCommentBody }
                    , toMsg = GotPostCommentResult
                    }
                )

        GotPostCommentResult result ->
            case result of
                Ok newComment ->
                    let
                        updatedComments =
                            case loaded.comments of
                                Success existing ->
                                    Success (existing ++ [ newComment ])

                                _ ->
                                    Success [ newComment ]
                    in
                    ( { loaded
                        | comments = updatedComments
                        , newCommentBody = ""
                        , isPostingComment = False
                      }
                    , Cmd.none
                    )

                Err _ ->
                    ( { loaded
                        | isPostingComment = False
                        , errorMessage = Just "コメントの投稿に失敗しました。"
                      }
                    , Cmd.none
                    )

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
                | isEditing = True
                , editFormData = formDataDict
                , editApprovers = approverStates
                , resubmitValidationErrors = Dict.empty
                , users = RemoteData.Loading
              }
            , UserApi.listUsers
                { config = Shared.toRequestConfig shared
                , toMsg = GotUsers
                }
            )

        CancelEditing ->
            ( { loaded
                | isEditing = False
                , editFormData = Dict.empty
                , editApprovers = Dict.empty
                , resubmitValidationErrors = Dict.empty
              }
            , Cmd.none
            )

        UpdateEditFormField fieldId fieldValue ->
            ( { loaded | editFormData = Dict.insert fieldId fieldValue loaded.editFormData }
            , Cmd.none
            )

        EditApproverSearchChanged stepId search ->
            ( { loaded | editApprovers = updateApproverState stepId (\s -> { s | search = search, dropdownOpen = True, highlightIndex = 0 }) loaded.editApprovers }
            , Cmd.none
            )

        EditApproverSelected stepId user ->
            ( { loaded | editApprovers = updateApproverState stepId (\s -> { s | selection = Selected user, search = "", dropdownOpen = False }) loaded.editApprovers }
            , Cmd.none
            )

        EditApproverCleared stepId ->
            ( { loaded | editApprovers = updateApproverState stepId (\_ -> ApproverSelector.init) loaded.editApprovers }
            , Cmd.none
            )

        EditApproverKeyDown stepId key ->
            case Dict.get stepId loaded.editApprovers of
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
                            ( { loaded | editApprovers = updateApproverState stepId (\s -> { s | highlightIndex = newIndex }) loaded.editApprovers }, Cmd.none )

                        ApproverSelector.Select user ->
                            ( { loaded | editApprovers = updateApproverState stepId (\s -> { s | selection = Selected user, search = "", dropdownOpen = False }) loaded.editApprovers }, Cmd.none )

                        ApproverSelector.Close ->
                            ( { loaded | editApprovers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) loaded.editApprovers }, Cmd.none )

                        ApproverSelector.NoChange ->
                            ( loaded, Cmd.none )

                Nothing ->
                    ( loaded, Cmd.none )

        EditApproverDropdownClosed stepId ->
            ( { loaded | editApprovers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) loaded.editApprovers }
            , Cmd.none
            )

        SubmitResubmit ->
            let
                validationErrors =
                    validateResubmit loaded

                approvers =
                    buildResubmitApprovers loaded
            in
            if Dict.isEmpty validationErrors then
                ( { loaded | isResubmitting = True, resubmitValidationErrors = Dict.empty, errorMessage = Nothing }
                , WorkflowApi.resubmitWorkflow
                    { config = Shared.toRequestConfig shared
                    , displayNumber = workflowDisplayNumber
                    , body =
                        { version = loaded.workflow.version
                        , formData = encodeFormValues loaded.editFormData
                        , approvers = approvers
                        }
                    , toMsg = GotResubmitResult
                    }
                )

            else
                ( { loaded | resubmitValidationErrors = validationErrors }, Cmd.none )

        GotResubmitResult result ->
            case result of
                Ok workflow ->
                    ( { loaded
                        | workflow = workflow
                        , isEditing = False
                        , isResubmitting = False
                        , editFormData = Dict.empty
                        , editApprovers = Dict.empty
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
                    ( { loaded
                        | isResubmitting = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー" } error)
                      }
                    , Cmd.none
                    )

        GotUsers result ->
            case result of
                Ok users ->
                    ( { loaded | users = Success users }, Cmd.none )

                Err err ->
                    ( { loaded | users = Failure err }, Cmd.none )

        DismissMessage ->
            ( { loaded | errorMessage = Nothing, successMessage = Nothing }
            , Cmd.none
            )

        _ ->
            ( loaded, Cmd.none )


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
handleApprovalResult : String -> Result ApiError WorkflowInstance -> LoadedState -> ( LoadedState, Cmd Msg )
handleApprovalResult successMsg result loaded =
    case result of
        Ok workflow ->
            ( { loaded
                | workflow = workflow
                , isSubmitting = False
                , successMessage = Just successMsg
                , errorMessage = Nothing
                , comment = ""
              }
            , Cmd.none
            )

        Err error ->
            ( { loaded
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
validateResubmit : LoadedState -> Dict String String
validateResubmit loaded =
    let
        approverErrors =
            loaded.editApprovers
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
buildResubmitApprovers : LoadedState -> List WorkflowApi.StepApproverRequest
buildResubmitApprovers loaded =
    loaded.editApprovers
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
        , viewBody model
        ]


{-| 状態に応じたコンテンツ描画
-}
viewBody : Model -> Html Msg
viewBody model =
    case model.state of
        Loading ->
            LoadingSpinner.view

        Failed err ->
            viewError err

        Loaded loaded ->
            viewLoaded model.shared loaded


{-| Loaded 状態のビュー
-}
viewLoaded : Shared -> LoadedState -> Html Msg
viewLoaded shared loaded =
    div []
        [ MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = loaded.successMessage
            , errorMessage = loaded.errorMessage
            }
        , viewWorkflowDetail shared loaded
        , viewConfirmDialog loaded.pendingAction
        ]


viewHeader : Html Msg
viewHeader =
    nav [ class "mb-4 flex items-center gap-2 text-sm" ]
        [ a [ href (Route.toString (Route.Workflows Route.emptyWorkflowFilter)), class "text-secondary-500 hover:text-primary-600 transition-colors" ] [ text "申請一覧" ]
        , span [ class "text-secondary-400" ] [ text "/" ]
        , span [ class "text-secondary-900 font-medium" ] [ text "申請詳細" ]
        ]


viewError : ApiError -> Html Msg
viewError err =
    ErrorState.view
        { message = ErrorMessage.toUserMessage { entityName = "ワークフロー" } err
        , onRefresh = Refresh
        }


viewWorkflowDetail : Shared -> LoadedState -> Html Msg
viewWorkflowDetail shared loaded =
    div [ class "space-y-6" ]
        [ viewTitle loaded.workflow
        , viewStatus loaded.workflow
        , viewStepProgress loaded.workflow
        , viewApprovalSection loaded.workflow loaded.comment loaded.isSubmitting shared
        , viewResubmitSection shared loaded
        , viewSteps loaded.workflow
        , viewBasicInfo (Shared.zone shared) loaded.workflow
        , if loaded.isEditing then
            viewEditableFormData loaded

          else
            viewFormData loaded.workflow loaded.definition
        , viewCommentSection loaded
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
    div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]
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
    if isChangesRequested && isInitiator && not loaded.isEditing then
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
viewEditableFormData : LoadedState -> Html Msg
viewEditableFormData loaded =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "フォームデータ（編集中）" ]
        , case loaded.definition of
            Success definition ->
                case DynamicForm.extractFormFields definition.definition of
                    Ok fields ->
                        div [ class "space-y-4" ]
                            (List.map (viewEditableFormField loaded.editFormData) fields
                                ++ [ viewEditableApprovers loaded definition
                                   , viewEditActions loaded
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


viewEditableApprovers : LoadedState -> WorkflowDefinition -> Html Msg
viewEditableApprovers loaded definition =
    let
        stepInfos =
            WorkflowDefinition.approvalStepInfos definition
    in
    div [ class "space-y-3" ]
        [ h3 [ class "text-sm font-semibold text-secondary-700" ] [ text "承認者" ]
        , div [ class "space-y-3" ]
            (List.map (viewEditableApproverStep loaded) stepInfos)
        ]


viewEditableApproverStep : LoadedState -> WorkflowDefinition.ApprovalStepInfo -> Html Msg
viewEditableApproverStep loaded stepInfo =
    let
        state =
            Dict.get stepInfo.id loaded.editApprovers
                |> Maybe.withDefault ApproverSelector.init
    in
    div [ class "space-y-1" ]
        [ label [ class "block text-sm font-medium text-secondary-600" ] [ text stepInfo.name ]
        , ApproverSelector.view
            { state = state
            , users = loaded.users
            , validationError = Dict.get ("approver_" ++ stepInfo.id) loaded.resubmitValidationErrors
            , onSearch = EditApproverSearchChanged stepInfo.id
            , onSelect = EditApproverSelected stepInfo.id
            , onClear = EditApproverCleared stepInfo.id
            , onKeyDown = EditApproverKeyDown stepInfo.id
            , onCloseDropdown = EditApproverDropdownClosed stepInfo.id
            }
        ]


viewEditActions : LoadedState -> Html Msg
viewEditActions loaded =
    div [ class "flex gap-3 pt-4 border-t border-secondary-100" ]
        [ Button.view
            { variant = Button.Primary
            , disabled = loaded.isResubmitting
            , onClick = SubmitResubmit
            }
            [ text
                (if loaded.isResubmitting then
                    "再申請中..."

                 else
                    "再申請する"
                )
            ]
        , Button.view
            { variant = Button.Outline
            , disabled = loaded.isResubmitting
            , onClick = CancelEditing
            }
            [ text "キャンセル" ]
        ]


viewFormData : WorkflowInstance -> RemoteData ApiError WorkflowDefinition -> Html Msg
viewFormData workflow maybeDefinition =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "フォームデータ" ]
        , case maybeDefinition of
            NotAsked ->
                text ""

            RemoteData.Loading ->
                LoadingSpinner.view

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
        fieldValue =
            Decode.decodeValue (Decode.field field.id Decode.string) formData
                |> Result.withDefault ""
    in
    [ dt [ class "text-secondary-500" ] [ text field.label ]
    , dd [ class "text-secondary-900" ]
        [ text
            (if String.isEmpty fieldValue then
                "-"

             else
                fieldValue
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
viewCommentSection : LoadedState -> Html Msg
viewCommentSection loaded =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "コメント" ]
        , case loaded.comments of
            NotAsked ->
                text ""

            RemoteData.Loading ->
                LoadingSpinner.view

            Failure _ ->
                div [ class "rounded-lg bg-error-50 p-3 text-sm text-error-700" ]
                    [ text "コメントの取得に失敗しました。" ]

            Success comments ->
                div [ class "space-y-4" ]
                    [ viewCommentList comments
                    , viewCommentForm loaded.newCommentBody loaded.isPostingComment
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
                Just stepComment ->
                    span [] [ text ("コメント: " ++ stepComment) ]

                Nothing ->
                    text ""
            ]
        ]
