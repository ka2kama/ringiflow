module Page.Task.Detail exposing
    ( Model
    , Msg
    , init
    , update
    , updateShared
    , view
    )

{-| タスク詳細ページ

タスク（承認ステップ）の詳細情報と、関連するワークフロー情報を表示する。
承認/却下操作が可能。


## 機能

  - タスク情報の表示（ステップ名、ステータス、担当者）
  - ワークフロー情報の表示（タイトル、申請者、フォームデータ）
  - 承認ステップの進捗表示
  - 承認/却下ボタン（Active なステップの場合のみ）
  - コメント入力欄

-}

import Api exposing (ApiError(..))
import Api.Task as TaskApi
import Api.Workflow as WorkflowApi
import Data.Task exposing (TaskDetail)
import Data.WorkflowInstance as WorkflowInstance
    exposing
        ( StepStatus(..)
        , WorkflowInstance
        , WorkflowStep
        )
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick, onInput)
import Json.Decode as Decode
import Route
import Shared exposing (Shared)



-- MODEL


{-| ページの状態
-}
type alias Model =
    { shared : Shared
    , taskId : String

    -- API データ
    , task : RemoteData TaskDetail

    -- 承認/却下の状態
    , comment : String
    , isSubmitting : Bool
    , errorMessage : Maybe String
    , successMessage : Maybe String
    }


{-| リモートデータの状態
-}
type RemoteData a
    = Loading
    | Failure
    | Success a


