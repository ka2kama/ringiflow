module Data.DesignerCanvasTest exposing (suite)

{-| DesignerCanvas データ型のテスト

ステップ型の文字列変換、グリッドスナップ、デフォルト名の検証。

-}

import Data.DesignerCanvas as DesignerCanvas exposing (StepType(..))
import Expect
import Json.Encode as Encode
import Test exposing (..)


suite : Test
suite =
    describe "DesignerCanvas"
        [ stepTypeToStringTests
        , snapToGridTests
        , defaultStepNameTests
        , stepColorsTests
        , stepDimensionsTests
        , clientToCanvasTests
        , generateStepIdTests
        , createStepFromDropTests
        , boundsDecoderTests
        ]



-- stepTypeToString


stepTypeToStringTests : Test
stepTypeToStringTests =
    describe "stepTypeToString"
        [ test "Start → \"start\"" <|
            \_ ->
                DesignerCanvas.stepTypeToString Start
                    |> Expect.equal "start"
        , test "Approval → \"approval\"" <|
            \_ ->
                DesignerCanvas.stepTypeToString Approval
                    |> Expect.equal "approval"
        , test "End → \"end\"" <|
            \_ ->
                DesignerCanvas.stepTypeToString End
                    |> Expect.equal "end"
        ]



-- snapToGrid


snapToGridTests : Test
snapToGridTests =
    describe "snapToGrid"
        [ test "0 はそのまま 0" <|
            \_ ->
                DesignerCanvas.snapToGrid 0
                    |> Expect.within (Expect.Absolute 0.001) 0
        , test "10 は 20 にスナップ（四捨五入）" <|
            \_ ->
                DesignerCanvas.snapToGrid 10
                    |> Expect.within (Expect.Absolute 0.001) 20
        , test "19 は 20 にスナップ" <|
            \_ ->
                DesignerCanvas.snapToGrid 19
                    |> Expect.within (Expect.Absolute 0.001) 20
        , test "20 はそのまま 20" <|
            \_ ->
                DesignerCanvas.snapToGrid 20
                    |> Expect.within (Expect.Absolute 0.001) 20
        , test "30 は 40 にスナップ" <|
            \_ ->
                DesignerCanvas.snapToGrid 30
                    |> Expect.within (Expect.Absolute 0.001) 40
        , test "9 は 0 にスナップ（切り捨て）" <|
            \_ ->
                DesignerCanvas.snapToGrid 9
                    |> Expect.within (Expect.Absolute 0.001) 0
        ]



-- defaultStepName


defaultStepNameTests : Test
defaultStepNameTests =
    describe "defaultStepName"
        [ test "Start → \"開始\"" <|
            \_ ->
                DesignerCanvas.defaultStepName Start
                    |> Expect.equal "開始"
        , test "Approval → \"承認\"" <|
            \_ ->
                DesignerCanvas.defaultStepName Approval
                    |> Expect.equal "承認"
        , test "End → \"終了\"" <|
            \_ ->
                DesignerCanvas.defaultStepName End
                    |> Expect.equal "終了"
        ]



-- stepColors


stepColorsTests : Test
stepColorsTests =
    describe "stepColors"
        [ test "Start は緑系（success）の色を返す" <|
            \_ ->
                let
                    colors =
                        DesignerCanvas.stepColors Start
                in
                Expect.all
                    [ \c -> c.fill |> Expect.equal "#d1fae5"
                    , \c -> c.stroke |> Expect.equal "#059669"
                    ]
                    colors
        , test "Approval は青系（primary）の色を返す" <|
            \_ ->
                let
                    colors =
                        DesignerCanvas.stepColors Approval
                in
                Expect.all
                    [ \c -> c.fill |> Expect.equal "#e0e7ff"
                    , \c -> c.stroke |> Expect.equal "#4f46e5"
                    ]
                    colors
        , test "End は灰系（secondary）の色を返す" <|
            \_ ->
                let
                    colors =
                        DesignerCanvas.stepColors End
                in
                Expect.all
                    [ \c -> c.fill |> Expect.equal "#f1f5f9"
                    , \c -> c.stroke |> Expect.equal "#475569"
                    ]
                    colors
        ]



