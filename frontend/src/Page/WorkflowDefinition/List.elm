module Page.WorkflowDefinition.List exposing (Model, Msg, init, update, updateShared, view)

{-| ワークフロー定義一覧画面

テナント管理者がワークフロー定義を管理する画面。
一覧表示・作成・公開・アーカイブ・削除に対応。

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Component.Badge as Badge
import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Component.EmptyState as EmptyState
import Component.ErrorState as ErrorState
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.WorkflowDefinition as WorkflowDefinition
    exposing
        ( WorkflowDefinition
        , WorkflowDefinitionStatus(..)
        )
import Dict exposing (Dict)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput, onSubmit)
import Json.Decode
import Ports
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , definitions : RemoteData ApiError (List WorkflowDefinition)
    , statusFilter : Maybe WorkflowDefinitionStatus
    , pendingAction : Maybe PendingAction
    , isProcessing : Bool
    , successMessage : Maybe String
    , errorMessage : Maybe String
    , showCreateDialog : Bool
    , createName : String
    , createDescription : String
    , createValidationErrors : Dict String String
    }


{-| 確認ダイアログで保留中の操作
-}
type PendingAction
    = ConfirmPublish WorkflowDefinition
    | ConfirmArchive WorkflowDefinition
    | ConfirmDelete WorkflowDefinition


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , definitions = Loading
      , statusFilter = Nothing
      , pendingAction = Nothing
      , isProcessing = False
      , successMessage = Nothing
      , errorMessage = Nothing
      , showCreateDialog = False
      , createName = ""
      , createDescription = ""
      , createValidationErrors = Dict.empty
      }
    , WorkflowDefinitionApi.listDefinitions
        { config = Shared.toRequestConfig shared
        , toMsg = GotDefinitions
        }
    )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = GotDefinitions (Result ApiError (List WorkflowDefinition))
    | ChangeStatusFilter String
    | Refresh
      -- 作成ダイアログ
    | OpenCreateDialog
    | CloseCreateDialog
    | InputCreateName String
    | InputCreateDescription String
    | SubmitCreate
    | GotCreateResult (Result ApiError WorkflowDefinition)
      -- ステータス操作
    | ClickPublish WorkflowDefinition
    | ClickArchive WorkflowDefinition
    | ClickDelete WorkflowDefinition
    | ConfirmAction
    | CancelAction
    | GotPublishResult (Result ApiError WorkflowDefinition)
    | GotArchiveResult (Result ApiError WorkflowDefinition)
    | GotDeleteResult (Result ApiError ())
      -- メッセージ
    | DismissMessage


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotDefinitions result ->
            case result of
                Ok defs ->
                    ( { model | definitions = Success defs }, Cmd.none )

                Err err ->
                    ( { model | definitions = Failure err }, Cmd.none )

        ChangeStatusFilter value ->
            let
                newFilter =
                    if value == "" then
                        Nothing

                    else
                        Just (WorkflowDefinition.statusFromString value)
            in
            ( { model | statusFilter = newFilter }, Cmd.none )

        Refresh ->
            ( { model | definitions = Loading }
            , WorkflowDefinitionApi.listDefinitions
                { config = Shared.toRequestConfig model.shared
                , toMsg = GotDefinitions
                }
            )

        -- 作成ダイアログ
        OpenCreateDialog ->
            ( { model
                | showCreateDialog = True
                , createName = ""
                , createDescription = ""
                , createValidationErrors = Dict.empty
              }
            , Ports.showModalDialog createDialogId
            )

        CloseCreateDialog ->
            ( { model | showCreateDialog = False }, Cmd.none )

        InputCreateName name ->
            ( { model | createName = name }, Cmd.none )

        InputCreateDescription description ->
            ( { model | createDescription = description }, Cmd.none )

        SubmitCreate ->
            let
                errors =
                    validateCreateForm model.createName
            in
            if Dict.isEmpty errors then
                ( { model | isProcessing = True, createValidationErrors = Dict.empty }
                , WorkflowDefinitionApi.createDefinition
                    { config = Shared.toRequestConfig model.shared
                    , body =
                        WorkflowDefinition.encodeCreateRequest
                            { name = String.trim model.createName
                            , description = String.trim model.createDescription
                            }
                    , toMsg = GotCreateResult
                    }
                )

            else
                ( { model | createValidationErrors = errors }, Cmd.none )

        GotCreateResult result ->
            case result of
                Ok _ ->
                    ( { model
                        | isProcessing = False
                        , showCreateDialog = False
                        , successMessage = Just "ワークフロー定義を作成しました。"
                        , errorMessage = Nothing
                        , definitions = Loading
                      }
                    , WorkflowDefinitionApi.listDefinitions
                        { config = Shared.toRequestConfig model.shared
                        , toMsg = GotDefinitions
                        }
                    )

                Err err ->
                    ( { model
                        | isProcessing = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err)
                      }
                    , Cmd.none
                    )

        -- ステータス操作
        ClickPublish def ->
            ( { model | pendingAction = Just (ConfirmPublish def) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ClickArchive def ->
            ( { model | pendingAction = Just (ConfirmArchive def) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ClickDelete def ->
            ( { model | pendingAction = Just (ConfirmDelete def) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ConfirmAction ->
            case model.pendingAction of
                Just (ConfirmPublish def) ->
                    ( { model | pendingAction = Nothing, isProcessing = True }
                    , WorkflowDefinitionApi.publishDefinition
                        { config = Shared.toRequestConfig model.shared
                        , id = def.id
                        , body = WorkflowDefinition.encodeVersionRequest { version = def.version }
                        , toMsg = GotPublishResult
                        }
                    )

                Just (ConfirmArchive def) ->
                    ( { model | pendingAction = Nothing, isProcessing = True }
                    , WorkflowDefinitionApi.archiveDefinition
                        { config = Shared.toRequestConfig model.shared
                        , id = def.id
                        , body = WorkflowDefinition.encodeVersionRequest { version = def.version }
                        , toMsg = GotArchiveResult
                        }
                    )

                Just (ConfirmDelete def) ->
                    ( { model | pendingAction = Nothing, isProcessing = True }
                    , WorkflowDefinitionApi.deleteDefinition
                        { config = Shared.toRequestConfig model.shared
                        , id = def.id
                        , toMsg = GotDeleteResult
                        }
                    )

                Nothing ->
                    ( model, Cmd.none )

        CancelAction ->
            ( { model | pendingAction = Nothing }, Cmd.none )

        GotPublishResult result ->
            handleOperationResult "公開" result model

        GotArchiveResult result ->
            handleOperationResult "アーカイブ" result model

        GotDeleteResult result ->
            case result of
                Ok () ->
                    ( { model
                        | isProcessing = False
                        , successMessage = Just "ワークフロー定義を削除しました。"
                        , errorMessage = Nothing
                        , definitions = Loading
                      }
                    , WorkflowDefinitionApi.listDefinitions
                        { config = Shared.toRequestConfig model.shared
                        , toMsg = GotDefinitions
                        }
                    )

                Err err ->
                    ( { model
                        | isProcessing = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err)
                      }
                    , Cmd.none
                    )

        DismissMessage ->
            ( { model | successMessage = Nothing, errorMessage = Nothing }, Cmd.none )


{-| 公開・アーカイブ操作の結果を処理する共通ヘルパー
-}
handleOperationResult : String -> Result ApiError WorkflowDefinition -> Model -> ( Model, Cmd Msg )
handleOperationResult operationName result model =
    case result of
        Ok _ ->
            ( { model
                | isProcessing = False
                , successMessage = Just ("ワークフロー定義を" ++ operationName ++ "しました。")
                , errorMessage = Nothing
                , definitions = Loading
              }
            , WorkflowDefinitionApi.listDefinitions
                { config = Shared.toRequestConfig model.shared
                , toMsg = GotDefinitions
                }
            )

        Err err ->
            ( { model
                | isProcessing = False
                , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err)
              }
            , Cmd.none
            )


{-| 作成フォームのバリデーション
-}
validateCreateForm : String -> Dict String String
validateCreateForm name =
    if String.isEmpty (String.trim name) then
        Dict.singleton "name" "名前を入力してください。"

    else
        Dict.empty



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewHeader
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = model.successMessage
            , errorMessage = model.errorMessage
            }
        , viewFilters model.statusFilter
        , viewContent model
        , viewConfirmDialog model.pendingAction
        , if model.showCreateDialog then
            viewCreateDialog model

          else
            text ""
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "flex items-center justify-between mb-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ]
            [ text "ワークフロー定義" ]
        , Button.view
            { variant = Button.Primary
            , disabled = False
            , onClick = OpenCreateDialog
            }
            [ text "新規作成" ]
        ]


{-| ステータスフィルタ（クライアントサイドフィルタリング）
-}
viewFilters : Maybe WorkflowDefinitionStatus -> Html Msg
viewFilters statusFilter =
    div [ class "mb-4" ]
        [ select
            [ class "rounded-lg border border-secondary-300 px-3 py-2 text-sm"
            , onInput ChangeStatusFilter
            , value (statusFilterToValue statusFilter)
            ]
            [ option [ value "" ] [ text "すべてのステータス" ]
            , option [ value "Draft" ] [ text "下書き" ]
            , option [ value "Published" ] [ text "公開済み" ]
            , option [ value "Archived" ] [ text "アーカイブ済み" ]
            ]
        ]


statusFilterToValue : Maybe WorkflowDefinitionStatus -> String
statusFilterToValue maybeStatus =
    case maybeStatus of
        Just Draft ->
            "Draft"

        Just Published ->
            "Published"

        Just Archived ->
            "Archived"

        Nothing ->
            ""


viewContent : Model -> Html Msg
viewContent model =
    case model.definitions of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure err ->
            ErrorState.view
                { message = ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err
                , onRefresh = Refresh
                }

        Success defs ->
            let
                filtered =
                    case model.statusFilter of
                        Just status ->
                            List.filter (\d -> WorkflowDefinition.definitionStatus d == status) defs

                        Nothing ->
                            defs
            in
            viewDefinitionList filtered


viewDefinitionList : List WorkflowDefinition -> Html Msg
viewDefinitionList definitions =
    if List.isEmpty definitions then
        EmptyState.view
            { message = "ワークフロー定義がありません。"
            , description = Just "「新規作成」ボタンから定義を作成してください。"
            }

    else
        div []
            [ viewDefinitionTable definitions
            , div [ class "mt-2 text-sm text-secondary-500" ]
                [ text (String.fromInt (List.length definitions) ++ " 件の定義") ]
            ]


viewDefinitionTable : List WorkflowDefinition -> Html Msg
viewDefinitionTable definitions =
    div [ class "overflow-x-auto rounded-lg border border-secondary-200" ]
        [ table [ class "w-full" ]
            [ thead [ class "bg-secondary-50" ]
                [ tr []
                    [ th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "名前" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "説明" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "ステータス" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "更新日時" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "操作" ]
                    ]
                ]
            , tbody [ class "divide-y divide-secondary-200 bg-white" ]
                (List.map viewDefinitionRow definitions)
            ]
        ]


viewDefinitionRow : WorkflowDefinition -> Html Msg
viewDefinitionRow def =
    let
        status =
            WorkflowDefinition.definitionStatus def
    in
    tr [ class "hover:bg-secondary-50 transition-colors" ]
        [ td [ class "px-4 py-3 text-sm font-medium text-secondary-900" ]
            [ text def.name ]
        , td [ class "px-4 py-3 text-sm text-secondary-500" ]
            [ text (Maybe.withDefault "—" def.description) ]
        , td [ class "px-4 py-3 text-sm" ]
            [ Badge.view (WorkflowDefinition.statusToBadge status) ]
        , td [ class "px-4 py-3 text-sm text-secondary-500" ]
            [ text (formatDateTime def.updatedAt) ]
        , td [ class "px-4 py-3 text-sm" ]
            [ viewActions status def ]
        ]


{-| ステータスに応じた操作ボタン
-}
viewActions : WorkflowDefinitionStatus -> WorkflowDefinition -> Html Msg
viewActions status def =
    div [ class "flex gap-2" ]
        (case status of
            Draft ->
                [ a
                    [ href (Route.toString (Route.WorkflowDefinitionDesignerEdit def.id))
                    , class "inline-flex items-center rounded-lg px-3 py-1.5 text-sm font-medium text-primary-600 ring-1 ring-inset ring-primary-200 hover:bg-primary-50 transition-colors"
                    ]
                    [ text "編集" ]
                , Button.view
                    { variant = Button.Success
                    , disabled = False
                    , onClick = ClickPublish def
                    }
                    [ text "公開" ]
                , Button.view
                    { variant = Button.Error
                    , disabled = False
                    , onClick = ClickDelete def
                    }
                    [ text "削除" ]
                ]

            Published ->
                [ Button.view
                    { variant = Button.Warning
                    , disabled = False
                    , onClick = ClickArchive def
                    }
                    [ text "アーカイブ" ]
                ]

            Archived ->
                [ span [ class "text-secondary-400" ] [ text "—" ] ]
        )


{-| 確認ダイアログ
-}
viewConfirmDialog : Maybe PendingAction -> Html Msg
viewConfirmDialog maybePending =
    case maybePending of
        Just (ConfirmPublish def) ->
            ConfirmDialog.view
                { title = "ワークフロー定義を公開"
                , message = "「" ++ def.name ++ "」を公開しますか？公開後はユーザーが申請に使用できるようになります。"
                , confirmLabel = "公開する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmAction
                , onCancel = CancelAction
                , actionStyle = ConfirmDialog.Positive
                }

        Just (ConfirmArchive def) ->
            ConfirmDialog.view
                { title = "ワークフロー定義をアーカイブ"
                , message = "「" ++ def.name ++ "」をアーカイブしますか？アーカイブ後は新規申請に使用できなくなります。"
                , confirmLabel = "アーカイブする"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmAction
                , onCancel = CancelAction
                , actionStyle = ConfirmDialog.Caution
                }

        Just (ConfirmDelete def) ->
            ConfirmDialog.view
                { title = "ワークフロー定義を削除"
                , message = "「" ++ def.name ++ "」を削除しますか？この操作は取り消せません。"
                , confirmLabel = "削除する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmAction
                , onCancel = CancelAction
                , actionStyle = ConfirmDialog.Destructive
                }

        Nothing ->
            text ""


{-| 作成ダイアログ（`<dialog>` 要素）
-}
viewCreateDialog : Model -> Html Msg
viewCreateDialog model =
    Html.node "dialog"
        [ id createDialogId
        , class "fixed inset-0 m-0 h-full w-full max-h-none max-w-none bg-transparent p-0 border-none outline-none"
        , Html.Events.preventDefaultOn "cancel"
            (Json.Decode.succeed ( CloseCreateDialog, True ))
        , Html.Events.on "click"
            (ConfirmDialog.backdropClickDecoder CloseCreateDialog)
        ]
        [ div [ class "flex h-full w-full items-center justify-center pointer-events-none" ]
            [ div [ class "dialog-content pointer-events-auto w-full max-w-md rounded-lg bg-white p-6 shadow-xl" ]
                [ h2 [ class "text-lg font-semibold text-secondary-900" ] [ text "新しいワークフロー定義を作成" ]
                , Html.form [ onSubmit SubmitCreate, class "mt-4 space-y-4" ]
                    [ viewFormField "名前"
                        "create-name"
                        model.createName
                        InputCreateName
                        (Dict.get "name" model.createValidationErrors)
                        True
                    , viewFormTextarea "説明"
                        "create-description"
                        model.createDescription
                        InputCreateDescription
                    , div [ class "mt-6 flex justify-end gap-3" ]
                        [ Button.view
                            { variant = Button.Outline
                            , disabled = model.isProcessing
                            , onClick = CloseCreateDialog
                            }
                            [ text "キャンセル" ]
                        , button
                            [ type_ "submit"
                            , class "inline-flex items-center rounded-lg px-4 py-2 text-sm font-medium bg-primary-500 hover:bg-primary-600 text-white transition-colors disabled:opacity-50"
                            , disabled model.isProcessing
                            ]
                            [ if model.isProcessing then
                                text "作成中..."

                              else
                                text "作成"
                            ]
                        ]
                    ]
                ]
            ]
        ]


{-| テキスト入力フィールド
-}
viewFormField : String -> String -> String -> (String -> Msg) -> Maybe String -> Bool -> Html Msg
viewFormField labelText fieldId fieldValue onInputMsg maybeError isRequired =
    div []
        [ label [ for fieldId, class "block text-sm font-medium text-secondary-700" ] [ text labelText ]
        , input
            [ id fieldId
            , type_ "text"
            , class "mt-1 block w-full rounded-lg border border-secondary-300 px-3 py-2 text-sm shadow-sm focus:border-primary-500 focus:ring-1 focus:ring-primary-500"
            , value fieldValue
            , onInput onInputMsg
            , required isRequired
            , autofocus True
            ]
            []
        , case maybeError of
            Just error ->
                p [ class "mt-1 text-sm text-error-600" ] [ text error ]

            Nothing ->
                text ""
        ]


{-| テキストエリアフィールド
-}
viewFormTextarea : String -> String -> String -> (String -> Msg) -> Html Msg
viewFormTextarea labelText fieldId fieldValue onInputMsg =
    div []
        [ label [ for fieldId, class "block text-sm font-medium text-secondary-700" ] [ text labelText ]
        , textarea
            [ id fieldId
            , class "mt-1 block w-full rounded-lg border border-secondary-300 px-3 py-2 text-sm shadow-sm focus:border-primary-500 focus:ring-1 focus:ring-primary-500"
            , value fieldValue
            , onInput onInputMsg
            , rows 3
            ]
            []
        ]


{-| 日時文字列のフォーマット（簡易表示）

ISO 8601 の "T" を空白に置換し、秒以下を除去する。

-}
formatDateTime : String -> String
formatDateTime isoString =
    isoString
        |> String.replace "T" " "
        |> String.left 16


{-| 作成ダイアログの HTML id
-}
createDialogId : String
createDialogId =
    "create-definition-dialog"
