module Page.WorkflowDefinition.Designer.Palette exposing (viewPalette)

{-| ステップパレット

ドラッグ可能な 3 種類のステップ（開始・承認・終了）を表示する。

-}

import Data.DesignerCanvas as DesignerCanvas exposing (StepType(..))
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onMouseDown)
import Page.WorkflowDefinition.Designer.Types exposing (Msg(..))
import Svg exposing (svg)
import Svg.Attributes as SvgAttr


{-| ステップパレット
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
