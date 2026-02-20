module Data.DesignerCanvasTest exposing (suite)

{-| DesignerCanvas データ型のテスト

ステップ型の文字列変換、グリッドスナップ、デフォルト名の検証。
定義のエンコード/デコード（JSON ↔ Dict/List 変換）の検証。

-}

import Data.DesignerCanvas as DesignerCanvas exposing (StepType(..))
import Dict
import Expect
import Json.Decode as Decode
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
        , encodeDefinitionTests
        , loadStepsFromDefinitionTests
        , loadTransitionsFromDefinitionTests
        , stepOutputPortPositionTests
        , stepInputPortPositionTests
        , stepContainsPointTests
        , autoTriggerTests
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
                    , \s -> s.assignee |> Expect.equal Nothing
                    , \s -> s.endStatus |> Expect.equal Nothing
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



-- encodeDefinition


encodeDefinitionTests : Test
encodeDefinitionTests =
    describe "encodeDefinition"
        [ test "steps と transitions から正しい JSON を生成する" <|
            \_ ->
                let
                    steps =
                        Dict.fromList
                            [ ( "start_1"
                              , { id = "start_1"
                                , stepType = Start
                                , name = "開始"
                                , position = { x = 100, y = 100 }
                                , assignee = Nothing
                                , endStatus = Nothing
                                }
                              )
                            , ( "approval_1"
                              , { id = "approval_1"
                                , stepType = Approval
                                , name = "承認"
                                , position = { x = 300, y = 100 }
                                , assignee = Just { type_ = "user" }
                                , endStatus = Nothing
                                }
                              )
                            , ( "end_1"
                              , { id = "end_1"
                                , stepType = End
                                , name = "承認完了"
                                , position = { x = 500, y = 100 }
                                , assignee = Nothing
                                , endStatus = Just "approved"
                                }
                              )
                            ]

                    transitions =
                        [ { from = "start_1", to = "approval_1", trigger = Nothing }
                        , { from = "approval_1", to = "end_1", trigger = Just "approve" }
                        ]

                    encoded =
                        DesignerCanvas.encodeDefinition steps transitions

                    -- steps 配列の要素数
                    stepsCount =
                        Decode.decodeValue
                            (Decode.field "steps" (Decode.list Decode.value)
                                |> Decode.map List.length
                            )
                            encoded

                    -- transitions 配列の要素数
                    transitionsCount =
                        Decode.decodeValue
                            (Decode.field "transitions" (Decode.list Decode.value)
                                |> Decode.map List.length
                            )
                            encoded

                    -- steps 内の approval ステップに assignee が含まれるか確認
                    approvalHasAssignee =
                        Decode.decodeValue
                            (Decode.field "steps"
                                (Decode.list
                                    (Decode.map2 Tuple.pair
                                        (Decode.field "id" Decode.string)
                                        (Decode.maybe (Decode.field "assignee" Decode.value))
                                    )
                                )
                                |> Decode.map
                                    (List.filterMap
                                        (\( id, maybeAssignee ) ->
                                            if id == "approval_1" then
                                                Just (maybeAssignee /= Nothing)

                                            else
                                                Nothing
                                        )
                                    )
                            )
                            encoded

                    -- transitions 内の trigger フィールド確認
                    triggerValues =
                        Decode.decodeValue
                            (Decode.field "transitions"
                                (Decode.list
                                    (Decode.maybe (Decode.field "trigger" Decode.string))
                                )
                            )
                            encoded
                in
                Expect.all
                    [ \_ -> stepsCount |> Expect.equal (Ok 3)
                    , \_ -> transitionsCount |> Expect.equal (Ok 2)
                    , \_ -> approvalHasAssignee |> Expect.equal (Ok [ True ])
                    , \_ -> triggerValues |> Expect.equal (Ok [ Nothing, Just "approve" ])
                    ]
                    ()
        ]



-- loadStepsFromDefinition


