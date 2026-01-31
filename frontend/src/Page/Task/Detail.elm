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
    div []
        [ viewHeader
        , viewMessages model
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "mb-6" ]
        [ a [ href (Route.toString Route.Tasks), class "text-sm text-primary-600 hover:text-primary-700 hover:underline" ]
            [ text "← タスク一覧に戻る" ]
        ]


viewMessages : Model -> Html Msg
viewMessages model =
    div [ class "space-y-2 mb-4" ]
        [ case model.successMessage of
            Just msg ->
                div [ class "flex items-center justify-between rounded-lg bg-success-50 p-4 text-success-700" ]
                    [ text msg
                    , button [ class "ml-4 cursor-pointer bg-transparent border-0 text-lg", onClick DismissMessage ] [ text "×" ]
                    ]

            Nothing ->
                text ""
        , case model.errorMessage of
            Just msg ->
                div [ class "flex items-center justify-between rounded-lg bg-error-50 p-4 text-error-700" ]
                    [ text msg
                    , button [ class "ml-4 cursor-pointer bg-transparent border-0 text-lg", onClick DismissMessage ] [ text "×" ]
                    ]

            Nothing ->
                text ""
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.task of
        Loading ->
            div [ class "py-8 text-center text-secondary-500" ] [ text "読み込み中..." ]

        Failure ->
            viewError

        Success taskDetail ->
            viewTaskDetail taskDetail model


viewError : Html Msg
viewError =
    div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , button [ onClick Refresh, class "mt-2 inline-flex items-center rounded-lg border border-secondary-100 px-4 py-2 text-sm font-medium text-secondary-700 transition-colors hover:bg-secondary-50" ]
            [ text "再読み込み" ]
        ]


viewTaskDetail : TaskDetail -> Model -> Html Msg
viewTaskDetail taskDetail model =
    div [ class "space-y-6" ]
        [ viewWorkflowTitle taskDetail.workflow
        , viewWorkflowStatus taskDetail.workflow
        , viewApprovalSection taskDetail.step model
        , hr [ class "border-t border-secondary-100" ] []
        , viewSteps taskDetail.workflow
        , hr [ class "border-t border-secondary-100" ] []
        , viewBasicInfo taskDetail.workflow
        , hr [ class "border-t border-secondary-100" ] []
        , viewFormData taskDetail.workflow
        ]


viewWorkflowTitle : WorkflowInstance -> Html Msg
viewWorkflowTitle workflow =
    h1 [ class "text-2xl font-bold text-secondary-900" ] [ text workflow.title ]


viewWorkflowStatus : WorkflowInstance -> Html Msg
viewWorkflowStatus workflow =
    div [ class "text-secondary-700" ]
        [ text "ステータス: "
        , span [ class ("inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium " ++ WorkflowInstance.statusToCssClass workflow.status) ]
            [ text (WorkflowInstance.statusToJapanese workflow.status) ]
        ]



-- APPROVAL SECTION


{-| 承認/却下セクション

タスクのステップが Active な場合のみ承認/却下ボタンとコメント入力欄を表示。

-}
viewApprovalSection : WorkflowStep -> Model -> Html Msg
viewApprovalSection step model =
    if step.status == StepActive then
        div [ class "space-y-4 rounded-lg border border-secondary-100 p-4" ]
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
            , class "w-full rounded-lg border border-secondary-100 px-3 py-2 text-sm"
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
        [ button
            [ class "inline-flex items-center rounded-lg bg-success-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-success-700"
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
            [ class "inline-flex items-center rounded-lg bg-error-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-error-700"
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
    div [ class "text-secondary-700" ]
        [ text "このタスクのステータス: "
        , span [ class ("inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium " ++ stepStatusToCssClass step.status) ]
            [ text (WorkflowInstance.stepStatusToJapanese step.status) ]
        , case step.decision of
            Just decision ->
                span []
                    [ text (" — " ++ WorkflowInstance.decisionToJapanese decision) ]

            Nothing ->
                text ""
        ]



-- WORKFLOW INFO VIEWS


viewBasicInfo : WorkflowInstance -> Html Msg
viewBasicInfo workflow =
    div []
        [ h2 [ class "mb-3 text-lg font-semibold text-secondary-900" ] [ text "基本情報" ]
        , dl [ class "grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm" ]
            [ dt [ class "text-secondary-500" ] [ text "申請者" ]
            , dd [ class "text-secondary-900" ] [ text workflow.initiatedBy ]
            , dt [ class "text-secondary-500" ] [ text "申請日" ]
            , dd [ class "text-secondary-900" ] [ text (formatDateTime workflow.submittedAt) ]
            , dt [ class "text-secondary-500" ] [ text "作成日" ]
            , dd [ class "text-secondary-900" ] [ text (formatDateTime (Just workflow.createdAt)) ]
            , dt [ class "text-secondary-500" ] [ text "更新日" ]
            , dd [ class "text-secondary-900" ] [ text (formatDateTime (Just workflow.updatedAt)) ]
            ]
        ]


viewFormData : WorkflowInstance -> Html Msg
viewFormData workflow =
    div []
        [ h2 [ class "mb-3 text-lg font-semibold text-secondary-900" ] [ text "フォームデータ" ]
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
            [ h2 [ class "mb-3 text-lg font-semibold text-secondary-900" ] [ text "承認ステップ" ]
            , ul [ class "space-y-3 list-none pl-0" ]
                (List.map viewStep workflow.steps)
            ]


viewStep : WorkflowStep -> Html Msg
viewStep step =
    li [ class "rounded-lg border border-secondary-100 p-4" ]
        [ div [ class "flex items-center justify-between" ]
            [ span [ class "font-medium text-secondary-900" ] [ text step.stepName ]
            , span [ class "text-sm text-secondary-500" ] [ text (WorkflowInstance.stepStatusToJapanese step.status) ]
            ]
        , div [ class "mt-2 flex flex-wrap gap-3 text-sm text-secondary-500" ]
            [ case step.assignedTo of
                Just assignee ->
                    span [] [ text ("担当: " ++ assignee) ]

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



-- HELPERS


stepStatusToCssClass : StepStatus -> String
stepStatusToCssClass status =
    case status of
        StepPending ->
            "bg-gray-100 text-gray-600"

        StepActive ->
            "bg-warning-50 text-warning-600"

        StepCompleted ->
            "bg-success-50 text-success-600"

        StepSkipped ->
            "bg-secondary-100 text-secondary-500"


formatDateTime : Maybe String -> String
formatDateTime maybeDateTime =
    case maybeDateTime of
        Nothing ->
            "-"

        Just dateTime ->
            String.left 16 dateTime
                |> String.replace "T" " "
