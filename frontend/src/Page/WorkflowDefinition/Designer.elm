module Page.WorkflowDefinition.Designer exposing (init, isDirty, subscriptions, update, updateShared, view)

{-| ワークフローデザイナー画面

SVG キャンバス上にワークフローのステップを配置・操作するビジュアルエディタ。
ADR-053 で決定した SVG + Elm 直接レンダリング方式に基づく。

Model は型安全ステートマシンで管理する（ADR-054）。
Loading 中はキャンバス関連フィールドが型レベルで存在しないため、
不正な状態（Loading 中のキャンバス操作）を表現不可能にしている。

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Browser.Events
import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Component.ErrorState as ErrorState
import Component.FormField as FormField
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.DesignerCanvas as DesignerCanvas exposing (Bounds, DraggingState(..), ReconnectEnd(..), StepNode, StepType(..), Transition, viewBoxHeight, viewBoxWidth)
import Data.WorkflowDefinition as WorkflowDefinition exposing (ValidationError, ValidationResult, WorkflowDefinition)
import Dict exposing (Dict)
import Form.DirtyState as DirtyState
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onMouseDown)
import Json.Decode as Decode
import Json.Encode as Encode
import List.Extra
import Maybe.Extra
import Page.WorkflowDefinition.Designer.Types as Types exposing (CanvasState, Model, Msg(..), PageState(..), canvasElementId)
import Page.WorkflowDefinition.Designer.Update as DesignerUpdate
import Ports
import Shared exposing (Shared)
import Svg exposing (svg)
import Svg.Attributes as SvgAttr
import Svg.Events


init : Shared -> String -> ( Model, Cmd Msg )
init shared definitionId =
    ( { shared = shared
      , definitionId = definitionId
      , state = Loading
      }
    , WorkflowDefinitionApi.getDefinition
        { config = Shared.toRequestConfig shared
        , id = definitionId
        , toMsg = GotDefinition
        }
    )


isDirty : Model -> Bool
isDirty model =
    case model.state of
        Loaded canvas ->
            DirtyState.isDirty canvas

        _ ->
            False


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotDefinition result ->
            DesignerUpdate.handleGotDefinition result model

        _ ->
            case model.state of
                Loaded canvas ->
                    let
                        ( newCanvas, cmd ) =
                            DesignerUpdate.updateLoaded msg model.shared model.definitionId canvas
                    in
                    ( { model | state = Loaded newCanvas }, cmd )

                _ ->
                    ( model, Cmd.none )



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch
        [ Ports.receiveCanvasBounds GotCanvasBounds
        , case model.state of
            Loaded canvas ->
                Sub.batch
                    [ if canvas.dragging /= Nothing then
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
                            |> Decode.andThen
                                (\key ->
                                    Decode.at [ "target", "tagName" ] Decode.string
                                        |> Decode.andThen
                                            (\tagName ->
                                                if List.member tagName [ "INPUT", "TEXTAREA", "SELECT" ] then
                                                    Decode.fail "ignore input element"

                                                else
                                                    Decode.succeed (KeyDown key)
                                            )
                                )
                        )
                    ]

            _ ->
                Sub.none
        ]



-- VIEW


view : Model -> Html Msg
view model =
    case model.state of
        Loading ->
            div [ class "flex items-center justify-center", style "height" "calc(100vh - 8rem)" ]
                [ LoadingSpinner.view ]

        Failed err ->
            div [ class "flex items-center justify-center", style "height" "calc(100vh - 8rem)" ]
                [ ErrorState.viewSimple (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err) ]

        Loaded canvas ->
            viewLoaded canvas


{-| Loaded 状態の view
-}
viewLoaded : CanvasState -> Html Msg
viewLoaded canvas =
    div [ class "flex flex-col", style "height" "calc(100vh - 8rem)" ]
        [ viewToolbar canvas
        , viewMessages canvas
        , div [ class "flex flex-1 overflow-hidden" ]
            [ viewPalette
            , viewCanvasArea canvas
            , viewPropertyPanel canvas
            ]
        , viewValidationPanel canvas
        , viewStatusBar canvas
        , viewPublishDialog canvas
        ]


