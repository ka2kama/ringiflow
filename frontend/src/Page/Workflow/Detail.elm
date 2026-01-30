module Page.Workflow.Detail exposing
    ( Model
    , Msg
    , init
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


## 設計

詳細: [申請フォーム UI 設計](../../../../docs/03_詳細設計書/10_ワークフロー申請フォームUI設計.md)

-}

import Api.Http exposing (ApiError(..))
import Api.Workflow as WorkflowApi
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Data.FormField exposing (FormField)
import Data.WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance as WorkflowInstance exposing (StepStatus(..), WorkflowInstance, WorkflowStep)
import Form.DynamicForm as DynamicForm
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick)
import Json.Decode as Decode
import Route
import Shared exposing (Shared)



-- MODEL


{-| ページの状態
-}
type alias Model =
    { -- 共有状態
      shared : Shared

    -- パラメータ
    , workflowId : String

    -- API データ
    , workflow : RemoteData WorkflowInstance
    , definition : RemoteData WorkflowDefinition

    -- 承認/却下の状態
    , isSubmitting : Bool
    , errorMessage : Maybe String
    , successMessage : Maybe String
    }


{-| リモートデータの状態
-}
type RemoteData a
    = NotAsked
    | Loading
    | Failure
    | Success a


{-| 初期化
-}
init : Shared -> String -> ( Model, Cmd Msg )
init shared workflowId =
    ( { shared = shared
      , workflowId = workflowId
      , workflow = Loading
      , definition = NotAsked
      , isSubmitting = False
      , errorMessage = Nothing
      , successMessage = Nothing
      }
    , WorkflowApi.getWorkflow
        { config = Shared.toRequestConfig shared
        , id = workflowId
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

                Err _ ->
                    ( { model | workflow = Failure }
                    , Cmd.none
                    )

        GotDefinition result ->
            case result of
                Ok definition ->
                    ( { model | definition = Success definition }
                    , Cmd.none
                    )

                Err _ ->
                    ( { model | definition = Failure }
                    , Cmd.none
                    )

        Refresh ->
            ( { model
                | workflow = Loading
                , definition = NotAsked
                , errorMessage = Nothing
                , successMessage = Nothing
              }
            , WorkflowApi.getWorkflow
                { config = Shared.toRequestConfig model.shared
                , id = model.workflowId
                , toMsg = GotWorkflow
                }
            )

        ClickApprove step ->
            ( { model | isSubmitting = True, errorMessage = Nothing }
            , WorkflowApi.approveStep
                { config = Shared.toRequestConfig model.shared
                , workflowId = model.workflowId
                , stepId = step.id
                , body = { version = step.version, comment = Nothing }
                , toMsg = GotApproveResult
                }
            )

        ClickReject step ->
            ( { model | isSubmitting = True, errorMessage = Nothing }
            , WorkflowApi.rejectStep
                { config = Shared.toRequestConfig model.shared
                , workflowId = model.workflowId
                , stepId = step.id
                , body = { version = step.version, comment = Nothing }
                , toMsg = GotRejectResult
                }
            )

        GotApproveResult result ->
            handleApprovalResult "承認しました" result model

        GotRejectResult result ->
            handleApprovalResult "却下しました" result model

        DismissMessage ->
            ( { model | errorMessage = Nothing, successMessage = Nothing }
            , Cmd.none
            )


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
              }
            , Cmd.none
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
            "このワークフローは既に更新されています。最新の状態を取得してください。（" ++ problem.detail ++ "）"

        Forbidden problem ->
            "この操作を実行する権限がありません。（" ++ problem.detail ++ "）"

        BadRequest problem ->
            problem.detail

        NotFound _ ->
            "ワークフローが見つかりません。"

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
    div [ class "workflow-detail-page" ]
        [ viewHeader
        , viewMessages model
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "page-header" ]
        [ a [ href (Route.toString Route.Workflows), class "back-link" ]
            [ text "← 一覧に戻る" ]
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
    case model.workflow of
        NotAsked ->
            div [] []

        Loading ->
            div [ class "loading" ] [ text "読み込み中..." ]

        Failure ->
            viewError

        Success workflow ->
            viewWorkflowDetail workflow model.definition model.isSubmitting model.shared


viewError : Html Msg
viewError =
    div [ class "error-message" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , button [ onClick Refresh, class "btn btn-secondary" ]
            [ text "再読み込み" ]
        ]


viewWorkflowDetail : WorkflowInstance -> RemoteData WorkflowDefinition -> Bool -> Shared -> Html Msg
viewWorkflowDetail workflow maybeDefinition isSubmitting shared =
    div [ class "workflow-detail" ]
        [ viewTitle workflow
        , viewStatus workflow
        , viewApprovalButtons workflow isSubmitting shared
        , hr [] []
        , viewSteps workflow
        , hr [] []
        , viewBasicInfo workflow
        , hr [] []
        , viewFormData workflow maybeDefinition
        ]


viewTitle : WorkflowInstance -> Html Msg
viewTitle workflow =
    h1 [ class "workflow-title" ] [ text workflow.title ]


viewStatus : WorkflowInstance -> Html Msg
viewStatus workflow =
    div [ class "workflow-status" ]
        [ text "ステータス: "
        , span [ class (WorkflowInstance.statusToCssClass workflow.status) ]
            [ text (WorkflowInstance.statusToJapanese workflow.status) ]
        ]


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


viewFormData : WorkflowInstance -> RemoteData WorkflowDefinition -> Html Msg
viewFormData workflow maybeDefinition =
    div [ class "form-data" ]
        [ h2 [] [ text "フォームデータ" ]
        , case maybeDefinition of
            NotAsked ->
                text ""

            Loading ->
                div [ class "loading" ] [ text "読み込み中..." ]

            Failure ->
                viewRawFormData workflow.formData

            Success definition ->
                viewFormDataWithLabels definition workflow.formData
        ]


viewFormDataWithLabels : WorkflowDefinition -> Decode.Value -> Html Msg
viewFormDataWithLabels definition formData =
    case DynamicForm.extractFormFields definition.definition of
        Ok fields ->
            dl []
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
    [ dt [] [ text field.label ]
    , dd []
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
    pre [ class "raw-json" ]
        [ text
            (Decode.decodeValue (Decode.keyValuePairs Decode.string) formData
                |> Result.map (List.map (\( k, v ) -> k ++ ": " ++ v) >> String.join "\n")
                |> Result.withDefault "（データなし）"
            )
        ]


formatDateTime : Maybe String -> String
formatDateTime maybeDateTime =
    case maybeDateTime of
        Nothing ->
            "-"

        Just dateTime ->
            -- ISO 8601 から日付と時刻を抽出（簡易実装）
            String.left 16 dateTime
                |> String.replace "T" " "



-- APPROVAL VIEWS


{-| 承認/却下ボタンを表示

現在のユーザーが担当者に割り当てられているアクティブなステップがある場合のみ表示。

-}
viewApprovalButtons : WorkflowInstance -> Bool -> Shared -> Html Msg
viewApprovalButtons workflow isSubmitting shared =
    let
        currentUserId =
            Shared.getUserId shared
    in
    case findActiveStepForUser workflow.steps currentUserId of
        Just step ->
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

        Nothing ->
            text ""


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
                        step.status == WorkflowInstance.StepActive && step.assignedTo == Just userId
                    )
                |> List.head


{-| ワークフローステップの一覧を表示
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
