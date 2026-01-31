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

import Api exposing (ApiError(..))
import Api.Workflow as WorkflowApi
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Data.FormField exposing (FormField)
import Data.WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance as WorkflowInstance exposing (WorkflowInstance, WorkflowStep)
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
    div []
        [ viewHeader
        , viewMessages model
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    nav [ class "mb-6 flex items-center gap-2 text-sm" ]
        [ a [ href (Route.toString Route.Home), class "text-secondary-500 hover:text-primary-600 transition-colors" ] [ text "ダッシュボード" ]
        , span [ class "text-secondary-400" ] [ text "/" ]
        , a [ href (Route.toString Route.Workflows), class "text-primary-600 hover:text-primary-700 hover:underline" ] [ text "申請一覧" ]
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
    case model.workflow of
        NotAsked ->
            div [] []

        Loading ->
            div [ class "flex flex-col items-center justify-center py-8" ]
                [ div [ class "h-8 w-8 animate-spin rounded-full border-4 border-secondary-100 border-t-primary-600" ] []
                , p [ class "mt-4 text-secondary-500" ] [ text "読み込み中..." ]
                ]

        Failure ->
            viewError

        Success workflow ->
            viewWorkflowDetail workflow model.definition model.isSubmitting model.shared


viewError : Html Msg
viewError =
    div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , button [ onClick Refresh, class "mt-2 inline-flex items-center rounded-lg border border-secondary-100 px-4 py-2 text-sm font-medium text-secondary-700 transition-colors hover:bg-secondary-50" ]
            [ text "再読み込み" ]
        ]


viewWorkflowDetail : WorkflowInstance -> RemoteData WorkflowDefinition -> Bool -> Shared -> Html Msg
viewWorkflowDetail workflow maybeDefinition isSubmitting shared =
    div [ class "space-y-6" ]
        [ viewTitle workflow
        , viewStatus workflow
        , viewApprovalButtons workflow isSubmitting shared
        , viewSteps workflow
        , viewBasicInfo workflow
        , viewFormData workflow maybeDefinition
        ]


viewTitle : WorkflowInstance -> Html Msg
viewTitle workflow =
    h1 [ class "text-2xl font-bold text-secondary-900" ] [ text workflow.title ]


viewStatus : WorkflowInstance -> Html Msg
viewStatus workflow =
    div [ class "text-secondary-700" ]
        [ text "ステータス: "
        , span [ class ("inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium " ++ WorkflowInstance.statusToCssClass workflow.status) ]
            [ text (WorkflowInstance.statusToJapanese workflow.status) ]
        ]


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


viewFormData : WorkflowInstance -> RemoteData WorkflowDefinition -> Html Msg
viewFormData workflow maybeDefinition =
    div []
        [ h2 [ class "mb-3 text-lg font-semibold text-secondary-900" ] [ text "フォームデータ" ]
        , case maybeDefinition of
            NotAsked ->
                text ""

            Loading ->
                div [ class "flex flex-col items-center justify-center py-8" ]
                    [ div [ class "h-8 w-8 animate-spin rounded-full border-4 border-secondary-100 border-t-primary-600" ] []
                    , p [ class "mt-4 text-secondary-500" ] [ text "読み込み中..." ]
                    ]

            Failure ->
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
            div [ class "flex gap-3" ]
                [ button
                    [ class "inline-flex items-center rounded-lg bg-success-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-success-700 disabled:opacity-50 disabled:cursor-not-allowed"
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
                    [ class "inline-flex items-center rounded-lg bg-error-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-error-700 disabled:opacity-50 disabled:cursor-not-allowed"
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
