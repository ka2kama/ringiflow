module Page.Workflow.Detail exposing (init, subscriptions, update, updateShared, view)

{-| 申請詳細ページ

ワークフローインスタンスの詳細情報を表示する。
型定義は Detail.Types に、各機能は Detail/ サブモジュールに分離している。

このモジュールはオーケストレーターとして、初期化・状態遷移・
メッセージルーティング・ページレイアウトを担当する。

-}

import Api exposing (ApiError)
import Api.Document as DocumentApi
import Api.ErrorMessage as ErrorMessage
import Api.Workflow as WorkflowApi
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Component.Badge as Badge
import Component.ErrorState as ErrorState
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.Document exposing (Document)
import Data.FormField exposing (FieldType(..), FormField)
import Data.WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance as WorkflowInstance exposing (WorkflowInstance, WorkflowStep)
import Form.DynamicForm as DynamicForm
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events
import Json.Decode as Decode
import Page.Workflow.Detail.Approval as Approval
import Page.Workflow.Detail.Comments as Comments
import Page.Workflow.Detail.Resubmit as Resubmit
import Page.Workflow.Detail.StepProgress as StepProgress
import Page.Workflow.Detail.Types as Types exposing (EditState(..), LoadedState, Model, Msg(..), initLoaded)
import Ports
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)
import Time
import Util.DateFormat as DateFormat


