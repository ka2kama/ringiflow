module Page.Workflow.New exposing
    ( Model
    , Msg(..)
    , SaveMessage(..)
    , init
    , isDirty
    , update
    , updateShared
    , view
    )

{-| 新規申請フォームページ

ワークフロー定義を選択し、フォームを入力して申請するページ。


## 画面フロー

1.  ワークフロー定義一覧を取得・表示
2.  ユーザーが定義を選択
3.  動的フォームを生成・表示
4.  フォーム入力 → バリデーション
5.  下書き保存 or 申請


## 設計

詳細: [申請フォーム UI 設計](../../../../docs/03_詳細設計書/10_ワークフロー申請フォームUI設計.md)

-}

import Api exposing (ApiError)
import Api.User as UserApi
import Api.Workflow as WorkflowApi
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Component.ApproverSelector as ApproverSelector exposing (ApproverSelection(..))
import Component.Button as Button
import Component.LoadingSpinner as LoadingSpinner
import Data.UserItem as UserItem exposing (UserItem)
import Data.WorkflowDefinition as WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance exposing (WorkflowInstance)
import Dict exposing (Dict)
import Form.DynamicForm as DynamicForm
import Form.Validation as Validation
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events
import Json.Encode as Encode
import List.Extra
import Ports
import RemoteData exposing (RemoteData(..))
import Shared exposing (Shared)



-- MODEL


{-| ページの状態
-}
type alias Model =
    { -- 共有状態（API 呼び出しに必要）
      shared : Shared

    -- API データ
    , definitions : RemoteData ApiError (List WorkflowDefinition)
    , selectedDefinitionId : Maybe String
    , users : RemoteData ApiError (List UserItem)

    -- フォーム状態
    , title : String
    , formValues : Dict String String
    , validationErrors : Dict String String

    -- 承認者選択（キー: ステップ ID）
    , approvers : Dict String ApproverSelector.State

    -- 保存状態
    , savedWorkflow : Maybe WorkflowInstance
    , saveMessage : Maybe SaveMessage

    -- 操作状態
    , submitting : Bool

    -- dirty 状態（未保存の変更があるか）
    , isDirty_ : Bool
    }


{-| 保存結果メッセージ
-}
type SaveMessage
    = SaveSuccess String
    | SaveError String


{-| 初期化

ページ表示時にワークフロー定義一覧を取得する。

-}
init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , definitions = Loading
      , selectedDefinitionId = Nothing
      , users = Loading
      , title = ""
      , formValues = Dict.empty
      , validationErrors = Dict.empty
      , approvers = Dict.empty
      , savedWorkflow = Nothing
      , saveMessage = Nothing
      , submitting = False
      , isDirty_ = False
      }
    , Cmd.batch
        [ fetchDefinitions shared
        , fetchUsers shared
        ]
    )


{-| ワークフロー定義一覧を取得
-}
fetchDefinitions : Shared -> Cmd Msg
fetchDefinitions shared =
    WorkflowDefinitionApi.listDefinitions
        { config = Shared.toRequestConfig shared
        , toMsg = GotDefinitions
        }


{-| テナント内ユーザー一覧を取得
-}
fetchUsers : Shared -> Cmd Msg
fetchUsers shared =
    UserApi.listUsers
        { config = Shared.toRequestConfig shared
        , toMsg = GotUsers
        }


{-| 共有状態を更新

Main.elm から新しい共有状態（CSRF トークン取得後など）を受け取る。

-}
updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }


{-| フォームに未保存の変更があるかを返す
-}
isDirty : Model -> Bool
isDirty model =
    model.isDirty_


{-| フォーム入力時の dirty 状態更新

isDirty が False → True に変わるときのみ beforeunload を有効にする。
既に dirty な場合は余分な Port 通信を避ける。

-}
markDirty : Model -> ( Model, Cmd Msg )
markDirty model =
    if model.isDirty_ then
        ( model, Cmd.none )

    else
        ( { model | isDirty_ = True }
        , Ports.setBeforeUnloadEnabled True
        )


{-| 保存/送信成功時の dirty リセット

isDirty が True → False に変わるときのみ beforeunload を無効にする。

-}
clearDirty : Model -> ( Model, Cmd Msg )
clearDirty model =
    if model.isDirty_ then
        ( { model | isDirty_ = False }
        , Ports.setBeforeUnloadEnabled False
        )

    else
        ( model, Cmd.none )



-- UPDATE