loadStepsFromDefinitionTests : Test
loadStepsFromDefinitionTests =
    describe "loadStepsFromDefinition"
        [ test "position あり JSON から StepNode Dict を生成する" <|
            \_ ->
                let
                    json =
                        Encode.object
                            [ ( "steps"
                              , Encode.list identity
                                    [ Encode.object
                                        [ ( "id", Encode.string "start_1" )
                                        , ( "type", Encode.string "start" )
                                        , ( "name", Encode.string "開始" )
                                        , ( "position"
                                          , Encode.object
                                                [ ( "x", Encode.float 100 )
                                                , ( "y", Encode.float 200 )
                                                ]
                                          )
                                        ]
                                    , Encode.object
                                        [ ( "id", Encode.string "approval_1" )
                                        , ( "type", Encode.string "approval" )
                                        , ( "name", Encode.string "部長承認" )
                                        , ( "assignee"
                                          , Encode.object [ ( "type", Encode.string "user" ) ]
                                          )
                                        , ( "position"
                                          , Encode.object
                                                [ ( "x", Encode.float 300 )
                                                , ( "y", Encode.float 200 )
                                                ]
                                          )
                                        ]
                                    , Encode.object
                                        [ ( "id", Encode.string "end_1" )
                                        , ( "type", Encode.string "end" )
                                        , ( "name", Encode.string "承認完了" )
                                        , ( "status", Encode.string "approved" )
                                        , ( "position"
                                          , Encode.object
                                                [ ( "x", Encode.float 500 )
                                                , ( "y", Encode.float 200 )
                                                ]
                                          )
                                        ]
                                    ]
                              )
                            , ( "transitions", Encode.list identity [] )
                            ]
                in
                case DesignerCanvas.loadStepsFromDefinition json of
                    Ok dict ->
                        Expect.all
                            [ \_ -> Dict.size dict |> Expect.equal 3
                            , \_ ->
                                Dict.get "start_1" dict
                                    |> Maybe.map .stepType
                                    |> Expect.equal (Just Start)
                            , \_ ->
                                Dict.get "start_1" dict
                                    |> Maybe.map .name
                                    |> Expect.equal (Just "開始")
                            , \_ ->
                                Dict.get "start_1" dict
                                    |> Maybe.map (\s -> ( s.position.x, s.position.y ))
                                    |> Expect.equal (Just ( 100, 200 ))
                            , \_ ->
                                Dict.get "approval_1" dict
                                    |> Maybe.map (\s -> s.assignee |> Maybe.map .type_)
                                    |> Expect.equal (Just (Just "user"))
                            , \_ ->
                                Dict.get "end_1" dict
                                    |> Maybe.map .endStatus
                                    |> Expect.equal (Just (Just "approved"))
                            ]
                            ()

                    Err err ->
                        Expect.fail ("Expected Ok, got Err: " ++ Decode.errorToString err)
        , test "position なし JSON から自動配置で StepNode Dict を生成する" <|
            \_ ->
                let
                    json =
                        Encode.object
                            [ ( "steps"
                              , Encode.list identity
                                    [ Encode.object
                                        [ ( "id", Encode.string "start" )
                                        , ( "type", Encode.string "start" )
                                        , ( "name", Encode.string "開始" )
                                        ]
                                    , Encode.object
                                        [ ( "id", Encode.string "approval" )
                                        , ( "type", Encode.string "approval" )
                                        , ( "name", Encode.string "承認" )
                                        ]
                                    , Encode.object
                                        [ ( "id", Encode.string "end" )
                                        , ( "type", Encode.string "end" )
                                        , ( "name", Encode.string "終了" )
                                        ]
                                    ]
                              )
                            , ( "transitions", Encode.list identity [] )
                            ]
                in
                case DesignerCanvas.loadStepsFromDefinition json of
                    Ok dict ->
                        let
                            -- 自動配置: 縦一列、等間隔
                            -- x = viewBoxWidth / 2 - stepWidth / 2 = 540
                            -- y = 60 + index * 100
                            positions =
                                Dict.toList dict
                                    |> List.sortBy (\( _, s ) -> s.position.y)
                                    |> List.map (\( _, s ) -> ( s.position.x, s.position.y ))
                        in
                        Expect.all
                            [ \_ -> Dict.size dict |> Expect.equal 3
                            , \_ ->
                                -- すべての x 座標が同じ（縦一列）
                                positions
                                    |> List.map Tuple.first
                                    |> List.all (\x -> x == 540)
                                    |> Expect.equal True
                            , \_ ->
                                -- y 座標が等間隔で増加
                                positions
                                    |> List.map Tuple.second
                                    |> Expect.equal [ 60, 160, 260 ]
                            ]
                            ()

                    Err err ->
                        Expect.fail ("Expected Ok, got Err: " ++ Decode.errorToString err)
        ]



-- loadTransitionsFromDefinition


loadTransitionsFromDefinitionTests : Test
loadTransitionsFromDefinitionTests =
    describe "loadTransitionsFromDefinition"
        [ test "transitions を正しくデコードする" <|
            \_ ->
                let
                    json =
                        Encode.object
                            [ ( "steps", Encode.list identity [] )
                            , ( "transitions"
                              , Encode.list identity
                                    [ Encode.object
                                        [ ( "from", Encode.string "start_1" )
                                        , ( "to", Encode.string "approval_1" )
                                        ]
                                    , Encode.object
                                        [ ( "from", Encode.string "approval_1" )
                                        , ( "to", Encode.string "end_1" )
                                        , ( "trigger", Encode.string "approve" )
                                        ]
                                    ]
                              )
                            ]
                in
                case DesignerCanvas.loadTransitionsFromDefinition json of
                    Ok transitions ->
                        Expect.all
                            [ \_ -> List.length transitions |> Expect.equal 2
                            , \_ ->
                                List.head transitions
                                    |> Maybe.map (\t -> ( t.from, t.to, t.trigger ))
                                    |> Expect.equal (Just ( "start_1", "approval_1", Nothing ))
                            , \_ ->
                                transitions
                                    |> List.drop 1
                                    |> List.head
                                    |> Maybe.map (\t -> ( t.from, t.to, t.trigger ))
                                    |> Expect.equal (Just ( "approval_1", "end_1", Just "approve" ))
                            ]
                            ()

                    Err err ->
                        Expect.fail ("Expected Ok, got Err: " ++ Decode.errorToString err)
        ]



