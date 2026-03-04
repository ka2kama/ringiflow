module Page.WorkflowDefinition.Designer.CanvasTransitions exposing
    ( viewArrowDefs
    , viewConnectionDragPreview
    , viewReconnectionHandleLayer
    , viewTransitions
    )

{-| 接続線（Transition）の SVG 描画

接続線・矢印マーカー・ドラッグプレビュー・付け替えハンドルなど、
Transition 関連の SVG 要素を描画する。

ステップノードの描画は Canvas.elm が担当する。

-}

import Data.DesignerCanvas as DesignerCanvas exposing (DraggingState(..), ReconnectEnd(..), Transition)
import Dict
import Html.Events
import Json.Decode as Decode
import List.Extra
import Page.WorkflowDefinition.Designer.Types exposing (CanvasState, Msg(..))
import Svg
import Svg.Attributes as SvgAttr


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
            case List.Extra.getAt index canvas.transitions of
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

                    pathData =
                        bezierPathData startPos endPos

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
    Svg.path
        [ SvgAttr.d (bezierPathData from to)
        , SvgAttr.fill "none"
        , SvgAttr.stroke "#94a3b8"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.strokeDasharray "6 3"
        , SvgAttr.markerEnd "url(#arrow-none)"
        , SvgAttr.class "pointer-events-none"
        ]
        []


{-| 2 点間のベジェ曲線パスデータを生成する

垂直方向に 1/3 オフセットの制御点を配置するキュービックベジェ曲線。
viewTransitionLine と viewPreviewLine で共通使用する。

-}
bezierPathData : DesignerCanvas.Position -> DesignerCanvas.Position -> String
bezierPathData from to =
    let
        dy =
            abs (to.y - from.y) / 3
    in
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
