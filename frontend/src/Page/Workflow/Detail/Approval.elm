module Page.Workflow.Detail.Approval exposing (updateApproval, viewApprovalSection, viewConfirmDialog)

{-| 承認操作

承認/却下/差し戻しの操作 UI と確認ダイアログを管理する。

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.Workflow as WorkflowApi
import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Data.WorkflowInstance exposing (WorkflowInstance, WorkflowStep)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)
import Page.Workflow.Detail.Types exposing (LoadedState, Msg(..), PendingAction(..))
import Ports
import Shared exposing (Shared)



-- UPDATE


{-| 承認関連メッセージの処理
-}
updateApproval : Msg -> Shared -> Int -> LoadedState -> ( LoadedState, Cmd Msg )
updateApproval msg shared workflowDisplayNumber loaded =
    case msg of
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

        _ ->
            ( loaded, Cmd.none )



-- HELPERS


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



-- VIEW


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
                        step.status == Data.WorkflowInstance.StepActive && Maybe.map .id step.assignedTo == Just userId
                    )
                |> List.head



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
