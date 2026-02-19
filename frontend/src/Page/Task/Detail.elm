module Page.Task.Detail exposing
    ( Model
    , Msg
    , init
    , subscriptions
    , update
    , updateShared
    , view
    )

{-| タスク詳細ページ

タスク（承認ステップ）の詳細情報と、関連するワークフロー情報を表示する。
承認/却下/差し戻し操作が可能。


## 機能

  - タスク情報の表示（ステップ名、ステータス、担当者）
  - ワークフロー情報の表示（タイトル、申請者、フォームデータ）
  - 承認ステップの進捗表示
  - 承認/却下/差し戻しボタン（Active なステップの場合のみ）
  - コメント入力欄
  - 承認/却下/差し戻し時の確認ダイアログ

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.Task as TaskApi
import Api.Workflow as WorkflowApi
import Component.Badge as Badge
import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Component.ErrorState as ErrorState
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.Task exposing (TaskDetail)
import Data.WorkflowInstance as WorkflowInstance
    exposing
        ( StepStatus(..)
        , WorkflowInstance
        , WorkflowStep
        )
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)
import Json.Decode as Decode
import Ports
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)
import Time
import Util.DateFormat as DateFormat



-- MODEL


{-| 確認待ちの操作

承認/却下/差し戻しボタンクリック後、確認ダイアログで最終確認するまで保持する。

-}
type PendingAction
    = ConfirmApprove WorkflowStep
    | ConfirmReject WorkflowStep
    | ConfirmRequestChanges WorkflowStep


{-| ページの状態
-}
type alias Model =
    { shared : Shared
    , workflowDisplayNumber : Int
    , stepDisplayNumber : Int

    -- API データ
    , task : RemoteData ApiError TaskDetail

    -- 承認/却下/差し戻しの状態
    , comment : String
    , isSubmitting : Bool
    , pendingAction : Maybe PendingAction
    , errorMessage : Maybe String
    , successMessage : Maybe String
    }


{-| 初期化
-}
init : Shared -> Int -> Int -> ( Model, Cmd Msg )
init shared workflowDisplayNumber stepDisplayNumber =
    ( { shared = shared
      , workflowDisplayNumber = workflowDisplayNumber
      , stepDisplayNumber = stepDisplayNumber
      , task = Loading
      , comment = ""
      , isSubmitting = False
      , pendingAction = Nothing
      , errorMessage = Nothing
      , successMessage = Nothing
      }
    , TaskApi.getTaskByDisplayNumbers
        { config = Shared.toRequestConfig shared
        , workflowDisplayNumber = workflowDisplayNumber
        , stepDisplayNumber = stepDisplayNumber
        , toMsg = GotTaskDetail
        }
    )


{-| 共有状態を更新
-}
updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


{-| メッセージ
-}
type Msg
    = GotTaskDetail (Result ApiError TaskDetail)
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
    | DismissMessage


