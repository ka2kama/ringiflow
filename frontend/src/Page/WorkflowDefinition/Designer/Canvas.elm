module Page.WorkflowDefinition.Designer.Canvas exposing (viewCanvasArea)

{-| SVG キャンバス描画

ワークフローのステップノード・グリッド・ドラッグプレビューなどの
SVG キャンバス要素を描画する。

接続線（Transition）関連の描画は CanvasTransitions モジュールが担当する。

-}

import Data.DesignerCanvas as DesignerCanvas exposing (DraggingState(..), viewBoxHeight, viewBoxWidth)
import Dict
import Html exposing (Html, div)
import Html.Attributes exposing (class)
import Html.Events
import Json.Decode as Decode
import Page.WorkflowDefinition.Designer.CanvasTransitions as CanvasTransitions
import Page.WorkflowDefinition.Designer.Types exposing (CanvasState, Msg(..), canvasElementId)
import Svg exposing (svg)
import Svg.Attributes as SvgAttr
import Svg.Events


{-| SVG キャンバスエリア

マウスイベントは Browser.Events（グローバル）で処理する。
背景クリック選択解除用のイベントのみ SVG 要素に設定。

-}
viewCanvasArea : CanvasState -> Html Msg
viewCanvasArea canvas =
    div [ class "flex-1 overflow-hidden bg-secondary-50" ]
        [ svg
            [ SvgAttr.id canvasElementId
            , SvgAttr.viewBox
                ("0 0 "
                    ++ String.fromFloat viewBoxWidth
                    ++ " "
                    ++ String.fromFloat viewBoxHeight
                )
            , SvgAttr.width "100%"
            , SvgAttr.height "100%"
            , SvgAttr.class "block"
            ]
            [ -- SVG レイヤー順: 先に描画 = 背面、後に描画 = 前面
              CanvasTransitions.viewArrowDefs
            , viewCanvasBackground
            , viewGrid
            , CanvasTransitions.viewTransitions canvas
            , CanvasTransitions.viewConnectionDragPreview canvas
            , viewSteps canvas
            , CanvasTransitions.viewReconnectionHandleLayer canvas
            , viewDragPreview canvas
            ]
        ]


{-| キャンバス背景（クリック選択解除用の透明レイヤー）

SVG 要素のクリック判定に使用。最背面に配置し、ステップがない領域のクリックを検出する。

-}
viewCanvasBackground : Svg.Svg Msg
viewCanvasBackground =
    Svg.rect
        [ SvgAttr.width (String.fromFloat viewBoxWidth)
        , SvgAttr.height (String.fromFloat viewBoxHeight)
        , SvgAttr.fill "transparent"
        , SvgAttr.class "pointer-events-all"
        , Svg.Events.onClick CanvasBackgroundClicked
        ]
        []


{-| グリッド線の描画

20px 間隔の薄い線。SVG viewBox 座標系で描画する。

-}
viewGrid : Svg.Svg Msg
viewGrid =
    let
        gridSpacing =
            20

        verticalLines =
            List.range 0 (floor (viewBoxWidth / gridSpacing))
                |> List.map
                    (\i ->
                        let
                            x =
                                String.fromInt (i * gridSpacing)
                        in
                        Svg.line
                            [ SvgAttr.x1 x
                            , SvgAttr.y1 "0"
                            , SvgAttr.x2 x
                            , SvgAttr.y2 (String.fromFloat viewBoxHeight)
                            , SvgAttr.stroke "#e2e8f0"
                            , SvgAttr.strokeWidth "0.5"
                            ]
                            []
                    )

        horizontalLines =
            List.range 0 (floor (viewBoxHeight / gridSpacing))
                |> List.map
                    (\i ->
                        let
                            y =
                                String.fromInt (i * gridSpacing)
                        in
                        Svg.line
                            [ SvgAttr.x1 "0"
                            , SvgAttr.y1 y
                            , SvgAttr.x2 (String.fromFloat viewBoxWidth)
                            , SvgAttr.y2 y
                            , SvgAttr.stroke "#e2e8f0"
                            , SvgAttr.strokeWidth "0.5"
                            ]
                            []
                    )
    in
    Svg.g [ SvgAttr.pointerEvents "none" ] (verticalLines ++ horizontalLines)


{-| 配置済みステップの描画
-}
viewSteps : CanvasState -> Svg.Svg Msg
viewSteps canvas =
    let
        errorStepIds =
            canvas.validationResult
                |> Maybe.map .errors
                |> Maybe.withDefault []
                |> List.filterMap .stepId
    in
    Svg.g []
        (canvas.steps
            |> Dict.values
            |> List.map (viewStepNode canvas.selectedStepId errorStepIds)
        )