{-| 初期化
-}
init : Shared -> String -> ( Model, Cmd Msg )
init shared taskId =
    ( { shared = shared
      , taskId = taskId
      , task = Loading
      , comment = ""
      , isSubmitting = False
      , errorMessage = Nothing
      , successMessage = Nothing
      }
    , TaskApi.getTask
        { config = Shared.toRequestConfig shared
        , id = taskId
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
    | GotApproveResult (Result ApiError WorkflowInstance)
    | GotRejectResult (Result ApiError WorkflowInstance)
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

                Err _ ->
                    ( { model | task = Failure }
                    , Cmd.none
                    )

        Refresh ->
            ( { model
                | task = Loading
                , errorMessage = Nothing
                , successMessage = Nothing
              }
            , TaskApi.getTask
                { config = Shared.toRequestConfig model.shared
                , id = model.taskId
                , toMsg = GotTaskDetail
                }
            )

        UpdateComment comment ->
            ( { model | comment = comment }
            , Cmd.none
            )

        ClickApprove step ->
            ( { model | isSubmitting = True, errorMessage = Nothing }
            , approveStep model step
            )

        ClickReject step ->
            ( { model | isSubmitting = True, errorMessage = Nothing }
            , rejectStep model step
            )

        GotApproveResult result ->
            handleApprovalResult "承認しました" result model

        GotRejectResult result ->
            handleApprovalResult "却下しました" result model

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
                , workflowId = taskDetail.workflow.id
                , stepId = step.id
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
                , workflowId = taskDetail.workflow.id
                , stepId = step.id
                , body =
                    { version = step.version
                    , comment = nonEmptyComment model.comment
                    }
                , toMsg = GotRejectResult
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
            , TaskApi.getTask
                { config = Shared.toRequestConfig model.shared
                , id = model.taskId
                , toMsg = GotTaskDetail
                }
            )

        Err error ->
            ( { model
                | isSubmitting = False
                , errorMessage = Just (apiErrorToMessage error)
              }
            , Cmd.none
            )


{-| API エラーをユーザー向けメッセージに変換
-}
apiErrorToMessage : ApiError -> String
apiErrorToMessage error =
    case error of
        Conflict problem ->
            "このタスクは既に更新されています。最新の状態を取得してください。（" ++ problem.detail ++ "）"

        Forbidden problem ->
            "この操作を実行する権限がありません。（" ++ problem.detail ++ "）"

        BadRequest problem ->
            problem.detail

        NotFound _ ->
            "タスクが見つかりません。"

        Unauthorized ->
            "ログインが必要です。"

        ServerError _ ->
            "サーバーエラーが発生しました。"

        NetworkError ->
            "ネットワークエラーが発生しました。"

        Timeout ->
            "リクエストがタイムアウトしました。"

        DecodeError _ ->
            "データの処理中にエラーが発生しました。"



-- VIEW


{-| ビュー
-}
view : Model -> Html Msg
view model =
    div [ class "task-detail-page" ]
        [ viewHeader
        , viewMessages model
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "page-header" ]
        [ a [ href (Route.toString Route.Tasks), class "back-link" ]
            [ text "← タスク一覧に戻る" ]
        ]


viewMessages : Model -> Html Msg
viewMessages model =
    div [ class "messages" ]
        [ case model.successMessage of
            Just msg ->
                div [ class "alert alert-success" ]
                    [ text msg
                    , button [ class "alert-dismiss", onClick DismissMessage ] [ text "×" ]
                    ]

            Nothing ->
                text ""
        , case model.errorMessage of
            Just msg ->
                div [ class "alert alert-error" ]
                    [ text msg
                    , button [ class "alert-dismiss", onClick DismissMessage ] [ text "×" ]
                    ]

            Nothing ->
                text ""
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.task of
        Loading ->
            div [ class "loading" ] [ text "読み込み中..." ]

        Failure ->
            viewError

        Success taskDetail ->
            viewTaskDetail taskDetail model


viewError : Html Msg
viewError =
    div [ class "error-message" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , button [ onClick Refresh, class "btn btn-secondary" ]
            [ text "再読み込み" ]
        ]


viewTaskDetail : TaskDetail -> Model -> Html Msg
viewTaskDetail taskDetail model =
    div [ class "task-detail" ]
        [ viewWorkflowTitle taskDetail.workflow
        , viewWorkflowStatus taskDetail.workflow
        , viewApprovalSection taskDetail.step model
        , hr [] []
        , viewSteps taskDetail.workflow
        , hr [] []
        , viewBasicInfo taskDetail.workflow
        , hr [] []
        , viewFormData taskDetail.workflow
        ]


viewWorkflowTitle : WorkflowInstance -> Html Msg
viewWorkflowTitle workflow =
    h1 [ class "workflow-title" ] [ text workflow.title ]


viewWorkflowStatus : WorkflowInstance -> Html Msg
viewWorkflowStatus workflow =
    div [ class "workflow-status" ]
        [ text "ステータス: "
        , span [ class (WorkflowInstance.statusToCssClass workflow.status) ]
            [ text (WorkflowInstance.statusToJapanese workflow.status) ]
        ]



-- APPROVAL SECTION


{-| 承認/却下セクション

タスクのステップが Active な場合のみ承認/却下ボタンとコメント入力欄を表示。

-}
viewApprovalSection : WorkflowStep -> Model -> Html Msg
viewApprovalSection step model =
    if step.status == StepActive then
        div [ class "approval-section" ]
            [ viewCommentInput model.comment
            , viewApprovalButtons step model.isSubmitting
            ]

    else
        viewStepStatusBadge step


viewCommentInput : String -> Html Msg
viewCommentInput comment =
    div [ class "comment-input" ]
        [ label [ for "approval-comment" ] [ text "コメント（任意）" ]
        , textarea
            [ id "approval-comment"
            , value comment
            , onInput UpdateComment
            , placeholder "承認/却下の理由を入力..."
            , rows 3
            ]
            []
        ]


viewApprovalButtons : WorkflowStep -> Bool -> Html Msg
viewApprovalButtons step isSubmitting =
    div [ class "approval-buttons" ]
        [ button
            [ class "btn btn-success"
            , onClick (ClickApprove step)
            , disabled isSubmitting
            ]
            [ text
                (if isSubmitting then
                    "処理中..."

                 else
                    "承認"
                )
            ]
        , button
            [ class "btn btn-danger"
            , onClick (ClickReject step)
            , disabled isSubmitting
            ]
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
    div [ class "step-status-badge" ]
        [ text "このタスクのステータス: "
        , span [ class (stepStatusToCssClass step.status) ]
            [ text (WorkflowInstance.stepStatusToJapanese step.status) ]
        , case step.decision of
            Just decision ->
                span [ class "step-decision" ]
                    [ text (" — " ++ WorkflowInstance.decisionToJapanese decision) ]

            Nothing ->
                text ""
        ]



-- WORKFLOW INFO VIEWS


viewBasicInfo : WorkflowInstance -> Html Msg
viewBasicInfo workflow =
    div [ class "basic-info" ]
        [ h2 [] [ text "基本情報" ]
        , dl []
            [ dt [] [ text "申請者" ]
            , dd [] [ text workflow.initiatedBy ]
            , dt [] [ text "申請日" ]
            , dd [] [ text (formatDateTime workflow.submittedAt) ]
            , dt [] [ text "作成日" ]
            , dd [] [ text (formatDateTime (Just workflow.createdAt)) ]
            , dt [] [ text "更新日" ]
            , dd [] [ text (formatDateTime (Just workflow.updatedAt)) ]
            ]
        ]


viewFormData : WorkflowInstance -> Html Msg
viewFormData workflow =
    div [ class "form-data" ]
        [ h2 [] [ text "フォームデータ" ]
        , viewRawFormData workflow.formData
        ]


viewRawFormData : Decode.Value -> Html Msg
viewRawFormData formData =
    pre [ class "raw-json" ]
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
        div [ class "workflow-steps" ]
            [ h2 [] [ text "承認ステップ" ]
            , ul [ class "step-list" ]
                (List.map viewStep workflow.steps)
            ]


viewStep : WorkflowStep -> Html Msg
viewStep step =
    li [ class ("step-item step-" ++ stepStatusToCssClass step.status) ]
        [ div [ class "step-header" ]
            [ span [ class "step-name" ] [ text step.stepName ]
            , span [ class "step-status" ] [ text (WorkflowInstance.stepStatusToJapanese step.status) ]
            ]
        , div [ class "step-details" ]
            [ case step.assignedTo of
                Just assignee ->
                    span [ class "step-assignee" ] [ text ("担当: " ++ assignee) ]

                Nothing ->
                    text ""
            , case step.decision of
                Just decision ->
                    span [ class "step-decision" ] [ text (WorkflowInstance.decisionToJapanese decision) ]

                Nothing ->
                    text ""
            , case step.comment of
                Just comment ->
                    span [ class "step-comment" ] [ text ("コメント: " ++ comment) ]

                Nothing ->
                    text ""
            ]
        ]



-- HELPERS


stepStatusToCssClass : StepStatus -> String
stepStatusToCssClass status =
    case status of
        StepPending ->
            "pending"

        StepActive ->
            "active"

        StepCompleted ->
            "completed"

        StepSkipped ->
            "skipped"


formatDateTime : Maybe String -> String
formatDateTime maybeDateTime =
    case maybeDateTime of
        Nothing ->
            "-"

        Just dateTime ->
            String.left 16 dateTime
                |> String.replace "T" " "