{-| 状態更新
-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotTaskDetail result ->
            case result of
                Ok taskDetail ->
                    ( { model | task = Success taskDetail }
                    , Cmd.none
                    )

                Err err ->
                    ( { model | task = Failure err }
                    , Cmd.none
                    )

        Refresh ->
            ( { model
                | task = Loading
                , errorMessage = Nothing
                , successMessage = Nothing
              }
            , TaskApi.getTaskByDisplayNumbers
                { config = Shared.toRequestConfig model.shared
                , workflowDisplayNumber = model.workflowDisplayNumber
                , stepDisplayNumber = model.stepDisplayNumber
                , toMsg = GotTaskDetail
                }
            )

        UpdateComment comment ->
            ( { model | comment = comment }
            , Cmd.none
            )

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
                    , approveStep model step
                    )

                Just (ConfirmReject step) ->
                    ( { model | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , rejectStep model step
                    )

                Just (ConfirmRequestChanges step) ->
                    ( { model | pendingAction = Nothing, isSubmitting = True, errorMessage = Nothing }
                    , requestChangesStep model step
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

        DismissMessage ->
            ( { model | errorMessage = Nothing, successMessage = Nothing }
            , Cmd.none
            )


{-| 承認 API 呼び出し
-}
approveStep : Model -> WorkflowStep -> Cmd Msg
approveStep model step =
    case model.task of
        Success taskDetail ->
            WorkflowApi.approveStep
                { config = Shared.toRequestConfig model.shared
                , workflowDisplayNumber = taskDetail.workflow.displayNumber
                , stepDisplayNumber = step.displayNumber
                , body =
                    { version = step.version
                    , comment = nonEmptyComment model.comment
                    }
                , toMsg = GotApproveResult
                }

        _ ->
            Cmd.none


{-| 却下 API 呼び出し
-}
rejectStep : Model -> WorkflowStep -> Cmd Msg
rejectStep model step =
    case model.task of
        Success taskDetail ->
            WorkflowApi.rejectStep
                { config = Shared.toRequestConfig model.shared
                , workflowDisplayNumber = taskDetail.workflow.displayNumber
                , stepDisplayNumber = step.displayNumber
                , body =
                    { version = step.version
                    , comment = nonEmptyComment model.comment
                    }
                , toMsg = GotRejectResult
                }

        _ ->
            Cmd.none


{-| 差し戻し API 呼び出し
-}
requestChangesStep : Model -> WorkflowStep -> Cmd Msg
requestChangesStep model step =
    case model.task of
        Success taskDetail ->
            WorkflowApi.requestChangesStep
                { config = Shared.toRequestConfig model.shared
                , workflowDisplayNumber = taskDetail.workflow.displayNumber
                , stepDisplayNumber = step.displayNumber
                , body =
                    { version = step.version
                    , comment = nonEmptyComment model.comment
                    }
                , toMsg = GotRequestChangesResult
                }

        _ ->
            Cmd.none


{-| 空文字列を Nothing に変換
-}
nonEmptyComment : String -> Maybe String
nonEmptyComment comment =
    if String.isEmpty (String.trim comment) then
        Nothing

    else
        Just (String.trim comment)


{-| 承認/却下結果のハンドリング

成功時はタスク詳細を再読み込みして最新の状態を反映する。

-}
handleApprovalResult : String -> Result ApiError WorkflowInstance -> Model -> ( Model, Cmd Msg )
handleApprovalResult successMsg result model =
    case result of
        Ok _ ->
            ( { model
                | isSubmitting = False
                , successMessage = Just successMsg
                , errorMessage = Nothing
                , comment = ""
              }
            , TaskApi.getTaskByDisplayNumbers
                { config = Shared.toRequestConfig model.shared
                , workflowDisplayNumber = model.workflowDisplayNumber
                , stepDisplayNumber = model.stepDisplayNumber
                , toMsg = GotTaskDetail
                }
            )

        Err error ->
            ( { model
                | isSubmitting = False
                , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "タスク" } error)
              }
            , Cmd.none
            )



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
    nav [ class "mb-4 flex items-center gap-2 text-sm" ]
        [ a [ href (Route.toString Route.Tasks), class "text-secondary-500 hover:text-primary-600 transition-colors" ] [ text "タスク一覧" ]
        , span [ class "text-secondary-400" ] [ text "/" ]
        , span [ class "text-secondary-900 font-medium" ] [ text "タスク詳細" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.task of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure _ ->
            viewError

        Success taskDetail ->
            viewTaskDetail taskDetail model


viewError : Html Msg
viewError =
    ErrorState.view
        { message = "データの取得に失敗しました。"
        , onRefresh = Refresh
        }


viewTaskDetail : TaskDetail -> Model -> Html Msg
viewTaskDetail taskDetail model =
    div [ class "space-y-6" ]
        [ viewWorkflowTitle taskDetail.workflow
        , viewWorkflowStatus taskDetail.workflow
        , viewApprovalSection taskDetail.step model
        , viewSteps taskDetail.workflow
        , viewBasicInfo (Shared.zone model.shared) taskDetail.workflow
        , viewFormData taskDetail.workflow
        ]


viewWorkflowTitle : WorkflowInstance -> Html Msg
viewWorkflowTitle workflow =
    h1 [ class "text-2xl font-bold text-secondary-900" ] [ text workflow.title ]


viewWorkflowStatus : WorkflowInstance -> Html Msg
viewWorkflowStatus workflow =
    div [ class "text-secondary-700" ]
        [ text "ステータス: "
        , Badge.view
            { colorClass = WorkflowInstance.statusToCssClass workflow.status
            , label = WorkflowInstance.statusToJapanese workflow.status
            }
        ]



-- APPROVAL SECTION


{-| 承認/却下/差し戻しセクション

タスクのステップが Active な場合のみ承認/却下/差し戻しボタンとコメント入力欄を表示。

-}
viewApprovalSection : WorkflowStep -> Model -> Html Msg
viewApprovalSection step model =
    if step.status == StepActive then
        div [ class "space-y-4 rounded-lg border border-secondary-200 bg-white p-4 shadow-sm" ]
            [ viewCommentInput model.comment
            , viewApprovalButtons step model.isSubmitting
            ]

    else
        viewStepStatusBadge step


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


viewStepStatusBadge : WorkflowStep -> Html Msg
viewStepStatusBadge step =
    div [ class "text-secondary-700" ]
        [ text "このタスクのステータス: "
        , Badge.view
            { colorClass = WorkflowInstance.stepStatusToCssClass step.status
            , label = WorkflowInstance.stepStatusToJapanese step.status
            }
        , case step.decision of
            Just decision ->
                span []
                    [ text (" — " ++ WorkflowInstance.decisionToJapanese decision) ]

            Nothing ->
                text ""
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

        Just (ConfirmRequestChanges _) ->
            ConfirmDialog.view
                { title = "差し戻しの確認"
                , message = "この申請を差し戻しますか？"
                , confirmLabel = "差し戻す"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmAction
                , onCancel = CancelAction
                , actionStyle = ConfirmDialog.Caution
                }

        Nothing ->
            text ""



-- WORKFLOW INFO VIEWS


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


viewFormData : WorkflowInstance -> Html Msg
viewFormData workflow =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "フォームデータ" ]
        , viewRawFormData workflow.formData
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



-- STEPS VIEW


{-| ワークフローステップの進捗表示
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
            [ span [ class "font-medium text-secondary-900" ] [ text step.stepName ]
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