{-| 初期化
-}
init : Shared -> Int -> ( Model, Cmd Msg )
init shared workflowDisplayNumber =
    ( { shared = shared
      , workflowDisplayNumber = workflowDisplayNumber
      , state = Types.Loading
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
            ( { model | state = Types.Loading }
            , WorkflowApi.getWorkflow
                { config = Shared.toRequestConfig model.shared
                , displayNumber = model.workflowDisplayNumber
                , toMsg = GotWorkflow
                }
            )

        _ ->
            case model.state of
                Types.Loaded loaded ->
                    let
                        ( newLoaded, cmd ) =
                            updateLoaded msg model.shared model.workflowDisplayNumber loaded
                    in
                    ( { model | state = Types.Loaded newLoaded }, cmd )

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
                Types.Loaded loaded ->
                    ( { model | state = Types.Loaded { loaded | workflow = workflow } }
                    , Cmd.none
                    )

                _ ->
                    ( { model | state = Types.Loaded (initLoaded workflow) }
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
                        , DocumentApi.listWorkflowAttachments
                            { config = Shared.toRequestConfig model.shared
                            , workflowInstanceId = workflow.id
                            , toMsg = GotAttachments
                            }
                        ]
                    )

        Err err ->
            ( { model | state = Types.Failed err }
            , Cmd.none
            )


{-| Loaded 状態専用の状態更新

メッセージをドメインごとにサブモジュールへルーティングする。

-}
updateLoaded : Msg -> Shared -> Int -> LoadedState -> ( LoadedState, Cmd Msg )
updateLoaded msg shared workflowDisplayNumber loaded =
    case msg of
        -- ページ固有
        GotDefinition result ->
            case result of
                Ok definition ->
                    ( { loaded | definition = Success definition }, Cmd.none )

                Err err ->
                    ( { loaded | definition = Failure err }, Cmd.none )

        -- 添付ファイル
        GotAttachments result ->
            case result of
                Ok attachments ->
                    ( { loaded | attachments = Success attachments }, Cmd.none )

                Err err ->
                    ( { loaded | attachments = Failure err }, Cmd.none )

        DownloadFile documentId ->
            ( loaded
            , DocumentApi.requestDownloadUrl
                { config = Shared.toRequestConfig shared
                , documentId = documentId
                , toMsg = GotDownloadUrl
                }
            )

        GotDownloadUrl result ->
            case result of
                Ok response ->
                    ( loaded
                    , Ports.openUrl response.downloadUrl
                    )

                Err _ ->
                    ( { loaded | errorMessage = Just "ダウンロード URL の取得に失敗しました" }
                    , Cmd.none
                    )

        DismissMessage ->
            ( { loaded | errorMessage = Nothing, successMessage = Nothing }
            , Cmd.none
            )

        -- 承認ドメイン
        UpdateComment _ ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        ClickApprove _ ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        ClickReject _ ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        ClickRequestChanges _ ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        ConfirmAction ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        CancelAction ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        GotApproveResult _ ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        GotRejectResult _ ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        GotRequestChangesResult _ ->
            Approval.updateApproval msg shared workflowDisplayNumber loaded

        -- コメントドメイン
        GotComments _ ->
            Comments.updateComments msg shared workflowDisplayNumber loaded

        UpdateNewComment _ ->
            Comments.updateComments msg shared workflowDisplayNumber loaded

        SubmitComment ->
            Comments.updateComments msg shared workflowDisplayNumber loaded

        GotPostCommentResult _ ->
            Comments.updateComments msg shared workflowDisplayNumber loaded

        -- 再提出ドメイン
        StartEditing ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        CancelEditing ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        UpdateEditFormField _ _ ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        EditApproverSearchChanged _ _ ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        EditApproverSelected _ _ ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        EditApproverCleared _ ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        EditApproverKeyDown _ _ ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        EditApproverDropdownClosed _ ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        SubmitResubmit ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        GotResubmitResult _ ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        GotUsers _ ->
            Resubmit.updateResubmit msg shared workflowDisplayNumber loaded

        -- 上位で処理済み
        GotWorkflow _ ->
            ( loaded, Cmd.none )

        Refresh ->
            ( loaded, Cmd.none )



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
        Types.Loading ->
            LoadingSpinner.view

        Types.Failed err ->
            viewError err

        Types.Loaded loaded ->
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
        , Approval.viewConfirmDialog loaded.pendingAction
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
        , StepProgress.viewStepProgress loaded.workflow
        , Approval.viewApprovalSection loaded.workflow loaded.comment loaded.isSubmitting shared
        , Resubmit.viewResubmitSection shared loaded
        , viewSteps loaded.workflow
        , viewBasicInfo (Shared.zone shared) loaded.workflow
        , case loaded.editState of
            Editing editing ->
                Resubmit.viewEditableFormData loaded editing

            Viewing ->
                viewFormData loaded.workflow loaded.definition
        , viewAttachments loaded.attachments
        , Comments.viewCommentSection loaded
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


{-| フォームデータ表示（読み取り専用）
-}
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


{-| 添付ファイルセクション
-}
viewAttachments : RemoteData ApiError (List Document) -> Html Msg
viewAttachments remoteAttachments =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]
        [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "添付ファイル" ]
        , case remoteAttachments of
            NotAsked ->
                text ""

            RemoteData.Loading ->
                LoadingSpinner.view

            Failure _ ->
                p [ class "text-sm text-secondary-500" ] [ text "添付ファイルの取得に失敗しました" ]

            Success attachments ->
                if List.isEmpty attachments then
                    p [ class "text-sm text-secondary-500" ] [ text "添付ファイルはありません" ]

                else
                    ul [ class "space-y-2 list-none pl-0" ]
                        (List.map viewAttachmentItem attachments)
        ]


{-| 添付ファイル個別表示
-}
viewAttachmentItem : Document -> Html Msg
viewAttachmentItem doc =
    li [ class "flex items-center justify-between rounded-lg border border-secondary-200 bg-secondary-50 p-3" ]
        [ div [ class "min-w-0 flex-1" ]
            [ span [ class "truncate text-sm font-medium text-secondary-900" ]
                [ text doc.filename ]
            , span [ class "ml-2 text-xs text-secondary-500" ]
                [ text (formatFileSize doc.size) ]
            ]
        , button
            [ Html.Events.onClick (DownloadFile doc.id)
            , class "shrink-0 rounded border border-primary-500 bg-white px-3 py-1 text-sm text-primary-600 hover:bg-primary-50 transition-colors cursor-pointer"
            , type_ "button"
            ]
            [ text "ダウンロード" ]
        ]


{-| ファイルサイズを読みやすい形式にフォーマット
-}
formatFileSize : Int -> String
formatFileSize bytes =
    if bytes >= 1048576 then
        String.fromFloat (toFloat (bytes * 10 // 1048576) / 10) ++ " MB"

    else if bytes >= 1024 then
        String.fromFloat (toFloat (bytes * 10 // 1024) / 10) ++ " KB"

    else
        String.fromInt bytes ++ " B"
