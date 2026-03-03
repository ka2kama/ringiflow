module Page.Workflow.Detail.StepProgress exposing (viewStepProgress)

{-| ステップ進捗バー

ワークフローの承認ステップの進行状況を水平プログレスバーで表示する。
状態管理を持たない純粋 View モジュール。

-}

import Data.WorkflowInstance as WorkflowInstance exposing (WorkflowInstance, WorkflowStep)
import Html exposing (..)
import Html.Attributes exposing (..)


{-| ステップ進行状況の水平プログレス表示

全ステップを水平に並べ、ステータスに応じた色分けで進行状況を可視化する。
ステップが1つ以下の場合は表示しない（単一ステップの場合はプログレス表示の意味がない）。

-}
viewStepProgress : WorkflowInstance -> Html msg
viewStepProgress workflow =
    if List.length workflow.steps <= 1 then
        text ""

    else
        div [ class "rounded-lg border border-secondary-200 bg-white p-4 shadow-sm" ]
            [ h2 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text "進行状況" ]
            , let
                totalSteps =
                    List.length workflow.steps
              in
              div [ class "flex items-center gap-1" ]
                (workflow.steps
                    |> List.indexedMap
                        (\index step ->
                            let
                                isLast =
                                    index == totalSteps - 1
                            in
                            div [ class "flex items-center gap-1 flex-1 min-w-0" ]
                                (viewStepProgressItem step
                                    :: (if isLast then
                                            []

                                        else
                                            [ viewStepConnector step ]
                                       )
                                )
                        )
                )
            ]


viewStepProgressItem : WorkflowStep -> Html msg
viewStepProgressItem step =
    let
        ( bgClass, textClass, borderClass ) =
            stepProgressStyle step
    in
    div [ class ("flex flex-col items-center min-w-0 flex-1 " ++ borderClass) ]
        [ div [ class ("flex h-8 w-8 items-center justify-center rounded-full text-xs font-bold " ++ bgClass ++ " " ++ textClass) ]
            [ text (String.fromInt step.displayNumber) ]
        , span [ class "mt-1 text-xs text-secondary-600 truncate max-w-full text-center" ]
            [ text step.stepName ]
        , case step.assignedTo of
            Just assignee ->
                span [ class "text-xs text-secondary-400 truncate max-w-full" ] [ text assignee.name ]

            Nothing ->
                text ""
        ]


viewStepConnector : WorkflowStep -> Html msg
viewStepConnector step =
    let
        connectorClass =
            if step.status == WorkflowInstance.StepCompleted then
                "bg-success-400"

            else
                "bg-secondary-200"
    in
    div [ class ("h-0.5 flex-1 " ++ connectorClass) ] []


{-| ステップの進行状況スタイル
-}
stepProgressStyle : WorkflowStep -> ( String, String, String )
stepProgressStyle step =
    case ( step.status, step.decision ) of
        ( WorkflowInstance.StepCompleted, Just WorkflowInstance.DecisionApproved ) ->
            ( "bg-success-100", "text-success-700", "" )

        ( WorkflowInstance.StepCompleted, Just WorkflowInstance.DecisionRejected ) ->
            ( "bg-error-100", "text-error-700", "" )

        ( WorkflowInstance.StepCompleted, Just WorkflowInstance.DecisionRequestChanges ) ->
            ( "bg-warning-100", "text-warning-700", "" )

        ( WorkflowInstance.StepActive, _ ) ->
            ( "bg-info-100", "text-info-700", "ring-2 ring-info-300" )

        _ ->
            ( "bg-secondary-100", "text-secondary-500", "" )
