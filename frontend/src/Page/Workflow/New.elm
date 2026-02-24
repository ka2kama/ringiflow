module Page.Workflow.New exposing
    ( EditingState
    , FormState(..)
    , LoadedState
    , Model
    , Msg(..)
    , PageState(..)
    , SaveMessage
    , init
    , isDirty
    , update
    , updateShared
    , view
    )

{-| 新規申請フォームページ

ワークフロー定義を選択し、フォームを入力して申請するページ。

型安全ステートマシンパターンの標準化（[ADR-054](../../docs/05_ADR/054_型安全ステートマシンパターンの標準化.md)）に基づき構造化。
Loading/Failed/Loaded の PageState と、Loaded 内の SelectingDefinition/Editing の
FormState で不正な状態を型レベルで排除する。


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
import Api.ErrorMessage as ErrorMessage
import Api.User as UserApi
import Api.Workflow as WorkflowApi
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Component.ApproverSelector as ApproverSelector exposing (ApproverSelection(..))
import Component.Button as Button
import Component.ErrorState as ErrorState
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
    { shared : Shared
    , users : RemoteData ApiError (List UserItem)
    , state : PageState
    }


{-| ページの状態遷移

    Loading → Loaded（定義取得成功）
    Loading → Failed（定義取得失敗）

-}
type PageState
    = Loading
    | Failed ApiError
    | Loaded LoadedState


{-| 定義ロード完了後の状態
-}
type alias LoadedState =
    { definitions : List WorkflowDefinition
    , formState : FormState
    }


{-| フォームの状態遷移

    SelectingDefinition → Editing（定義選択）

-}
type FormState
    = SelectingDefinition
    | Editing EditingState


{-| フォーム編集中の状態

定義が選択済みであることが型で保証される。

-}
type alias EditingState =
    { selectedDefinition : WorkflowDefinition
    , title : String
    , formValues : Dict String String
    , validationErrors : Dict String String
    , approvers : Dict String ApproverSelector.State
    , savedWorkflow : Maybe WorkflowInstance
    , saveMessage : Maybe SaveMessage
    , submitting : Bool
    , isDirty_ : Bool
    }


{-| 保存結果メッセージ
-}
type SaveMessage
    = SaveSuccess String
    | SaveError String


{-| 初期化

ページ表示時にワークフロー定義一覧とユーザー一覧を並行取得する。

-}
init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , users = RemoteData.Loading
      , state = Loading
      }
    , Cmd.batch
        [ fetchDefinitions shared
        , fetchUsers shared
        ]
    )


{-| 編集状態の初期化

定義選択時に新しい EditingState を構築する。
承認ステップ情報から ApproverSelector の初期状態を生成する。

-}
initEditing : WorkflowDefinition -> EditingState
initEditing definition =
    { selectedDefinition = definition
    , title = ""
    , formValues = Dict.empty
    , validationErrors = Dict.empty
    , approvers =
        WorkflowDefinition.approvalStepInfos definition
            |> List.map (\info -> ( info.id, ApproverSelector.init ))
            |> Dict.fromList
    , savedWorkflow = Nothing
    , saveMessage = Nothing
    , submitting = False
    , isDirty_ = False
    }


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
    case model.state of
        Loaded loaded ->
            case loaded.formState of
                Editing editing ->
                    editing.isDirty_

                SelectingDefinition ->
                    False

        _ ->
            False


{-| フォーム入力時の dirty 状態更新

isDirty が False → True に変わるときのみ beforeunload を有効にする。
既に dirty な場合は余分な Port 通信を避ける。

-}
markDirty : EditingState -> ( EditingState, Cmd Msg )
markDirty editing =
    if editing.isDirty_ then
        ( editing, Cmd.none )

    else
        ( { editing | isDirty_ = True }
        , Ports.setBeforeUnloadEnabled True
        )