{-| メッセージ
-}
type Msg
    = -- 初期化
      GotDefinitions (Result ApiError (List WorkflowDefinition))
    | GotUsers (Result ApiError (List UserItem))
      -- ワークフロー定義選択
    | SelectDefinition String
      -- フォーム入力
    | UpdateTitle String
    | UpdateField String String
      -- 承認者選択（第1引数: ステップ ID）
    | UpdateApproverSearch String String
    | SelectApprover String UserItem
    | ClearApprover String
    | ApproverKeyDown String String
    | CloseApproverDropdown String
      -- 保存・申請
    | SaveDraft
    | GotSaveResult (Result ApiError WorkflowInstance)
    | Submit
    | GotSaveAndSubmitResult (List WorkflowApi.StepApproverRequest) (Result ApiError WorkflowInstance)
    | GotSubmitResult (Result ApiError WorkflowInstance)
      -- メッセージクリア
    | ClearMessage


{-| 状態更新
-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotDefinitions result ->
            case result of
                Ok definitions ->
                    ( { model | definitions = Success definitions }
                    , Cmd.none
                    )

                Err error ->
                    ( { model | definitions = Failure error }
                    , Cmd.none
                    )

        GotUsers result ->
            case result of
                Ok users ->
                    ( { model | users = Success users }
                    , Cmd.none
                    )

                Err error ->
                    ( { model | users = Failure error }
                    , Cmd.none
                    )

        SelectDefinition definitionId ->
            let
                ( dirtyModel, dirtyCmd ) =
                    markDirty model

                approverStates =
                    case model.definitions of
                        Success definitions ->
                            case getSelectedDefinition (Just definitionId) definitions of
                                Just def ->
                                    WorkflowDefinition.approvalStepInfos def
                                        |> List.map (\info -> ( info.id, ApproverSelector.init ))
                                        |> Dict.fromList

                                Nothing ->
                                    Dict.empty

                        _ ->
                            Dict.empty
            in
            ( { dirtyModel
                | selectedDefinitionId = Just definitionId
                , formValues = Dict.empty
                , validationErrors = Dict.empty
                , approvers = approverStates
              }
            , dirtyCmd
            )

        UpdateTitle newTitle ->
            let
                ( dirtyModel, dirtyCmd ) =
                    markDirty model
            in
            ( { dirtyModel | title = newTitle }
            , dirtyCmd
            )

        UpdateField fieldId value ->
            let
                ( dirtyModel, dirtyCmd ) =
                    markDirty model
            in
            ( { dirtyModel | formValues = Dict.insert fieldId value model.formValues }
            , dirtyCmd
            )

        UpdateApproverSearch stepId query ->
            ( { model
                | approvers =
                    updateApproverState stepId
                        (\s ->
                            { s
                                | search = query
                                , dropdownOpen = not (String.isEmpty (String.trim query))
                                , highlightIndex = 0
                            }
                        )
                        model.approvers
              }
            , Cmd.none
            )

        SelectApprover stepId user ->
            let
                ( dirtyModel, dirtyCmd ) =
                    markDirty model
            in
            ( { dirtyModel
                | approvers =
                    updateApproverState stepId
                        (\s ->
                            { s
                                | selection = Selected user
                                , search = ""
                                , dropdownOpen = False
                                , highlightIndex = 0
                            }
                        )
                        dirtyModel.approvers
                , validationErrors = Dict.remove ("approver_" ++ stepId) dirtyModel.validationErrors
              }
            , dirtyCmd
            )

        ClearApprover stepId ->
            let
                ( dirtyModel, dirtyCmd ) =
                    markDirty model
            in
            ( { dirtyModel | approvers = Dict.insert stepId ApproverSelector.init dirtyModel.approvers }
            , dirtyCmd
            )

        ApproverKeyDown stepId key ->
            handleApproverKeyDown stepId key model

        CloseApproverDropdown stepId ->
            ( { model
                | approvers =
                    updateApproverState stepId
                        (\s -> { s | dropdownOpen = False })
                        model.approvers
              }
            , Cmd.none
            )

        SaveDraft ->
            -- 下書き保存時は最小限のバリデーション（タイトル + 定義選択）
            case ( model.selectedDefinitionId, Validation.validateTitle model.title ) of
                ( Nothing, _ ) ->
                    ( { model
                        | saveMessage = Just (SaveError "ワークフロー種類を選択してください")
                      }
                    , Cmd.none
                    )

                ( _, Err errorMsg ) ->
                    ( { model
                        | validationErrors = Dict.singleton "title" errorMsg
                        , saveMessage = Nothing
                      }
                    , Cmd.none
                    )

                ( Just definitionId, Ok _ ) ->
                    ( { model
                        | submitting = True
                        , saveMessage = Nothing
                        , validationErrors = Dict.empty
                      }
                    , saveDraft model.shared definitionId model.title model.formValues
                    )

        GotSaveResult result ->
            case result of
                Ok workflow ->
                    let
                        ( cleanModel, cleanCmd ) =
                            clearDirty model
                    in
                    ( { cleanModel
                        | submitting = False
                        , savedWorkflow = Just workflow
                        , saveMessage = Just (SaveSuccess "下書きを保存しました")
                      }
                    , cleanCmd
                    )

                Err _ ->
                    ( { model
                        | submitting = False
                        , saveMessage = Just (SaveError "保存に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        Submit ->
            -- 申請時は全項目 + 承認者バリデーション
            let
                validationErrors =
                    validateFormWithApprover model
            in
            if Dict.isEmpty validationErrors then
                let
                    approvers =
                        buildApprovers model
                in
                case model.savedWorkflow of
                    Just workflow ->
                        -- 既に下書き保存済みならそのまま申請
                        let
                            ( cleanModel, cleanCmd ) =
                                clearDirty model
                        in
                        ( { cleanModel
                            | submitting = True
                            , saveMessage = Nothing
                          }
                        , Cmd.batch
                            [ submitWorkflow cleanModel.shared workflow.displayNumber approvers
                            , cleanCmd
                            ]
                        )

                    Nothing ->
                        -- 未保存の場合、まず保存してから申請
                        case model.selectedDefinitionId of
                            Just definitionId ->
                                ( { model
                                    | submitting = True
                                    , saveMessage = Nothing
                                  }
                                , saveAndSubmit model.shared
                                    definitionId
                                    model.title
                                    model.formValues
                                    approvers
                                )

                            Nothing ->
                                ( { model
                                    | saveMessage = Just (SaveError "ワークフロー種類を選択してください")
                                  }
                                , Cmd.none
                                )

            else
                ( { model
                    | validationErrors = validationErrors
                    , saveMessage = Nothing
                  }
                , Cmd.none
                )

        GotSaveAndSubmitResult approvers result ->
            case result of
                Ok workflow ->
                    -- 保存成功 → 続けて申請（データは永続化済みなので dirty リセット）
                    let
                        ( cleanModel, cleanCmd ) =
                            clearDirty model
                    in
                    ( { cleanModel | savedWorkflow = Just workflow }
                    , Cmd.batch
                        [ submitWorkflow cleanModel.shared workflow.displayNumber approvers
                        , cleanCmd
                        ]
                    )

                Err _ ->
                    ( { model
                        | submitting = False
                        , saveMessage = Just (SaveError "保存に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        GotSubmitResult result ->
            case result of
                Ok workflow ->
                    let
                        ( cleanModel, cleanCmd ) =
                            clearDirty model
                    in
                    ( { cleanModel
                        | submitting = False
                        , savedWorkflow = Just workflow
                        , saveMessage = Just (SaveSuccess "申請が完了しました")
                      }
                    , cleanCmd
                    )

                Err _ ->
                    ( { model
                        | submitting = False
                        , saveMessage = Just (SaveError "申請に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        ClearMessage ->
            ( { model | saveMessage = Nothing }
            , Cmd.none
            )


{-| フォーム全体のバリデーション（下書き保存用）

タイトルと動的フォームフィールドを検証する。

-}
validateForm : Model -> Dict String String
validateForm model =
    let
        -- タイトルのバリデーション
        titleErrors =
            case Validation.validateTitle model.title of
                Err msg ->
                    Dict.singleton "title" msg

                Ok _ ->
                    Dict.empty

        -- 動的フィールドのバリデーション
        fieldErrors =
            case model.definitions of
                Success definitions ->
                    case getSelectedDefinition model.selectedDefinitionId definitions of
                        Just definition ->
                            case DynamicForm.extractFormFields definition.definition of
                                Ok fields ->
                                    Validation.validateAllFields fields model.formValues

                                Err _ ->
                                    Dict.empty

                        Nothing ->
                            Dict.empty

                _ ->
                    Dict.empty
    in
    Dict.union titleErrors fieldErrors


{-| フォーム全体 + 承認者のバリデーション（申請用）
-}
validateFormWithApprover : Model -> Dict String String
validateFormWithApprover model =
    let
        formErrors =
            validateForm model

        approverErrors =
            model.approvers
                |> Dict.toList
                |> List.filterMap
                    (\( stepId, state ) ->
                        if ApproverSelector.selectedUserId state.selection == Nothing then
                            Just ( "approver_" ++ stepId, "承認者を選択してください" )

                        else
                            Nothing
                    )
                |> Dict.fromList
    in
    Dict.union formErrors approverErrors


{-| 承認者検索のキーボードイベントを処理
-}
handleApproverKeyDown : String -> String -> Model -> ( Model, Cmd Msg )
handleApproverKeyDown stepId key model =
    case Dict.get stepId model.approvers of
        Just state ->
            let
                candidates =
                    case model.users of
                        Success users ->
                            UserItem.filterUsers state.search users

                        _ ->
                            []

                result =
                    ApproverSelector.handleKeyDown
                        { key = key
                        , candidates = candidates
                        , highlightIndex = state.highlightIndex
                        }
            in
            case result of
                ApproverSelector.NoChange ->
                    ( model, Cmd.none )

                ApproverSelector.Navigate newIndex ->
                    ( { model | approvers = updateApproverState stepId (\s -> { s | highlightIndex = newIndex }) model.approvers }
                    , Cmd.none
                    )

                ApproverSelector.Select user ->
                    let
                        ( dirtyModel, dirtyCmd ) =
                            markDirty model
                    in
                    ( { dirtyModel
                        | approvers =
                            updateApproverState stepId
                                (\s ->
                                    { s
                                        | selection = Selected user
                                        , search = ""
                                        , dropdownOpen = False
                                        , highlightIndex = 0
                                    }
                                )
                                dirtyModel.approvers
                        , validationErrors = Dict.remove ("approver_" ++ stepId) dirtyModel.validationErrors
                      }
                    , dirtyCmd
                    )

                ApproverSelector.Close ->
                    ( { model | approvers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) model.approvers }
                    , Cmd.none
                    )

        Nothing ->
            ( model, Cmd.none )


{-| ApproverSelector.State を更新するヘルパー
-}
updateApproverState : String -> (ApproverSelector.State -> ApproverSelector.State) -> Dict String ApproverSelector.State -> Dict String ApproverSelector.State
updateApproverState stepId updater dict =
    Dict.update stepId (Maybe.map updater) dict


{-| 選択されたワークフロー定義を取得
-}
getSelectedDefinition : Maybe String -> List WorkflowDefinition -> Maybe WorkflowDefinition
getSelectedDefinition maybeId definitions =
    maybeId
        |> Maybe.andThen (\defId -> List.Extra.find (\d -> d.id == defId) definitions)


{-| 各ステップの承認者選択から承認者リストを構築する
-}
buildApprovers : Model -> List WorkflowApi.StepApproverRequest
buildApprovers model =
    model.approvers
        |> Dict.toList
        |> List.filterMap
            (\( stepId, state ) ->
                ApproverSelector.selectedUserId state.selection
                    |> Maybe.map (\userId -> { stepId = stepId, assignedTo = userId })
            )


{-| 下書き保存 API を呼び出す
-}
saveDraft : Shared -> String -> String -> Dict String String -> Cmd Msg
saveDraft shared definitionId title formValues =
    WorkflowApi.createWorkflow
        { config = Shared.toRequestConfig shared
        , body =
            { definitionId = definitionId
            , title = title
            , formData = encodeFormValues formValues
            }
        , toMsg = GotSaveResult
        }


{-| フォーム値を JSON にエンコード
-}
encodeFormValues : Dict String String -> Encode.Value
encodeFormValues values =
    Dict.toList values
        |> List.map (\( k, v ) -> ( k, Encode.string v ))
        |> Encode.object


{-| ワークフローを申請
-}
submitWorkflow : Shared -> Int -> List WorkflowApi.StepApproverRequest -> Cmd Msg
submitWorkflow shared workflowDisplayNumber approvers =
    WorkflowApi.submitWorkflow
        { config = Shared.toRequestConfig shared
        , displayNumber = workflowDisplayNumber
        , body = { approvers = approvers }
        , toMsg = GotSubmitResult
        }


{-| 保存と申請を連続実行

未保存の場合、まず下書き保存し、成功したら申請を行う。
MVP では保存結果を GotSaveResult で受け取り、そこから申請を行うフローに。
ただし、この実装では簡略化のため保存→申請を一度に行う。

将来的には Task.andThen パターンで連結する方がエレガント。

-}
saveAndSubmit : Shared -> String -> String -> Dict String String -> List WorkflowApi.StepApproverRequest -> Cmd Msg
saveAndSubmit shared definitionId title formValues approvers =
    -- MVP では簡略化: 保存のみ行い、保存成功後にユーザーが再度申請ボタンを押す
    -- 理由: Elm で Cmd のチェーンは Task 変換が必要で複雑になるため
    -- TODO: 将来的には保存→申請の連続処理を実装
    WorkflowApi.createWorkflow
        { config = Shared.toRequestConfig shared
        , body =
            { definitionId = definitionId
            , title = title
            , formData = encodeFormValues formValues
            }
        , toMsg = GotSaveAndSubmitResult approvers
        }



-- VIEW


{-| ページの描画
-}
view : Model -> Html Msg
view model =
    div []
        [ h2 [ class "text-2xl font-bold text-secondary-900" ] [ text "新規申請" ]
        , viewSaveMessage model.saveMessage
        , viewContent model
        ]


{-| 保存メッセージバナー
-}
viewSaveMessage : Maybe SaveMessage -> Html Msg
viewSaveMessage maybeSaveMessage =
    case maybeSaveMessage of
        Just (SaveSuccess message) ->
            div
                [ class "flex items-center justify-between rounded-lg bg-success-50 p-4 text-success-700 mb-4" ]
                [ text message
                , button
                    [ Html.Events.onClick ClearMessage
                    , class "border-0 bg-transparent cursor-pointer text-xl text-success-700"
                    ]
                    [ text "×" ]
                ]

        Just (SaveError message) ->
            div
                [ class "flex items-center justify-between rounded-lg bg-error-50 p-4 text-error-700 mb-4" ]
                [ text message
                , button
                    [ Html.Events.onClick ClearMessage
                    , class "border-0 bg-transparent cursor-pointer text-xl text-error-700"
                    ]
                    [ text "×" ]
                ]

        Nothing ->
            text ""


{-| メインコンテンツ
-}
viewContent : Model -> Html Msg
viewContent model =
    case model.definitions of
        NotAsked ->
            viewLoading

        Loading ->
            viewLoading

        Failure _ ->
            viewError

        Success definitions ->
            viewForm model definitions


{-| ローディング表示
-}
viewLoading : Html Msg
viewLoading =
    LoadingSpinner.view


{-| エラー表示
-}
viewError : Html Msg
viewError =
    div [ class "rounded-lg bg-error-50 p-8 text-center text-error-700" ]
        [ text "データの取得に失敗しました。"
        , br [] []
        , text "ページを再読み込みしてください。"
        ]


{-| フォーム表示
-}
viewForm : Model -> List WorkflowDefinition -> Html Msg
viewForm model definitions =
    let
        -- 選択された定義を取得
        selectedDefinition =
            model.selectedDefinitionId
                |> Maybe.andThen (\defId -> List.Extra.find (\d -> d.id == defId) definitions)
    in
    div []
        [ -- Step 1: ワークフロー定義選択
          viewDefinitionSelector definitions model.selectedDefinitionId

        -- Step 2: フォーム入力（定義選択後に表示）
        , case selectedDefinition of
            Just definition ->
                viewFormInputs model definition

            Nothing ->
                text ""
        ]


{-| ワークフロー定義セレクター
-}
viewDefinitionSelector : List WorkflowDefinition -> Maybe String -> Html Msg
viewDefinitionSelector definitions selectedId =
    div
        [ class "mb-8" ]
        [ h3 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "Step 1: ワークフロー種類を選択" ]
        , div
            [ class "flex flex-col gap-2" ]
            (List.map (viewDefinitionOption selectedId) definitions)
        ]


{-| 定義選択肢
-}
viewDefinitionOption : Maybe String -> WorkflowDefinition -> Html Msg
viewDefinitionOption selectedId definition =
    let
        isSelected =
            selectedId == Just definition.id
    in
    label
        [ class
            ("flex items-center cursor-pointer rounded-lg border border-secondary-100 p-4"
                ++ (if isSelected then
                        " bg-primary-50"

                    else
                        " bg-white"
                   )
            )
        ]
        [ input
            [ type_ "radio"
            , name "workflow-definition"
            , Html.Attributes.value definition.id
            , checked isSelected
            , Html.Events.onClick (SelectDefinition definition.id)
            , class "mr-4"
            ]
            []
        , div []
            [ div [ class "font-medium" ] [ text definition.name ]
            , case definition.description of
                Just desc ->
                    div [ class "text-sm text-secondary-500" ]
                        [ text desc ]

                Nothing ->
                    text ""
            ]
        ]


{-| フォーム入力エリア
-}
viewFormInputs : Model -> WorkflowDefinition -> Html Msg
viewFormInputs model definition =
    div []
        [ h3 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "Step 2: フォーム入力" ]

        -- タイトル入力
        , div [ class "mb-6" ]
            [ label
                [ for "title"
                , class "block mb-2 font-medium"
                ]
                [ text "タイトル"
                , span [ class "text-error-600" ] [ text " *" ]
                ]
            , input
                [ type_ "text"
                , id "title"
                , Html.Attributes.value model.title
                , Html.Events.onInput UpdateTitle
                , placeholder "申請のタイトルを入力"
                , class "w-full rounded border border-secondary-300 bg-white px-3 py-3 text-base outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
                ]
                []
            , viewTitleError model
            ]

        -- 動的フォームフィールド
        , viewDynamicFormFields definition model

        -- Step 3: 承認者選択
        , viewApproverSection definition model

        -- アクションボタン
        , viewActions model
        ]


{-| 承認者選択セクション

各承認ステップごとに承認者を選択する UI を表示する。
ステップ情報は WorkflowDefinition から取得する。

-}
viewApproverSection : WorkflowDefinition -> Model -> Html Msg
viewApproverSection definition model =
    let
        stepInfos =
            WorkflowDefinition.approvalStepInfos definition
    in
    div []
        [ h3 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "Step 3: 承認者選択" ]
        , div [ class "flex flex-col gap-4" ]
            (List.map (viewApproverStep model) stepInfos)
        ]


{-| 承認ステップごとの承認者選択
-}
viewApproverStep : Model -> WorkflowDefinition.ApprovalStepInfo -> Html Msg
viewApproverStep model stepInfo =
    let
        state =
            Dict.get stepInfo.id model.approvers
                |> Maybe.withDefault ApproverSelector.init
    in
    div [ class "mb-2" ]
        [ label
            [ class "block mb-2 font-medium" ]
            [ text stepInfo.name
            , span [ class "text-error-600" ] [ text " *" ]
            ]
        , ApproverSelector.view
            { state = state
            , users = model.users
            , validationError = Dict.get ("approver_" ++ stepInfo.id) model.validationErrors
            , onSearch = UpdateApproverSearch stepInfo.id
            , onSelect = SelectApprover stepInfo.id
            , onClear = ClearApprover stepInfo.id
            , onKeyDown = ApproverKeyDown stepInfo.id
            , onCloseDropdown = CloseApproverDropdown stepInfo.id
            }
        ]


{-| タイトルのエラー表示
-}
viewTitleError : Model -> Html Msg
viewTitleError model =
    case Dict.get "title" model.validationErrors of
        Just errorMsg ->
            div
                [ class "mt-1 text-sm text-error-600" ]
                [ text errorMsg ]

        Nothing ->
            text ""


{-| 動的フォームフィールドを描画
-}
viewDynamicFormFields : WorkflowDefinition -> Model -> Html Msg
viewDynamicFormFields definition model =
    case DynamicForm.extractFormFields definition.definition of
        Ok fields ->
            if List.isEmpty fields then
                text ""

            else
                div
                    [ class "mb-6 rounded-lg bg-secondary-50 p-4" ]
                    [ h4
                        [ class "mb-4 text-secondary-900" ]
                        [ text (definition.name ++ " フォーム") ]
                    , DynamicForm.viewFields
                        fields
                        model.formValues
                        model.validationErrors
                        UpdateField
                    ]

        Err _ ->
            div
                [ class "rounded bg-error-50 p-4 text-error-600" ]
                [ text "フォーム定義の読み込みに失敗しました。" ]


{-| アクションボタン
-}
viewActions : Model -> Html Msg
viewActions model =
    div
        [ class "mt-8 flex justify-end gap-4 border-t border-secondary-100 pt-4" ]
        [ Button.view
            { variant = Button.Outline
            , disabled = model.submitting
            , onClick = SaveDraft
            }
            [ text "下書き保存" ]
        , Button.view
            { variant = Button.Primary
            , disabled = model.submitting
            , onClick = Submit
            }
            [ text "申請する" ]
        ]
