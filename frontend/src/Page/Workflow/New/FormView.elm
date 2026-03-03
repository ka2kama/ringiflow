module Page.Workflow.New.FormView exposing
    ( viewDefinitionSelector
    , viewFormInputs
    , viewSaveMessage
    )

{-| New ページのフォーム表示

定義選択、タイトル入力、動的フォームフィールド、
承認者選択、アクションボタンの UI を提供する。

-}

import Api exposing (ApiError)
import Component.ApproverSelector as ApproverSelector
import Component.Button as Button
import Data.UserItem exposing (UserItem)
import Data.WorkflowDefinition as WorkflowDefinition exposing (WorkflowDefinition)
import Dict
import Form.DynamicForm as DynamicForm
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events
import Page.Workflow.New.Types exposing (..)
import RemoteData exposing (RemoteData)


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