{-| 個別のステップノード描画
-}
viewStepNode : Maybe String -> List String -> DesignerCanvas.StepNode -> Svg.Svg Msg
viewStepNode selectedStepId errorStepIds step =
    let
        colors =
            DesignerCanvas.stepColors step.stepType

        dim =
            DesignerCanvas.stepDimensions

        isSelected =
            selectedStepId == Just step.id

        hasError =
            List.member step.id errorStepIds

        strokeColor =
            if hasError then
                "#dc2626"

            else
                colors.stroke

        strokeWidth =
            if isSelected then
                "3"

            else if hasError then
                "3"

            else
                "2"
    in
    Svg.g
        [ SvgAttr.transform
            ("translate("
                ++ String.fromFloat step.position.x
                ++ ","
                ++ String.fromFloat step.position.y
                ++ ")"
            )
        , SvgAttr.class "cursor-move"
        , Html.Events.stopPropagationOn "mousedown"
            (Decode.map2 (\cx cy -> ( StepMouseDown step.id cx cy, True ))
                (Decode.field "clientX" Decode.float)
                (Decode.field "clientY" Decode.float)
            )
        ]
        [ -- ステップ背景
          Svg.rect
            [ SvgAttr.width (String.fromFloat dim.width)
            , SvgAttr.height (String.fromFloat dim.height)
            , SvgAttr.rx "12"
            , SvgAttr.fill colors.fill
            , SvgAttr.stroke strokeColor
            , SvgAttr.strokeWidth strokeWidth
            ]
            []

        -- 選択ハイライト（外枠リング）
        , if isSelected then
            Svg.rect
                [ SvgAttr.x "-5"
                , SvgAttr.y "-5"
                , SvgAttr.width (String.fromFloat (dim.width + 10))
                , SvgAttr.height (String.fromFloat (dim.height + 10))
                , SvgAttr.rx "17"
                , SvgAttr.fill "none"
                , SvgAttr.stroke "#6366f1"
                , SvgAttr.strokeWidth "2"
                , SvgAttr.strokeDasharray "4 2"
                , SvgAttr.opacity "0.5"
                ]
                []

          else
            Svg.text ""

        -- ステップ名テキスト
        , Svg.text_
            [ SvgAttr.x (String.fromFloat (dim.width / 2))
            , SvgAttr.y (String.fromFloat (dim.height / 2))
            , SvgAttr.textAnchor "middle"
            , SvgAttr.dominantBaseline "central"
            , SvgAttr.fill colors.stroke
            , SvgAttr.fontSize "18"
            , SvgAttr.fontWeight "500"
            , SvgAttr.class "pointer-events-none select-none"
            ]
            [ Svg.text step.name ]

        -- 出力ポート（下端中央の円）
        , Svg.circle
            [ SvgAttr.cx (String.fromFloat (dim.width / 2))
            , SvgAttr.cy (String.fromFloat dim.height)
            , SvgAttr.r "7"
            , SvgAttr.fill colors.stroke
            , SvgAttr.stroke "white"
            , SvgAttr.strokeWidth "2"
            , SvgAttr.class "cursor-crosshair"
            , Html.Events.stopPropagationOn "mousedown"
                (Decode.map2 (\cx cy -> ( ConnectionPortMouseDown step.id cx cy, True ))
                    (Decode.field "clientX" Decode.float)
                    (Decode.field "clientY" Decode.float)
                )
            ]
            []

        -- 入力ポート（上端中央の円）
        , Svg.circle
            [ SvgAttr.cx (String.fromFloat (dim.width / 2))
            , SvgAttr.cy "0"
            , SvgAttr.r "7"
            , SvgAttr.fill colors.stroke
            , SvgAttr.stroke "white"
            , SvgAttr.strokeWidth "2"
            , SvgAttr.class "pointer-events-none"
            ]
            []
        ]


{-| ドラッグ中のプレビュー表示

パレットからの新規配置時、マウス位置にゴーストステップを表示する。

-}
viewDragPreview : CanvasState -> Svg.Svg Msg
viewDragPreview canvas =
    case canvas.dragging of
        Just (DraggingNewStep stepType pos) ->
            let
                colors =
                    DesignerCanvas.stepColors stepType

                dim =
                    DesignerCanvas.stepDimensions

                clampedPos =
                    DesignerCanvas.clampToViewBox
                        { x = DesignerCanvas.snapToGrid pos.x
                        , y = DesignerCanvas.snapToGrid pos.y
                        }
            in
            Svg.g
                [ SvgAttr.transform
                    ("translate("
                        ++ String.fromFloat clampedPos.x
                        ++ ","
                        ++ String.fromFloat clampedPos.y
                        ++ ")"
                    )
                , SvgAttr.opacity "0.6"
                , SvgAttr.class "pointer-events-none"
                ]
                [ Svg.rect
                    [ SvgAttr.width (String.fromFloat dim.width)
                    , SvgAttr.height (String.fromFloat dim.height)
                    , SvgAttr.rx "12"
                    , SvgAttr.fill colors.fill
                    , SvgAttr.stroke colors.stroke
                    , SvgAttr.strokeWidth "2"
                    , SvgAttr.strokeDasharray "6 3"
                    ]
                    []
                , Svg.text_
                    [ SvgAttr.x (String.fromFloat (dim.width / 2))
                    , SvgAttr.y (String.fromFloat (dim.height / 2))
                    , SvgAttr.textAnchor "middle"
                    , SvgAttr.dominantBaseline "central"
                    , SvgAttr.fill colors.stroke
                    , SvgAttr.fontSize "18"
                    , SvgAttr.fontWeight "500"
                    ]
                    [ Svg.text (DesignerCanvas.defaultStepName stepType) ]
                ]

        _ ->
            Svg.text ""