-- stepDimensions


stepDimensionsTests : Test
stepDimensionsTests =
    describe "stepDimensions"
        [ test "幅 120, 高さ 60 を返す" <|
            \_ ->
                let
                    dim =
                        DesignerCanvas.stepDimensions
                in
                Expect.all
                    [ \d -> d.width |> Expect.within (Expect.Absolute 0.001) 120
                    , \d -> d.height |> Expect.within (Expect.Absolute 0.001) 60
                    ]
                    dim
        ]



-- clientToCanvas


clientToCanvasTests : Test
clientToCanvasTests =
    describe "clientToCanvas"
        [ test "マウス座標を SVG 座標に正しく変換する" <|
            \_ ->
                let
                    bounds =
                        { x = 100, y = 50, width = 600, height = 400 }

                    -- clientX=400 → (400-100)/600 * 1200 = 600
                    -- clientY=250 → (250-50)/400 * 800 = 400
                    result =
                        DesignerCanvas.clientToCanvas (Just bounds) 400 250
                in
                case result of
                    Just pos ->
                        Expect.all
                            [ \p -> p.x |> Expect.within (Expect.Absolute 0.1) 600
                            , \p -> p.y |> Expect.within (Expect.Absolute 0.1) 400
                            ]
                            pos

                    Nothing ->
                        Expect.fail "Expected Just Position, got Nothing"
        , test "Bounds 未取得時（Nothing）に Nothing を返す" <|
            \_ ->
                DesignerCanvas.clientToCanvas Nothing 400 250
                    |> Expect.equal Nothing
        ]



-- generateStepId


generateStepIdTests : Test
generateStepIdTests =
    describe "generateStepId"
        [ test "stepType と番号から一意な ID を生成する" <|
            \_ ->
                Expect.all
                    [ \_ -> DesignerCanvas.generateStepId Start 1 |> Expect.equal "start_1"
                    , \_ -> DesignerCanvas.generateStepId Approval 2 |> Expect.equal "approval_2"
                    , \_ -> DesignerCanvas.generateStepId End 3 |> Expect.equal "end_3"
                    ]
                    ()
        ]



-- createStepFromDrop


createStepFromDropTests : Test
createStepFromDropTests =
    describe "createStepFromDrop"
        [ test "パレットドロップからグリッドスナップされた StepNode を生成する" <|
            \_ ->
                let
                    step =
                        DesignerCanvas.createStepFromDrop Start 1 { x = 155, y = 83 }
                in
                Expect.all
                    [ \s -> s.id |> Expect.equal "start_1"
                    , \s -> s.stepType |> Expect.equal Start
                    , \s -> s.name |> Expect.equal "開始"
                    , \s -> s.position.x |> Expect.within (Expect.Absolute 0.001) 160
                    , \s -> s.position.y |> Expect.within (Expect.Absolute 0.001) 80
                    ]
                    step
        ]



-- boundsDecoder


boundsDecoderTests : Test
boundsDecoderTests =
    describe "boundsDecoder"
        [ test "{ x, y, width, height } を Bounds にデコードする" <|
            \_ ->
                let
                    json =
                        Encode.object
                            [ ( "x", Encode.float 100 )
                            , ( "y", Encode.float 50 )
                            , ( "width", Encode.float 800 )
                            , ( "height", Encode.float 600 )
                            ]

                    result =
                        DesignerCanvas.decodeBounds json
                in
                case result of
                    Ok bounds ->
                        Expect.all
                            [ \b -> b.x |> Expect.within (Expect.Absolute 0.001) 100
                            , \b -> b.y |> Expect.within (Expect.Absolute 0.001) 50
                            , \b -> b.width |> Expect.within (Expect.Absolute 0.001) 800
                            , \b -> b.height |> Expect.within (Expect.Absolute 0.001) 600
                            ]
                            bounds

                    Err _ ->
                        Expect.fail "Expected Ok Bounds, got Err"
        ]
