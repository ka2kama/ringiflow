module Page.WorkflowDefinition.Designer exposing (Model, Msg(..), init, subscriptions, update, updateShared, view)

{-| ワークフローデザイナー画面

SVG キャンバス上にワークフローのステップを配置・操作するビジュアルエディタ。
ADR-053 で決定した SVG + Elm 直接レンダリング方式に基づく。

-}

import Browser.Events
import Data.DesignerCanvas as DesignerCanvas exposing (Bounds, DraggingState(..), StepNode, StepType(..), viewBoxHeight, viewBoxWidth)
import Dict exposing (Dict)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onMouseDown)
import Json.Decode as Decode
import Json.Encode as Encode
import Ports
import Shared exposing (Shared)
import Svg exposing (svg)
import Svg.Attributes as SvgAttr
import Svg.Events



-- CONSTANTS


{-| キャンバス SVG 要素の HTML id
-}
canvasElementId : String
canvasElementId =
    "designer-canvas"



-- MODEL


type alias Model =
    { shared : Shared
    , steps : Dict String StepNode
    , selectedStepId : Maybe String
    , dragging : Maybe DraggingState
    , canvasBounds : Maybe Bounds
    , nextStepNumber : Int
    }


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , steps = Dict.empty
      , selectedStepId = Nothing
      , dragging = Nothing
      , canvasBounds = Nothing
      , nextStepNumber = 1
      }
    , Ports.requestCanvasBounds canvasElementId
    )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = PaletteMouseDown StepType
    | CanvasMouseMove Float Float
    | CanvasMouseUp
    | StepClicked String
    | CanvasBackgroundClicked
    | StepMouseDown String Float Float -- stepId, clientX, clientY
    | KeyDown String
    | GotCanvasBounds Encode.Value


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        PaletteMouseDown stepType ->
            ( { model
                | dragging = Just (DraggingNewStep stepType { x = 0, y = 0 })
              }
            , Ports.requestCanvasBounds canvasElementId
            )

        CanvasMouseMove clientX clientY ->
            case model.dragging of
                Just (DraggingNewStep stepType _) ->
                    case DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY of
                        Just canvasPos ->
                            ( { model
                                | dragging = Just (DraggingNewStep stepType canvasPos)
                              }
                            , Cmd.none
                            )

                        Nothing ->
                            ( model, Cmd.none )

                Just (DraggingExistingStep stepId offset) ->
                    case DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY of
                        Just canvasPos ->
                            let
                                newPos =
                                    { x = DesignerCanvas.snapToGrid (canvasPos.x - offset.x)
                                    , y = DesignerCanvas.snapToGrid (canvasPos.y - offset.y)
                                    }

                                updatedSteps =
                                    Dict.update stepId
                                        (Maybe.map (\step -> { step | position = newPos }))
                                        model.steps
                            in
                            ( { model | steps = updatedSteps }
                            , Cmd.none
                            )

                        Nothing ->
                            ( model, Cmd.none )

                Just (DraggingConnection sourceId _) ->
                    -- Phase 2 で接続線プレビュー更新を実装
                    case DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY of
                        Just canvasPos ->
                            ( { model | dragging = Just (DraggingConnection sourceId canvasPos) }
                            , Cmd.none
                            )

                        Nothing ->
                            ( model, Cmd.none )

                Nothing ->
                    ( model, Cmd.none )

        CanvasMouseUp ->
            case model.dragging of
                Just (DraggingNewStep stepType dropPos) ->
                    let
                        newStep =
                            DesignerCanvas.createStepFromDrop stepType model.nextStepNumber dropPos
                    in
                    ( { model
                        | steps = Dict.insert newStep.id newStep model.steps
                        , dragging = Nothing
                        , nextStepNumber = model.nextStepNumber + 1
                        , selectedStepId = Just newStep.id
                      }
                    , Cmd.none
                    )

                Just (DraggingExistingStep _ _) ->
                    ( { model | dragging = Nothing }
                    , Cmd.none
                    )

                Just (DraggingConnection _ _) ->
                    -- Phase 2 で接続ドロップ判定を実装
                    ( { model | dragging = Nothing }
                    , Cmd.none
                    )

                Nothing ->
                    ( model, Cmd.none )

        StepClicked stepId ->
            ( { model | selectedStepId = Just stepId }
            , Cmd.none
            )

        CanvasBackgroundClicked ->
            ( { model | selectedStepId = Nothing }
            , Cmd.none
            )

        StepMouseDown stepId clientX clientY ->
            case ( Dict.get stepId model.steps, DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY ) of
                ( Just step, Just canvasPos ) ->
                    let
                        offset =
                            { x = canvasPos.x - step.position.x
                            , y = canvasPos.y - step.position.y
                            }
                    in
                    ( { model
                        | dragging = Just (DraggingExistingStep stepId offset)
                        , selectedStepId = Just stepId
                      }
                    , Cmd.none
                    )

                _ ->
                    ( { model | selectedStepId = Just stepId }
                    , Cmd.none
                    )

        KeyDown key ->
            case ( key == "Delete" || key == "Backspace", model.selectedStepId ) of
                ( True, Just stepId ) ->
                    ( { model
                        | steps = Dict.remove stepId model.steps
                        , selectedStepId = Nothing
                      }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        GotCanvasBounds value ->
            case DesignerCanvas.decodeBounds value of
                Ok bounds ->
                    ( { model | canvasBounds = Just bounds }
                    , Cmd.none
                    )

                Err _ ->
                    ( model, Cmd.none )



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch
        [ Ports.receiveCanvasBounds GotCanvasBounds
        , if model.dragging /= Nothing then
            Sub.batch
                [ Browser.Events.onMouseMove
                    (Decode.map2 CanvasMouseMove
                        (Decode.field "clientX" Decode.float)
                        (Decode.field "clientY" Decode.float)
                    )
                , Browser.Events.onMouseUp
                    (Decode.succeed CanvasMouseUp)
                ]

          else
            Sub.none
        , Browser.Events.onKeyDown
            (Decode.field "key" Decode.string
                |> Decode.map KeyDown
            )
        ]



-- VIEW


view : Model -> Html Msg
view model =
    div [ class "flex flex-col", style "height" "calc(100vh - 8rem)" ]
        [ viewToolbar
        , div [ class "flex flex-1 overflow-hidden" ]
            [ viewPalette
            , viewCanvasArea model
            ]
        , viewStatusBar model
        ]


{-| ミニマルツールバー
-}
viewToolbar : Html Msg
viewToolbar =
    div [ class "flex items-center border-b border-secondary-200 bg-white px-4 py-2" ]
        [ h1 [ class "text-base font-semibold text-secondary-800" ]
            [ text "ワークフローデザイナー" ]
        ]


{-| ステップパレット

ドラッグ可能な 3 種類のステップ（開始・承認・終了）を表示する。

-}
viewPalette : Html Msg
viewPalette =
    div [ class "w-48 shrink-0 border-r border-secondary-200 bg-white p-4" ]
        [ h2 [ class "mb-3 text-xs font-semibold uppercase tracking-wider text-secondary-500" ]
            [ text "ステップ" ]
        , div [ class "space-y-2" ]
            [ viewPaletteItem Start
            , viewPaletteItem Approval
            , viewPaletteItem End
            ]
        ]


{-| パレットの各ステップアイテム
-}
viewPaletteItem : StepType -> Html Msg
viewPaletteItem stepType =
    let
        colors =
            DesignerCanvas.stepColors stepType
    in
    div
        [ class "flex cursor-grab items-center gap-2 rounded-lg border-2 px-3 py-2 text-sm font-medium select-none transition-shadow hover:shadow-md"
        , style "background-color" colors.fill
        , style "border-color" colors.stroke
        , style "color" colors.stroke
        , onMouseDown (PaletteMouseDown stepType)
        ]
        [ viewStepIcon stepType
        , span [] [ text (DesignerCanvas.defaultStepName stepType) ]
        ]


{-| ステップ種別に応じたミニアイコン
-}
viewStepIcon : StepType -> Html msg
viewStepIcon stepType =
    case stepType of
        Start ->
            svg
                [ SvgAttr.viewBox "0 0 16 16"
                , SvgAttr.class "h-4 w-4"
                ]
                [ Svg.polygon
                    [ SvgAttr.points "3,2 13,8 3,14"
                    , SvgAttr.fill "currentColor"
                    ]
                    []
                ]

        Approval ->
            svg
                [ SvgAttr.viewBox "0 0 16 16"
                , SvgAttr.fill "none"
                , SvgAttr.stroke "currentColor"
                , SvgAttr.strokeWidth "2"
                , SvgAttr.class "h-4 w-4"
                ]
                [ Svg.polyline [ SvgAttr.points "3,8 6,12 13,4" ] [] ]

        End ->
            svg
                [ SvgAttr.viewBox "0 0 16 16"
                , SvgAttr.fill "none"
                , SvgAttr.stroke "currentColor"
                , SvgAttr.strokeWidth "1.5"
                , SvgAttr.class "h-4 w-4"
                ]
                [ Svg.circle [ SvgAttr.cx "8", SvgAttr.cy "8", SvgAttr.r "6" ] []
                , Svg.circle [ SvgAttr.cx "8", SvgAttr.cy "8", SvgAttr.r "3", SvgAttr.fill "currentColor" ] []
                ]


{-| SVG キャンバスエリア

マウスイベントは Browser.Events（グローバル）で処理する。
背景クリック選択解除用のイベントのみ SVG 要素に設定。

-}
viewCanvasArea : Model -> Html Msg
viewCanvasArea model =
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
              viewCanvasBackground
            , viewGrid
            , viewSteps model
            , viewDragPreview model
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
viewSteps : Model -> Svg.Svg Msg
viewSteps model =
    Svg.g []
        (model.steps
            |> Dict.values
            |> List.map (viewStepNode model.selectedStepId)
        )


{-| 個別のステップノード描画
-}
viewStepNode : Maybe String -> StepNode -> Svg.Svg Msg
viewStepNode selectedStepId step =
    let
        colors =
            DesignerCanvas.stepColors step.stepType

        dim =
            DesignerCanvas.stepDimensions

        isSelected =
            selectedStepId == Just step.id

        strokeWidth =
            if isSelected then
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
            , SvgAttr.rx "8"
            , SvgAttr.fill colors.fill
            , SvgAttr.stroke colors.stroke
            , SvgAttr.strokeWidth strokeWidth
            ]
            []

        -- 選択ハイライト（外枠リング）
        , if isSelected then
            Svg.rect
                [ SvgAttr.x "-4"
                , SvgAttr.y "-4"
                , SvgAttr.width (String.fromFloat (dim.width + 8))
                , SvgAttr.height (String.fromFloat (dim.height + 8))
                , SvgAttr.rx "12"
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
            , SvgAttr.fontSize "13"
            , SvgAttr.fontWeight "500"
            , SvgAttr.class "pointer-events-none select-none"
            ]
            [ Svg.text step.name ]
        ]


