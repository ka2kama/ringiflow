module Page.WorkflowDefinition.Designer.PropertyPanel exposing (viewPropertyPanel)

{-| プロパティパネル

選択中の要素のプロパティを表示・編集するパネル。
接続線選択時は接続情報（読み取り専用）と削除 UI を表示し、
ステップ選択時はステップ種別に応じたフィールドを表示する。

-}

import Component.Button as Button
import Component.FormField as FormField
import Data.DesignerCanvas as DesignerCanvas exposing (StepNode, StepType(..), Transition)
import Dict
import Html exposing (..)
import Html.Attributes exposing (..)
import List.Extra
import Maybe.Extra
import Page.WorkflowDefinition.Designer.Types exposing (CanvasState, Msg(..))


{-| プロパティパネル（右サイドバー）
-}
viewPropertyPanel : CanvasState -> Html Msg
viewPropertyPanel canvas =
    div [ class "w-64 shrink-0 border-l border-secondary-200 bg-white p-4 overflow-y-auto" ]
        [ h2 [ class "mb-3 text-xs font-semibold uppercase tracking-wider text-secondary-500" ]
            [ text "プロパティ" ]
        , case canvas.selectedTransitionIndex of
            Just index ->
                case List.Extra.getAt index canvas.transitions of
                    Just transition ->
                        viewTransitionProperties canvas transition

                    Nothing ->
                        viewNoSelection

            Nothing ->
                case canvas.selectedStepId of
                    Just stepId ->
                        case Dict.get stepId canvas.steps of
                            Just step ->
                                viewStepProperties canvas step

                            Nothing ->
                                viewNoSelection

                    Nothing ->
                        viewNoSelection
        ]


{-| 未選択時のプレースホルダー
-}
viewNoSelection : Html msg
viewNoSelection =
    p [ class "text-sm text-secondary-400" ]
        [ text "ステップまたは接続線を選択してください" ]


{-| 接続線のプロパティ表示
-}
viewTransitionProperties : CanvasState -> Transition -> Html Msg
viewTransitionProperties canvas transition =
    let
        stepName stepId =
            Dict.get stepId canvas.steps
                |> Maybe.Extra.unwrap stepId .name
    in
    div [ class "space-y-4" ]
        [ -- 種別ラベル
          div [ class "mb-2" ]
            [ span
                [ class "inline-block rounded-full bg-secondary-100 px-2 py-0.5 text-xs font-medium text-secondary-600" ]
                [ text "接続" ]
            ]
        , FormField.viewReadOnlyField "transition-from" "接続元" (stepName transition.from)
        , FormField.viewReadOnlyField "transition-to" "接続先" (stepName transition.to)
        , FormField.viewReadOnlyField "transition-trigger" "トリガー" (DesignerCanvas.triggerLabel transition.trigger)
        , div [ class "mt-6 border-t border-secondary-200 pt-4" ]
            [ Button.view
                { variant = Button.Error
                , disabled = False
                , onClick = DeleteSelectedTransition
                }
                [ text "接続を削除" ]
            , p [ class "mt-1 text-xs text-secondary-400" ]
                [ text "Delete キーでも削除できます" ]
            ]
        ]


{-| ステップ種別に応じたプロパティフィールド
-}
viewStepProperties : CanvasState -> StepNode -> Html Msg
viewStepProperties canvas step =
    div [ class "space-y-4" ]
        ([ -- 種別ラベル
           div [ class "mb-2" ]
            [ span
                [ class "inline-block rounded-full px-2 py-0.5 text-xs font-medium"
                , style "background-color" (DesignerCanvas.stepColors step.stepType).fill
                , style "color" (DesignerCanvas.stepColors step.stepType).stroke
                ]
                [ text (DesignerCanvas.defaultStepName step.stepType) ]
            ]

         -- ステップ名（全種別共通）
         , FormField.viewTextField
            { label = "ステップ名"
            , value = canvas.propertyName
            , onInput = UpdatePropertyName
            , error = Nothing
            , inputType = "text"
            , placeholder = "ステップ名を入力"
            , fieldId = "step-name"
            }
         ]
            ++ viewStepTypeSpecificFields canvas step
            ++ [ div [ class "mt-6 border-t border-secondary-200 pt-4" ]
                    [ Button.view
                        { variant = Button.Error
                        , disabled = False
                        , onClick = DeleteSelectedStep
                        }
                        [ text "ステップを削除" ]
                    , p [ class "mt-1 text-xs text-secondary-400" ]
                        [ text "Delete キーでも削除できます" ]
                    ]
               ]
        )


{-| ステップ種別固有のフィールド
-}
viewStepTypeSpecificFields : CanvasState -> StepNode -> List (Html Msg)
viewStepTypeSpecificFields canvas step =
    case step.stepType of
        Start ->
            []

        Approval ->
            [ FormField.viewReadOnlyField "step-approver" "承認者指定" "申請時にユーザーを選択" ]

        End ->
            [ FormField.viewSelectField
                { label = "終了ステータス"
                , value = canvas.propertyEndStatus
                , onInput = UpdatePropertyEndStatus
                , error = Nothing
                , options =
                    [ { value = "approved", label = "承認" }
                    , { value = "rejected", label = "却下" }
                    ]
                , placeholder = "選択してください"
                , fieldId = "step-end-status"
                }
            ]
