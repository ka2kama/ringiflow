module Page.WorkflowDefinition.Designer.Canvas exposing (viewCanvasArea)

{-| SVG キャンバス描画

ワークフローのステップ・接続線・ドラッグプレビューなどの
SVG キャンバス要素を描画する。

-}

import Data.DesignerCanvas as DesignerCanvas exposing (DraggingState(..), ReconnectEnd(..), StepNode, Transition, viewBoxHeight, viewBoxWidth)
import Dict
import Html exposing (Html, div)
import Html.Attributes exposing (class)
import Html.Events
import Json.Decode as Decode
import List.Extra
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
              viewArrowDefs
            , viewCanvasBackground
            , viewGrid
            , viewTransitions canvas
            , viewConnectionDragPreview canvas
            , viewSteps canvas
            , viewReconnectionHandleLayer canvas
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
viewStepNode : Maybe String -> List String -> StepNode -> Svg.Svg Msg
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


{-| SVG マーカー定義（矢印の先端形状）

trigger に応じて3種の矢印を定義する:

  - arrow-none: グレー（trigger なし）
  - arrow-approve: 緑（承認）
  - arrow-reject: 赤（却下）

-}
viewArrowDefs : Svg.Svg Msg
viewArrowDefs =
    Svg.defs []
        [ viewArrowMarker "arrow-none" "#94a3b8"
        , viewArrowMarker "arrow-approve" "#059669"
        , viewArrowMarker "arrow-reject" "#dc2626"
        ]


{-| 個別の矢印マーカー定義
-}
viewArrowMarker : String -> String -> Svg.Svg Msg
viewArrowMarker markerId color =
    Svg.marker
        [ SvgAttr.id markerId
        , SvgAttr.viewBox "0 0 10 10"
        , SvgAttr.refX "10"
        , SvgAttr.refY "5"
        , SvgAttr.markerWidth "10"
        , SvgAttr.markerHeight "10"
        , SvgAttr.orient "auto"
        ]
        [ Svg.path
            [ SvgAttr.d "M 0 0 L 10 5 L 0 10 Z"
            , SvgAttr.fill color
            ]
            []
        ]


{-| 接続線の描画

全 Transition をベジェ曲線で描画する。trigger に応じて色と線種を変える:

  - approve: 実線 + 緑矢印
  - reject: 破線 + 赤矢印
  - none: 実線 + グレー矢印

-}
viewTransitions : CanvasState -> Svg.Svg Msg
viewTransitions canvas =
    Svg.g []
        (canvas.transitions
            |> List.indexedMap (viewTransitionLine canvas)
        )


{-| 選択中の接続線のドラッグハンドルレイヤー

ステップノードより上（前面）に描画するために、viewSteps の後に配置する。

-}
viewReconnectionHandleLayer : CanvasState -> Svg.Svg Msg
viewReconnectionHandleLayer canvas =
    case ( canvas.selectedTransitionIndex, canvas.dragging ) of
        ( Just index, Nothing ) ->
            case List.drop index canvas.transitions |> List.head of
                Just transition ->
                    let
                        fromStep =
                            Dict.get transition.from canvas.steps

                        toStep =
                            Dict.get transition.to canvas.steps
                    in
                    case ( fromStep, toStep ) of
                        ( Just from, Just to ) ->
                            viewReconnectionHandles index
                                (DesignerCanvas.stepOutputPortPosition from)
                                (DesignerCanvas.stepInputPortPosition to)

                        _ ->
                            Svg.text ""

                Nothing ->
                    Svg.text ""

        _ ->
            Svg.text ""


{-| 個別の接続線描画
-}
viewTransitionLine : CanvasState -> Int -> Transition -> Svg.Svg Msg
viewTransitionLine canvas index transition =
    let
        -- DraggingReconnection 中は対象の接続線を非表示にする（プレビュー線のみ表示）
        isBeingReconnected =
            case canvas.dragging of
                Just (DraggingReconnection reconnectIndex _ _) ->
                    reconnectIndex == index

                _ ->
                    False
    in
    if isBeingReconnected then
        Svg.text ""

    else
        let
            fromStep =
                Dict.get transition.from canvas.steps

            toStep =
                Dict.get transition.to canvas.steps
        in
        case ( fromStep, toStep ) of
            ( Just from, Just to ) ->
                let
                    startPos =
                        DesignerCanvas.stepOutputPortPosition from

                    endPos =
                        DesignerCanvas.stepInputPortPosition to

                    -- ベジェ曲線の制御点（垂直方向に 1/3 オフセット）
                    dy =
                        abs (endPos.y - startPos.y) / 3

                    pathData =
                        "M "
                            ++ String.fromFloat startPos.x
                            ++ " "
                            ++ String.fromFloat startPos.y
                            ++ " C "
                            ++ String.fromFloat startPos.x
                            ++ " "
                            ++ String.fromFloat (startPos.y + dy)
                            ++ ", "
                            ++ String.fromFloat endPos.x
                            ++ " "
                            ++ String.fromFloat (endPos.y - dy)
                            ++ ", "
                            ++ String.fromFloat endPos.x
                            ++ " "
                            ++ String.fromFloat endPos.y

                    ( strokeColor, markerId, dashArray ) =
                        case transition.trigger of
                            Just "approve" ->
                                ( "#059669", "arrow-approve", "" )

                            Just "reject" ->
                                ( "#dc2626", "arrow-reject", "6 3" )

                            _ ->
                                ( "#94a3b8", "arrow-none", "" )

                    isSelected =
                        canvas.selectedTransitionIndex == Just index

                    strokeWidth =
                        if isSelected then
                            "3"

                        else
                            "2"
                in
                Svg.g []
                    [ -- クリック判定用の透明な太いパス
                      -- pointer-events="all" により stroke の塗り状態に依存せずクリックを受け取る
                      Svg.path
                        [ SvgAttr.d pathData
                        , SvgAttr.fill "none"
                        , SvgAttr.stroke "transparent"
                        , SvgAttr.strokeWidth "12"
                        , SvgAttr.pointerEvents "all"
                        , SvgAttr.class "cursor-pointer"
                        , Html.Events.stopPropagationOn "click"
                            (Decode.succeed ( TransitionClicked index, True ))
                        ]
                        []

                    -- 表示用のパス
                    , Svg.path
                        ([ SvgAttr.d pathData
                         , SvgAttr.fill "none"
                         , SvgAttr.stroke strokeColor
                         , SvgAttr.strokeWidth strokeWidth
                         , SvgAttr.markerEnd ("url(#" ++ markerId ++ ")")
                         , SvgAttr.class "pointer-events-none"
                         ]
                            ++ (if dashArray /= "" then
                                    [ SvgAttr.strokeDasharray dashArray ]

                                else
                                    []
                               )
                            ++ (if isSelected then
                                    [ SvgAttr.filter "drop-shadow(0 0 3px rgba(99, 102, 241, 0.5))" ]

                                else
                                    []
                               )
                        )
                        []

                    -- ハンドルは viewReconnectionHandleLayer で描画（ステップの上に表示するため）
                    ]

            _ ->
                Svg.text ""