{-| ドラッグ中のプレビュー表示

パレットからの新規配置時、マウス位置にゴーストステップを表示する。

-}
viewDragPreview : Model -> Svg.Svg Msg
viewDragPreview model =
    case model.dragging of
        Just (DraggingNewStep stepType pos) ->
            let
                colors =
                    DesignerCanvas.stepColors stepType

                dim =
                    DesignerCanvas.stepDimensions

                snappedX =
                    DesignerCanvas.snapToGrid pos.x

                snappedY =
                    DesignerCanvas.snapToGrid pos.y
            in
            Svg.g
                [ SvgAttr.transform
                    ("translate("
                        ++ String.fromFloat snappedX
                        ++ ","
                        ++ String.fromFloat snappedY
                        ++ ")"
                    )
                , SvgAttr.opacity "0.6"
                , SvgAttr.class "pointer-events-none"
                ]
                [ Svg.rect
                    [ SvgAttr.width (String.fromFloat dim.width)
                    , SvgAttr.height (String.fromFloat dim.height)
                    , SvgAttr.rx "8"
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
                    , SvgAttr.fontSize "13"
                    , SvgAttr.fontWeight "500"
                    ]
                    [ Svg.text (DesignerCanvas.defaultStepName stepType) ]
                ]

        _ ->
            Svg.text ""


{-| ステータスバー
-}
viewStatusBar : Model -> Html Msg
viewStatusBar model =
    div [ class "border-t border-secondary-200 bg-white px-4 py-1.5 text-xs text-secondary-500" ]
        [ text (String.fromInt (Dict.size model.steps) ++ " ステップ") ]