{-| ツールバー（定義名 + バリデーション・保存・公開ボタン）
-}
viewToolbar : CanvasState -> Html Msg
viewToolbar canvas =
    div [ class "flex items-center gap-4 border-b border-secondary-200 bg-white px-4 py-2" ]
        [ h1 [ class "shrink-0 text-base font-semibold text-secondary-800" ]
            [ text "ワークフローデザイナー" ]
        , input
            [ type_ "text"
            , class "min-w-0 flex-1 rounded border border-secondary-300 px-3 py-1 text-sm focus:border-primary-500 focus:ring-1 focus:ring-primary-500"
            , value canvas.name
            , Html.Events.onInput UpdateDefinitionName
            , placeholder "定義名を入力"
            ]
            []
        , div [ class "flex shrink-0 gap-2" ]
            [ Button.view
                { variant = Button.Outline
                , disabled = canvas.isValidating
                , onClick = ValidateClicked
                }
                [ text
                    (if canvas.isValidating then
                        "検証中..."

                     else
                        "検証"
                    )
                ]
            , Button.view
                { variant = Button.Primary
                , disabled = canvas.isSaving || not canvas.isDirty_
                , onClick = SaveClicked
                }
                [ text
                    (if canvas.isSaving then
                        "保存中..."

                     else
                        "保存"
                    )
                ]
            , Button.view
                { variant = Button.Success
                , disabled = canvas.isPublishing || canvas.isSaving || canvas.isValidating
                , onClick = PublishClicked
                }
                [ text
                    (if canvas.isPublishing then
                        "公開中..."

                     else
                        "公開"
                    )
                ]
            ]
        ]


{-| 成功・エラーメッセージ表示
-}
viewMessages : CanvasState -> Html Msg
viewMessages canvas =
    MessageAlert.view
        { onDismiss = DismissMessage
        , successMessage = canvas.successMessage
        , errorMessage = canvas.errorMessage
        }


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


{-| プロパティパネル（右サイドバー）

選択中の要素のプロパティを表示・編集するパネル。
接続線選択時は接続情報（読み取り専用）と削除 UI を表示し、
ステップ選択時はステップ種別に応じたフィールドを表示する:

  - Start: ステップ名
  - Approval: ステップ名 + 承認者指定方式（読み取り専用）
  - End: ステップ名 + 終了ステータス

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


{-| バリデーション結果パネル

キャンバス下部に表示。valid なら緑、invalid ならエラー一覧。

-}
viewValidationPanel : CanvasState -> Html Msg
viewValidationPanel canvas =
    case canvas.validationResult of
        Just result ->
            if result.valid then
                div [ class "border-t border-success-200 bg-success-50 px-4 py-2 text-sm text-success-700" ]
                    [ text "フロー定義は有効です" ]

            else
                div [ class "border-t border-error-200 bg-error-50 px-4 py-2" ]
                    [ p [ class "text-sm font-medium text-error-700" ]
                        [ text ("バリデーションエラー（" ++ String.fromInt (List.length result.errors) ++ " 件）") ]
                    , ul [ class "mt-1 space-y-1" ]
                        (List.map viewValidationError result.errors)
                    ]

        Nothing ->
            text ""


{-| 個別のバリデーションエラー行

stepId がある場合、クリックで該当ステップを選択する。

-}
viewValidationError : ValidationError -> Html Msg
viewValidationError error =
    li
        (class "text-sm text-error-600"
            :: (case error.stepId of
                    Just stepId ->
                        [ class "cursor-pointer hover:underline"
                        , Html.Events.onClick (StepClicked stepId)
                        ]

                    Nothing ->
                        []
               )
        )
        [ text error.message ]


{-| ステータスバー
-}
viewStatusBar : CanvasState -> Html Msg
viewStatusBar canvas =
    div [ class "border-t border-secondary-200 bg-white px-4 py-1.5 text-xs text-secondary-500" ]
        [ text
            (String.fromInt (Dict.size canvas.steps)
                ++ " ステップ / "
                ++ String.fromInt (List.length canvas.transitions)
                ++ " 接続"
            )
        ]


{-| 公開確認ダイアログ
-}
viewPublishDialog : CanvasState -> Html Msg
viewPublishDialog canvas =
    if canvas.pendingPublish then
        ConfirmDialog.view
            { title = "ワークフロー定義を公開"
            , message = "「" ++ canvas.name ++ "」を公開しますか？公開後はユーザーが申請に使用できるようになります。"
            , confirmLabel = "公開する"
            , cancelLabel = "キャンセル"
            , onConfirm = ConfirmPublish
            , onCancel = CancelPublish
            , actionStyle = ConfirmDialog.Positive
            }

    else
        text ""