-- stepOutputPortPosition


stepOutputPortPositionTests : Test
stepOutputPortPositionTests =
    describe "stepOutputPortPosition"
        [ test "ステップ右端中央の座標を返す" <|
            \_ ->
                let
                    step =
                        makeStep "step_1" Start { x = 100, y = 200 }

                    pos =
                        DesignerCanvas.stepOutputPortPosition step
                in
                -- stepDimensions: width=120, height=60
                -- 右端中央: (100+120, 200+60/2) = (220, 230)
                Expect.all
                    [ \p -> p.x |> Expect.within (Expect.Absolute 0.1) 220
                    , \p -> p.y |> Expect.within (Expect.Absolute 0.1) 230
                    ]
                    pos
        ]



-- stepInputPortPosition


stepInputPortPositionTests : Test
stepInputPortPositionTests =
    describe "stepInputPortPosition"
        [ test "ステップ左端中央の座標を返す" <|
            \_ ->
                let
                    step =
                        makeStep "step_1" Approval { x = 300, y = 200 }

                    pos =
                        DesignerCanvas.stepInputPortPosition step
                in
                -- 左端中央: (300, 200+60/2) = (300, 230)
                Expect.all
                    [ \p -> p.x |> Expect.within (Expect.Absolute 0.1) 300
                    , \p -> p.y |> Expect.within (Expect.Absolute 0.1) 230
                    ]
                    pos
        ]



-- stepContainsPoint


stepContainsPointTests : Test
stepContainsPointTests =
    describe "stepContainsPoint"
        [ test "矩形内の座標で True を返す" <|
            \_ ->
                let
                    step =
                        makeStep "step_1" Start { x = 100, y = 200 }
                in
                -- 矩形: x=[100,220], y=[200,260]
                DesignerCanvas.stepContainsPoint { x = 150, y = 230 } step
                    |> Expect.equal True
        , test "矩形の境界で True を返す" <|
            \_ ->
                let
                    step =
                        makeStep "step_1" Start { x = 100, y = 200 }
                in
                Expect.all
                    [ \_ ->
                        DesignerCanvas.stepContainsPoint { x = 100, y = 200 } step
                            |> Expect.equal True
                    , \_ ->
                        DesignerCanvas.stepContainsPoint { x = 220, y = 260 } step
                            |> Expect.equal True
                    ]
                    ()
        , test "矩形外の座標で False を返す" <|
            \_ ->
                let
                    step =
                        makeStep "step_1" Start { x = 100, y = 200 }
                in
                Expect.all
                    [ \_ ->
                        DesignerCanvas.stepContainsPoint { x = 99, y = 230 } step
                            |> Expect.equal False
                    , \_ ->
                        DesignerCanvas.stepContainsPoint { x = 221, y = 230 } step
                            |> Expect.equal False
                    , \_ ->
                        DesignerCanvas.stepContainsPoint { x = 150, y = 199 } step
                            |> Expect.equal False
                    ]
                    ()
        ]



-- autoTrigger


autoTriggerTests : Test
autoTriggerTests =
    describe "autoTrigger"
        [ test "Approval から approve なし → Just \"approve\"" <|
            \_ ->
                DesignerCanvas.autoTrigger Approval "approval_1" []
                    |> Expect.equal (Just "approve")
        , test "Approval で approve あり reject なし → Just \"reject\"" <|
            \_ ->
                let
                    existingTransitions =
                        [ { from = "approval_1", to = "end_1", trigger = Just "approve" } ]
                in
                DesignerCanvas.autoTrigger Approval "approval_1" existingTransitions
                    |> Expect.equal (Just "reject")
        , test "Approval で approve/reject 両方あり → Nothing" <|
            \_ ->
                let
                    existingTransitions =
                        [ { from = "approval_1", to = "end_1", trigger = Just "approve" }
                        , { from = "approval_1", to = "end_2", trigger = Just "reject" }
                        ]
                in
                DesignerCanvas.autoTrigger Approval "approval_1" existingTransitions
                    |> Expect.equal Nothing
        , test "Start → Nothing" <|
            \_ ->
                DesignerCanvas.autoTrigger Start "start_1" []
                    |> Expect.equal Nothing
        , test "End → Nothing" <|
            \_ ->
                DesignerCanvas.autoTrigger End "end_1" []
                    |> Expect.equal Nothing
        ]



-- ヘルパー


makeStep : String -> StepType -> DesignerCanvas.Position -> DesignerCanvas.StepNode
makeStep id stepType position =
    { id = id
    , stepType = stepType
    , name = DesignerCanvas.defaultStepName stepType
    , position = position
    , assignee = Nothing
    , endStatus = Nothing
    }