{-| 接続線端点のドラッグハンドル

選択中の接続線の始点・終点に表示するハンドル。
ドラッグすることで接続先を変更できる。

-}
viewReconnectionHandles : Int -> DesignerCanvas.Position -> DesignerCanvas.Position -> Svg.Svg Msg
viewReconnectionHandles index startPos endPos =
    let
        handleAttrs pos reconnectEnd =
            [ SvgAttr.cx (String.fromFloat pos.x)
            , SvgAttr.cy (String.fromFloat pos.y)
            , SvgAttr.r "10"
            , SvgAttr.fill "white"
            , SvgAttr.stroke "#6366f1"
            , SvgAttr.strokeWidth "2.5"
            , SvgAttr.filter "drop-shadow(0 0 4px rgba(99, 102, 241, 0.6))"
            , SvgAttr.class "cursor-grab"
            , Html.Events.stopPropagationOn "mousedown"
                (Decode.map2
                    (\cx cy -> ( TransitionEndpointMouseDown index reconnectEnd cx cy, True ))
                    (Decode.field "clientX" Decode.float)
                    (Decode.field "clientY" Decode.float)
                )
            ]
    in
    Svg.g []
        [ -- 始点ハンドル
          Svg.circle (handleAttrs startPos SourceEnd) []

        -- 終点ハンドル
        , Svg.circle (handleAttrs endPos TargetEnd) []
        ]


{-| 接続線ドラッグ中のプレビュー

DraggingConnection / DraggingReconnection 中に破線を描画する。

  - DraggingConnection: 接続元の出力ポートから現在のマウス位置まで
  - DraggingReconnection: 固定端から現在のマウス位置まで（SourceEnd なら to 側固定、TargetEnd なら from 側固定）

-}
viewConnectionDragPreview : CanvasState -> Svg.Svg Msg
viewConnectionDragPreview canvas =
    case canvas.dragging of
        Just (DraggingConnection sourceId mousePos) ->
            case Dict.get sourceId canvas.steps of
                Just sourceStep ->
                    viewPreviewLine
                        (DesignerCanvas.stepOutputPortPosition sourceStep)
                        mousePos

                Nothing ->
                    Svg.text ""

        Just (DraggingReconnection index end mousePos) ->
            case List.Extra.getAt index canvas.transitions of
                Just transition ->
                    case end of
                        SourceEnd ->
                            -- 始点をドラッグ中: マウス位置 → to ステップの入力ポート
                            case Dict.get transition.to canvas.steps of
                                Just toStep ->
                                    viewPreviewLine mousePos (DesignerCanvas.stepInputPortPosition toStep)

                                Nothing ->
                                    Svg.text ""

                        TargetEnd ->
                            -- 終点をドラッグ中: from ステップの出力ポート → マウス位置
                            case Dict.get transition.from canvas.steps of
                                Just fromStep ->
                                    viewPreviewLine (DesignerCanvas.stepOutputPortPosition fromStep) mousePos

                                Nothing ->
                                    Svg.text ""

                Nothing ->
                    Svg.text ""

        _ ->
            Svg.text ""


{-| 接続プレビュー線の描画（共通）
-}
viewPreviewLine : DesignerCanvas.Position -> DesignerCanvas.Position -> Svg.Svg Msg
viewPreviewLine from to =
    let
        dy =
            abs (to.y - from.y) / 3

        pathData =
            "M "
                ++ String.fromFloat from.x
                ++ " "
                ++ String.fromFloat from.y
                ++ " C "
                ++ String.fromFloat from.x
                ++ " "
                ++ String.fromFloat (from.y + dy)
                ++ ", "
                ++ String.fromFloat to.x
                ++ " "
                ++ String.fromFloat (to.y - dy)
                ++ ", "
                ++ String.fromFloat to.x
                ++ " "
                ++ String.fromFloat to.y
    in
    Svg.path
        [ SvgAttr.d pathData
        , SvgAttr.fill "none"
        , SvgAttr.stroke "#94a3b8"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.strokeDasharray "6 3"
        , SvgAttr.markerEnd "url(#arrow-none)"
        , SvgAttr.class "pointer-events-none"
        ]
        []


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