{-| 保存/送信成功時の dirty リセット

isDirty が True → False に変わるときのみ beforeunload を無効にする。

-}
clearDirty : EditingState -> ( EditingState, Cmd Msg )
clearDirty editing =
    if editing.isDirty_ then
        ( { editing | isDirty_ = False }
        , Ports.setBeforeUnloadEnabled False
        )

    else
        ( editing, Cmd.none )



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


{-| 状態更新（外側）

GotDefinitions で Loading → Loaded/Failed の状態遷移を処理。
GotUsers は state に依存せず users を更新。
それ以外は Loaded 状態のときのみ updateLoaded に委譲。

-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotDefinitions result ->
            case result of
                Ok definitions ->
                    ( { model
                        | state =
                            Loaded
                                { definitions = definitions
                                , formState = SelectingDefinition
                                }
                      }
                    , Cmd.none
                    )

                Err error ->
                    ( { model | state = Failed error }
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

        _ ->
            case model.state of
                Loaded loaded ->
                    let
                        ( newLoaded, cmd ) =
                            updateLoaded msg model.shared model.users loaded
                    in
                    ( { model | state = Loaded newLoaded }, cmd )

                _ ->
                    ( model, Cmd.none )


{-| Loaded 状態の更新

SelectDefinition で FormState を遷移。
それ以外は Editing 状態のときのみ updateEditing に委譲。

-}
updateLoaded : Msg -> Shared -> RemoteData ApiError (List UserItem) -> LoadedState -> ( LoadedState, Cmd Msg )
updateLoaded msg shared users loaded =
    case msg of
        SelectDefinition definitionId ->
            case List.Extra.find (\d -> d.id == definitionId) loaded.definitions of
                Just definition ->
                    let
                        previousIsDirty =
                            case loaded.formState of
                                Editing prev ->
                                    prev.isDirty_

                                SelectingDefinition ->
                                    False

                        newEditing =
                            initEditing definition

                        ( dirtyEditing, dirtyCmd ) =
                            markDirty { newEditing | isDirty_ = previousIsDirty }
                    in
                    ( { loaded | formState = Editing dirtyEditing }, dirtyCmd )

                Nothing ->
                    ( loaded, Cmd.none )

        _ ->
            case loaded.formState of
                Editing editing ->
                    let
                        ( newEditing, cmd ) =
                            updateEditing msg shared users editing
                    in
                    ( { loaded | formState = Editing newEditing }, cmd )

                SelectingDefinition ->
                    ( loaded, Cmd.none )


{-| Editing 状態の更新

フォーム入力、承認者選択、保存、申請の処理を行う。
定義が選択済みであることが型で保証されているため、
定義未選択チェックが不要。

-}
updateEditing : Msg -> Shared -> RemoteData ApiError (List UserItem) -> EditingState -> ( EditingState, Cmd Msg )
updateEditing msg shared users editing =
    case msg of
        UpdateTitle newTitle ->
            let
                ( dirtyEditing, dirtyCmd ) =
                    markDirty editing
            in
            ( { dirtyEditing | title = newTitle }
            , dirtyCmd
            )

        UpdateField fieldId value ->
            let
                ( dirtyEditing, dirtyCmd ) =
                    markDirty editing
            in
            ( { dirtyEditing | formValues = Dict.insert fieldId value editing.formValues }
            , dirtyCmd
            )

        UpdateApproverSearch stepId query ->
            ( { editing
                | approvers =
                    updateApproverState stepId
                        (\s ->
                            { s
                                | search = query
                                , dropdownOpen = not (String.isEmpty (String.trim query))
                                , highlightIndex = 0
                            }
                        )
                        editing.approvers
              }
            , Cmd.none
            )

        SelectApprover stepId user ->
            let
                ( dirtyEditing, dirtyCmd ) =
                    markDirty editing
            in
            ( { dirtyEditing
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
                        dirtyEditing.approvers
                , validationErrors = Dict.remove ("approver_" ++ stepId) dirtyEditing.validationErrors
              }
            , dirtyCmd
            )

        ClearApprover stepId ->
            let
                ( dirtyEditing, dirtyCmd ) =
                    markDirty editing
            in
            ( { dirtyEditing | approvers = Dict.insert stepId ApproverSelector.init dirtyEditing.approvers }
            , dirtyCmd
            )

        ApproverKeyDown stepId key ->
            handleApproverKeyDown stepId key users editing

        CloseApproverDropdown stepId ->
            ( { editing
                | approvers =
                    updateApproverState stepId
                        (\s -> { s | dropdownOpen = False })
                        editing.approvers
              }
            , Cmd.none
            )

        SaveDraft ->
            case Validation.validateTitle editing.title of
                Err errorMsg ->
                    ( { editing
                        | validationErrors = Dict.singleton "title" errorMsg
                        , saveMessage = Nothing
                      }
                    , Cmd.none
                    )

                Ok _ ->
                    ( { editing
                        | submitting = True
                        , saveMessage = Nothing
                        , validationErrors = Dict.empty
                      }
                    , saveDraft shared editing.selectedDefinition.id editing.title editing.formValues
                    )

        GotSaveResult result ->
            case result of
                Ok workflow ->
                    let
                        ( cleanEditing, cleanCmd ) =
                            clearDirty editing
                    in
                    ( { cleanEditing
                        | submitting = False
                        , savedWorkflow = Just workflow
                        , saveMessage = Just (SaveSuccess "下書きを保存しました")
                      }
                    , cleanCmd
                    )

                Err _ ->
                    ( { editing
                        | submitting = False
                        , saveMessage = Just (SaveError "保存に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        Submit ->
            let
                validationErrors =
                    validateFormWithApprover editing
            in
            if Dict.isEmpty validationErrors then
                let
                    approvers =
                        buildApprovers editing
                in
                case editing.savedWorkflow of
                    Just workflow ->
                        let
                            ( cleanEditing, cleanCmd ) =
                                clearDirty editing
                        in
                        ( { cleanEditing
                            | submitting = True
                            , saveMessage = Nothing
                          }
                        , Cmd.batch
                            [ submitWorkflow shared workflow.displayNumber approvers
                            , cleanCmd
                            ]
                        )

                    Nothing ->
                        ( { editing
                            | submitting = True
                            , saveMessage = Nothing
                          }
                        , saveAndSubmit shared
                            editing.selectedDefinition.id
                            editing.title
                            editing.formValues
                            approvers
                        )

            else
                ( { editing
                    | validationErrors = validationErrors
                    , saveMessage = Nothing
                  }
                , Cmd.none
                )

        GotSaveAndSubmitResult approvers result ->
            case result of
                Ok workflow ->
                    let
                        ( cleanEditing, cleanCmd ) =
                            clearDirty editing
                    in
                    ( { cleanEditing | savedWorkflow = Just workflow }
                    , Cmd.batch
                        [ submitWorkflow shared workflow.displayNumber approvers
                        , cleanCmd
                        ]
                    )

                Err _ ->
                    ( { editing
                        | submitting = False
                        , saveMessage = Just (SaveError "保存に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        GotSubmitResult result ->
            case result of
                Ok workflow ->
                    let
                        ( cleanEditing, cleanCmd ) =
                            clearDirty editing
                    in
                    ( { cleanEditing
                        | submitting = False
                        , savedWorkflow = Just workflow
                        , saveMessage = Just (SaveSuccess "申請が完了しました")
                      }
                    , cleanCmd
                    )

                Err _ ->
                    ( { editing
                        | submitting = False
                        , saveMessage = Just (SaveError "申請に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        ClearMessage ->
            ( { editing | saveMessage = Nothing }
            , Cmd.none
            )

        -- 外側レベルで処理済みのメッセージ（GotDefinitions, GotUsers, SelectDefinition）
        _ ->
            ( editing, Cmd.none )


{-| フォーム全体のバリデーション

タイトルと動的フォームフィールドを検証する。
selectedDefinition により定義検索が不要。

-}
validateForm : EditingState -> Dict String String
validateForm editing =
    let
        titleErrors =
            case Validation.validateTitle editing.title of
                Err msg ->
                    Dict.singleton "title" msg

                Ok _ ->
                    Dict.empty

        fieldErrors =
            case DynamicForm.extractFormFields editing.selectedDefinition.definition of
                Ok fields ->
                    Validation.validateAllFields fields editing.formValues

                Err _ ->
                    Dict.empty
    in
    Dict.union titleErrors fieldErrors


{-| フォーム全体 + 承認者のバリデーション（申請用）
-}
validateFormWithApprover : EditingState -> Dict String String
validateFormWithApprover editing =
    let
        formErrors =
            validateForm editing

        approverErrors =
            editing.approvers
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
handleApproverKeyDown : String -> String -> RemoteData ApiError (List UserItem) -> EditingState -> ( EditingState, Cmd Msg )
handleApproverKeyDown stepId key users editing =
    case Dict.get stepId editing.approvers of
        Just state ->
            let
                candidates =
                    case users of
                        Success userList ->
                            UserItem.filterUsers state.search userList

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
                    ( editing, Cmd.none )

                ApproverSelector.Navigate newIndex ->
                    ( { editing | approvers = updateApproverState stepId (\s -> { s | highlightIndex = newIndex }) editing.approvers }
                    , Cmd.none
                    )

                ApproverSelector.Select user ->
                    let
                        ( dirtyEditing, dirtyCmd ) =
                            markDirty editing
                    in
                    ( { dirtyEditing
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
                                dirtyEditing.approvers
                        , validationErrors = Dict.remove ("approver_" ++ stepId) dirtyEditing.validationErrors
                      }
                    , dirtyCmd
                    )

                ApproverSelector.Close ->
                    ( { editing | approvers = updateApproverState stepId (\s -> { s | dropdownOpen = False }) editing.approvers }
                    , Cmd.none
                    )

        Nothing ->
            ( editing, Cmd.none )


{-| ApproverSelector.State を更新するヘルパー
-}
updateApproverState : String -> (ApproverSelector.State -> ApproverSelector.State) -> Dict String ApproverSelector.State -> Dict String ApproverSelector.State
updateApproverState stepId updater dict =
    Dict.update stepId (Maybe.map updater) dict


{-| 各ステップの承認者選択から承認者リストを構築する

定義の承認ステップ順序に従って構築する。
selectedDefinition により直接ステップ情報にアクセスできるため、
Dict のキー順へのフォールバックが不要。

-}
buildApprovers : EditingState -> List WorkflowApi.StepApproverRequest
buildApprovers editing =
    let
        stepIds =
            WorkflowDefinition.approvalStepInfos editing.selectedDefinition
                |> List.map .id
    in
    stepIds
        |> List.filterMap
            (\stepId ->
                Dict.get stepId editing.approvers
                    |> Maybe.andThen (\state -> ApproverSelector.selectedUserId state.selection)
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
    -- TODO(#889): 将来的には保存→申請の連続処理を実装
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
        [ h1 [ class "mb-6 text-2xl font-bold text-secondary-900" ] [ text "新規申請" ]
        , viewBody model
        ]


{-| メインコンテンツ

PageState のパターンマッチで Loading/Failed/Loaded を分岐。
Failed では ErrorState.viewSimple + ErrorMessage.toUserMessage で
ApiError に応じた具体的なエラーメッセージを表示する。

-}
viewBody : Model -> Html Msg
viewBody model =
    case model.state of
        Loading ->
            LoadingSpinner.view

        Failed error ->
            ErrorState.viewSimple
                (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } error)

        Loaded loaded ->
            viewLoaded model.users loaded


{-| Loaded 状態の描画

FormState のパターンマッチで SelectingDefinition/Editing を分岐。

-}
viewLoaded : RemoteData ApiError (List UserItem) -> LoadedState -> Html Msg
viewLoaded users loaded =
    case loaded.formState of
        SelectingDefinition ->
            viewDefinitionSelector loaded.definitions Nothing

        Editing editing ->
            div []
                [ viewSaveMessage editing.saveMessage
                , viewDefinitionSelector loaded.definitions (Just editing.selectedDefinition.id)
                , viewFormInputs users editing
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
viewFormInputs : RemoteData ApiError (List UserItem) -> EditingState -> Html Msg
viewFormInputs users editing =
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
                , Html.Attributes.value editing.title
                , Html.Events.onInput UpdateTitle
                , placeholder "申請のタイトルを入力"
                , class "w-full rounded border border-secondary-300 bg-white px-3 py-3 text-base outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
                ]
                []
            , viewTitleError editing
            ]

        -- 動的フォームフィールド
        , viewDynamicFormFields editing

        -- Step 3: 承認者選択
        , viewApproverSection users editing

        -- アクションボタン
        , viewActions editing
        ]


{-| 承認者選択セクション

各承認ステップごとに承認者を選択する UI を表示する。
ステップ情報は selectedDefinition から直接取得する。

-}
viewApproverSection : RemoteData ApiError (List UserItem) -> EditingState -> Html Msg
viewApproverSection users editing =
    let
        stepInfos =
            WorkflowDefinition.approvalStepInfos editing.selectedDefinition
    in
    div []
        [ h3 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "Step 3: 承認者選択" ]
        , div [ class "flex flex-col gap-4" ]
            (List.map (viewApproverStep users editing) stepInfos)
        ]


{-| 承認ステップごとの承認者選択
-}
viewApproverStep : RemoteData ApiError (List UserItem) -> EditingState -> WorkflowDefinition.ApprovalStepInfo -> Html Msg
viewApproverStep users editing stepInfo =
    let
        state =
            Dict.get stepInfo.id editing.approvers
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
            , users = users
            , validationError = Dict.get ("approver_" ++ stepInfo.id) editing.validationErrors
            , onSearch = UpdateApproverSearch stepInfo.id
            , onSelect = SelectApprover stepInfo.id
            , onClear = ClearApprover stepInfo.id
            , onKeyDown = ApproverKeyDown stepInfo.id
            , onCloseDropdown = CloseApproverDropdown stepInfo.id
            }
        ]


{-| タイトルのエラー表示
-}
viewTitleError : EditingState -> Html Msg
viewTitleError editing =
    case Dict.get "title" editing.validationErrors of
        Just errorMsg ->
            div
                [ class "mt-1 text-sm text-error-600" ]
                [ text errorMsg ]

        Nothing ->
            text ""


{-| 動的フォームフィールドを描画
-}
viewDynamicFormFields : EditingState -> Html Msg
viewDynamicFormFields editing =
    case DynamicForm.extractFormFields editing.selectedDefinition.definition of
        Ok fields ->
            if List.isEmpty fields then
                text ""

            else
                div
                    [ class "mb-6 rounded-lg bg-secondary-50 p-4" ]
                    [ h4
                        [ class "mb-4 text-secondary-900" ]
                        [ text (editing.selectedDefinition.name ++ " フォーム") ]
                    , DynamicForm.viewFields
                        fields
                        editing.formValues
                        editing.validationErrors
                        UpdateField
                    ]

        Err _ ->
            div
                [ class "rounded bg-error-50 p-4 text-error-600" ]
                [ text "フォーム定義の読み込みに失敗しました。" ]


{-| アクションボタン
-}
viewActions : EditingState -> Html Msg
viewActions editing =
    div
        [ class "mt-8 flex justify-end gap-4 border-t border-secondary-100 pt-4" ]
        [ Button.view
            { variant = Button.Outline
            , disabled = editing.submitting
            , onClick = SaveDraft
            }
            [ text "下書き保存" ]
        , Button.view
            { variant = Button.Primary
            , disabled = editing.submitting
            , onClick = Submit
            }
            [ text "申請する" ]
        ]
