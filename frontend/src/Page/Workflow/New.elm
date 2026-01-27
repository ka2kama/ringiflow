module Page.Workflow.New exposing
    ( Model
    , Msg
    , init
    , update
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

import Api.Http exposing (ApiError)
import Api.Workflow as WorkflowApi
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Data.WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance exposing (WorkflowInstance)
import Dict exposing (Dict)
import Form.DynamicForm as DynamicForm
import Form.Validation as Validation
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events
import Json.Encode as Encode
import Session exposing (Session)



-- MODEL


{-| ページの状態
-}
type alias Model =
    { -- セッション（API 呼び出しに必要）
      session : Session

    -- API データ
    , definitions : RemoteData (List WorkflowDefinition)
    , selectedDefinitionId : Maybe String

    -- フォーム状態
    , title : String
    , formValues : Dict String String
    , validationErrors : Dict String String

    -- 保存状態
    , savedWorkflow : Maybe WorkflowInstance
    , saveMessage : Maybe SaveMessage

    -- 操作状態
    , submitting : Bool
    }


{-| 保存結果メッセージ
-}
type SaveMessage
    = SaveSuccess String
    | SaveError String


{-| リモートデータの状態

API レスポンスのライフサイクルを型で表現する。

-}
type RemoteData a
    = NotAsked
    | Loading
    | Failure ApiError
    | Success a


{-| 初期化

ページ表示時にワークフロー定義一覧を取得する。

-}
init : Session -> ( Model, Cmd Msg )
init session =
    ( { session = session
      , definitions = Loading
      , selectedDefinitionId = Nothing
      , title = ""
      , formValues = Dict.empty
      , validationErrors = Dict.empty
      , savedWorkflow = Nothing
      , saveMessage = Nothing
      , submitting = False
      }
    , fetchDefinitions session
    )


{-| ワークフロー定義一覧を取得
-}
fetchDefinitions : Session -> Cmd Msg
fetchDefinitions session =
    WorkflowDefinitionApi.listDefinitions
        { config = Session.toRequestConfig session
        , toMsg = GotDefinitions
        }



-- UPDATE


{-| メッセージ
-}
type Msg
    = -- 初期化
      GotDefinitions (Result ApiError (List WorkflowDefinition))
      -- ワークフロー定義選択
    | SelectDefinition String
      -- フォーム入力
    | UpdateTitle String
    | UpdateField String String
      -- 保存・申請
    | SaveDraft
    | GotSaveResult (Result ApiError WorkflowInstance)
    | Submit
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

        SelectDefinition definitionId ->
            ( { model
                | selectedDefinitionId = Just definitionId
                , formValues = Dict.empty
                , validationErrors = Dict.empty
              }
            , Cmd.none
            )

        UpdateTitle newTitle ->
            ( { model | title = newTitle }
            , Cmd.none
            )

        UpdateField fieldId value ->
            ( { model | formValues = Dict.insert fieldId value model.formValues }
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

                ( _, Err msg ) ->
                    ( { model
                        | validationErrors = Dict.singleton "title" msg
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
                    , saveDraft model.session definitionId model.title model.formValues
                    )

        GotSaveResult result ->
            case result of
                Ok workflow ->
                    ( { model
                        | submitting = False
                        , savedWorkflow = Just workflow
                        , saveMessage = Just (SaveSuccess "下書きを保存しました")
                      }
                    , Cmd.none
                    )

                Err _ ->
                    ( { model
                        | submitting = False
                        , saveMessage = Just (SaveError "保存に失敗しました。もう一度お試しください。")
                      }
                    , Cmd.none
                    )

        Submit ->
            -- 申請時は全項目バリデーション
            let
                validationErrors =
                    validateForm model
            in
            if Dict.isEmpty validationErrors then
                -- TODO: 申請 API 呼び出し（Sub-Phase 2-9 で実装）
                ( model, Cmd.none )

            else
                ( { model
                    | validationErrors = validationErrors
                    , saveMessage = Nothing
                  }
                , Cmd.none
                )

        GotSubmitResult result ->
            case result of
                Ok _ ->
                    -- TODO: 申請完了後の遷移（Sub-Phase 2-9 で実装）
                    ( { model | submitting = False }
                    , Cmd.none
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


{-| フォーム全体のバリデーション

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


{-| 選択されたワークフロー定義を取得
-}
getSelectedDefinition : Maybe String -> List WorkflowDefinition -> Maybe WorkflowDefinition
getSelectedDefinition maybeId definitions =
    maybeId
        |> Maybe.andThen
            (\defId ->
                List.filter (\d -> d.id == defId) definitions
                    |> List.head
            )


{-| 下書き保存 API を呼び出す
-}
saveDraft : Session -> String -> String -> Dict String String -> Cmd Msg
saveDraft session definitionId title formValues =
    WorkflowApi.createWorkflow
        { config = Session.toRequestConfig session
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



-- VIEW


{-| ページの描画
-}
view : Model -> Html Msg
view model =
    div []
        [ h2 [] [ text "新規申請" ]
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
                [ style "padding" "1rem"
                , style "margin-bottom" "1rem"
                , style "background-color" "#e6f4ea"
                , style "color" "#137333"
                , style "border-radius" "4px"
                , style "display" "flex"
                , style "justify-content" "space-between"
                , style "align-items" "center"
                ]
                [ text message
                , button
                    [ Html.Events.onClick ClearMessage
                    , style "background" "none"
                    , style "border" "none"
                    , style "cursor" "pointer"
                    , style "font-size" "1.25rem"
                    , style "color" "#137333"
                    ]
                    [ text "×" ]
                ]

        Just (SaveError message) ->
            div
                [ style "padding" "1rem"
                , style "margin-bottom" "1rem"
                , style "background-color" "#fce8e6"
                , style "color" "#d93025"
                , style "border-radius" "4px"
                , style "display" "flex"
                , style "justify-content" "space-between"
                , style "align-items" "center"
                ]
                [ text message
                , button
                    [ Html.Events.onClick ClearMessage
                    , style "background" "none"
                    , style "border" "none"
                    , style "cursor" "pointer"
                    , style "font-size" "1.25rem"
                    , style "color" "#d93025"
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
    div
        [ style "text-align" "center"
        , style "padding" "2rem"
        , style "color" "#5f6368"
        ]
        [ text "読み込み中..." ]


{-| エラー表示
-}
viewError : Html Msg
viewError =
    div
        [ style "text-align" "center"
        , style "padding" "2rem"
        , style "color" "#d93025"
        ]
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
                |> Maybe.andThen
                    (\defId ->
                        List.filter (\d -> d.id == defId) definitions
                            |> List.head
                    )
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
        [ style "margin-bottom" "2rem" ]
        [ h3 [] [ text "Step 1: ワークフロー種類を選択" ]
        , div
            [ style "display" "flex"
            , style "flex-direction" "column"
            , style "gap" "0.5rem"
            ]
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
        [ style "display" "flex"
        , style "align-items" "center"
        , style "padding" "1rem"
        , style "border" "1px solid #dadce0"
        , style "border-radius" "8px"
        , style "cursor" "pointer"
        , style "background-color"
            (if isSelected then
                "#e8f0fe"

             else
                "white"
            )
        ]
        [ input
            [ type_ "radio"
            , name "workflow-definition"
            , Html.Attributes.value definition.id
            , checked isSelected
            , Html.Events.onClick (SelectDefinition definition.id)
            , style "margin-right" "1rem"
            ]
            []
        , div []
            [ div [ style "font-weight" "500" ] [ text definition.name ]
            , case definition.description of
                Just desc ->
                    div [ style "color" "#5f6368", style "font-size" "0.875rem" ]
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
        [ h3 [] [ text "Step 2: フォーム入力" ]

        -- タイトル入力
        , div [ style "margin-bottom" "1.5rem" ]
            [ label
                [ for "title"
                , style "display" "block"
                , style "margin-bottom" "0.5rem"
                , style "font-weight" "500"
                ]
                [ text "タイトル"
                , span [ style "color" "#d93025" ] [ text " *" ]
                ]
            , input
                [ type_ "text"
                , id "title"
                , Html.Attributes.value model.title
                , Html.Events.onInput UpdateTitle
                , placeholder "申請のタイトルを入力"
                , style "width" "100%"
                , style "padding" "0.75rem"
                , style "border" "1px solid #dadce0"
                , style "border-radius" "4px"
                , style "font-size" "1rem"
                , style "box-sizing" "border-box"
                ]
                []
            , viewTitleError model
            ]

        -- 動的フォームフィールド
        , viewDynamicFormFields definition model

        -- アクションボタン
        , viewActions model
        ]


{-| タイトルのエラー表示
-}
viewTitleError : Model -> Html Msg
viewTitleError model =
    case Dict.get "title" model.validationErrors of
        Just errorMsg ->
            div
                [ style "color" "#d93025"
                , style "font-size" "0.875rem"
                , style "margin-top" "0.25rem"
                ]
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
                    [ style "margin-bottom" "1.5rem"
                    , style "padding" "1rem"
                    , style "background-color" "#f8f9fa"
                    , style "border-radius" "8px"
                    ]
                    [ h4
                        [ style "margin" "0 0 1rem 0"
                        , style "color" "#202124"
                        ]
                        [ text (definition.name ++ " フォーム") ]
                    , DynamicForm.viewFields
                        fields
                        model.formValues
                        model.validationErrors
                        UpdateField
                    ]

        Err _ ->
            div
                [ style "color" "#d93025"
                , style "padding" "1rem"
                , style "background-color" "#fce8e6"
                , style "border-radius" "4px"
                ]
                [ text "フォーム定義の読み込みに失敗しました。" ]


{-| アクションボタン
-}
viewActions : Model -> Html Msg
viewActions model =
    div
        [ style "display" "flex"
        , style "justify-content" "flex-end"
        , style "gap" "1rem"
        , style "margin-top" "2rem"
        , style "padding-top" "1rem"
        , style "border-top" "1px solid #dadce0"
        ]
        [ button
            [ Html.Events.onClick SaveDraft
            , disabled model.submitting
            , style "padding" "0.75rem 1.5rem"
            , style "border" "1px solid #dadce0"
            , style "border-radius" "4px"
            , style "background-color" "white"
            , style "cursor" "pointer"
            ]
            [ text "下書き保存" ]
        , button
            [ Html.Events.onClick Submit
            , disabled model.submitting
            , style "padding" "0.75rem 1.5rem"
            , style "border" "none"
            , style "border-radius" "4px"
            , style "background-color" "#1a73e8"
            , style "color" "white"
            , style "cursor" "pointer"
            ]
            [ text "申請する" ]
        ]
