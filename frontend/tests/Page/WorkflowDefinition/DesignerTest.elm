module Page.WorkflowDefinition.DesignerTest exposing (suite)

{-| Designer ページの update ロジックテスト

ドラッグ&ドロップ、選択、移動、削除の状態遷移を検証する。

-}

import Data.DesignerCanvas exposing (DraggingState(..), StepType(..))
import Dict
import Expect
import Page.WorkflowDefinition.Designer as Designer exposing (Model, Msg(..))
import Shared exposing (Shared)
import Test exposing (..)


suite : Test
suite =
    describe "Designer"
        [ paletteMouseDownTests
        , canvasMouseUpTests
        , canvasMouseMoveTests
        , stepClickedTests
        , canvasBackgroundClickedTests
        , stepMouseDownTests
        , keyDownTests
        ]



-- テスト用ヘルパー


testShared : Shared
testShared =
    Shared.init { apiBaseUrl = "", timezoneOffsetMinutes = 540 }


initModel : Model
initModel =
    let
        ( model, _ ) =
            Designer.init testShared
    in
    model


{-| Bounds を設定したモデル（座標変換が機能する状態）
-}
modelWithBounds : Model
modelWithBounds =
    { initModel
        | canvasBounds = Just { x = 0, y = 0, width = 1200, height = 800 }
    }


{-| ステップが1つ配置済みのモデル
-}
modelWithOneStep : Model
modelWithOneStep =
    let
        step =
            { id = "approval_1"
            , stepType = Approval
            , name = "承認"
            , position = { x = 200, y = 100 }
            }
    in
    { modelWithBounds
        | steps = Dict.singleton "approval_1" step
        , nextStepNumber = 2
    }



-- PaletteMouseDown


paletteMouseDownTests : Test
paletteMouseDownTests =
    describe "PaletteMouseDown"
        [ test "dragging が DraggingNewStep に遷移する" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (PaletteMouseDown Start) modelWithBounds
                in
                case newModel.dragging of
                    Just (DraggingNewStep Start _) ->
                        Expect.pass

                    _ ->
                        Expect.fail "Expected DraggingNewStep Start"
        ]



-- CanvasMouseUp


canvasMouseUpTests : Test
canvasMouseUpTests =
    describe "CanvasMouseUp"
        [ test "DraggingNewStep 時に新しい StepNode が steps に追加される" <|
            \_ ->
                let
                    draggingModel =
                        { modelWithBounds
                            | dragging = Just (DraggingNewStep Approval { x = 200, y = 100 })
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasMouseUp draggingModel
                in
                Expect.all
                    [ \m -> Dict.size m.steps |> Expect.equal 1
                    , \m -> m.dragging |> Expect.equal Nothing
                    , \m -> m.nextStepNumber |> Expect.equal 2
                    ]
                    newModel
        , test "dragging が Nothing にリセットされる" <|
            \_ ->
                let
                    draggingModel =
                        { modelWithBounds
                            | dragging = Just (DraggingNewStep Start { x = 100, y = 100 })
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasMouseUp draggingModel
                in
                newModel.dragging |> Expect.equal Nothing
        , test "DraggingExistingStep 時に dragging が Nothing になり位置が確定する" <|
            \_ ->
                let
                    draggingModel =
                        { modelWithOneStep
                            | dragging = Just (DraggingExistingStep "approval_1" { x = 10, y = 10 })
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasMouseUp draggingModel
                in
                Expect.all
                    [ \m -> m.dragging |> Expect.equal Nothing
                    , \m -> Dict.size m.steps |> Expect.equal 1
                    ]
                    newModel
        ]



-- CanvasMouseMove


canvasMouseMoveTests : Test
canvasMouseMoveTests =
    describe "CanvasMouseMove"
        [ test "DraggingNewStep の位置が更新される" <|
            \_ ->
                let
                    draggingModel =
                        { modelWithBounds
                            | dragging = Just (DraggingNewStep Start { x = 100, y = 100 })
                        }

                    ( newModel, _ ) =
                        Designer.update (CanvasMouseMove 300 200) draggingModel
                in
                case newModel.dragging of
                    Just (DraggingNewStep Start pos) ->
                        Expect.all
                            [ \p -> p.x |> Expect.within (Expect.Absolute 0.1) 300
                            , \p -> p.y |> Expect.within (Expect.Absolute 0.1) 200
                            ]
                            pos

                    _ ->
                        Expect.fail "Expected DraggingNewStep with updated position"
        , test "DraggingExistingStep 時にステップ位置がグリッドスナップで更新される" <|
            \_ ->
                let
                    draggingModel =
                        { modelWithOneStep
                            | dragging = Just (DraggingExistingStep "approval_1" { x = 10, y = 10 })
                        }

                    ( newModel, _ ) =
                        Designer.update (CanvasMouseMove 400 300) draggingModel
                in
                case Dict.get "approval_1" newModel.steps of
                    Just step ->
                        -- clientX=400 → canvas 400, offset 10 → 390 → snap to 400
                        -- clientY=300 → canvas 300, offset 10 → 290 → snap to 300
                        Expect.all
                            [ \s -> s.position.x |> Expect.within (Expect.Absolute 0.1) 400
                            , \s -> s.position.y |> Expect.within (Expect.Absolute 0.1) 300
                            ]
                            step

                    Nothing ->
                        Expect.fail "Step not found"
        ]



-- StepClicked


stepClickedTests : Test
stepClickedTests =
    describe "StepClicked"
        [ test "selectedStepId が設定される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (StepClicked "approval_1") modelWithOneStep
                in
                newModel.selectedStepId |> Expect.equal (Just "approval_1")
        ]



-- CanvasBackgroundClicked


canvasBackgroundClickedTests : Test
canvasBackgroundClickedTests =
    describe "CanvasBackgroundClicked"
        [ test "selectedStepId が Nothing になる" <|
            \_ ->
                let
                    selectedModel =
                        { modelWithOneStep | selectedStepId = Just "approval_1" }

                    ( newModel, _ ) =
                        Designer.update CanvasBackgroundClicked selectedModel
                in
                newModel.selectedStepId |> Expect.equal Nothing
        ]



-- StepMouseDown


stepMouseDownTests : Test
stepMouseDownTests =
    describe "StepMouseDown"
        [ test "dragging が DraggingExistingStep に遷移する（clientX/clientY → offset を計算）" <|
            \_ ->
                let
                    -- ステップ位置は (200, 100)、clientX=230, clientY=120
                    -- canvasBounds は (0, 0, 1200, 800) なので 1:1 変換
                    -- offset = (230-200, 120-100) = (30, 20)
                    ( newModel, _ ) =
                        Designer.update (StepMouseDown "approval_1" 230 120) modelWithOneStep
                in
                case newModel.dragging of
                    Just (DraggingExistingStep stepId offset) ->
                        Expect.all
                            [ \_ -> stepId |> Expect.equal "approval_1"
                            , \_ -> offset.x |> Expect.within (Expect.Absolute 0.1) 30
                            , \_ -> offset.y |> Expect.within (Expect.Absolute 0.1) 20
                            ]
                            ()

                    _ ->
                        Expect.fail "Expected DraggingExistingStep"
        , test "selectedStepId も設定される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (StepMouseDown "approval_1" 230 120) modelWithOneStep
                in
                newModel.selectedStepId |> Expect.equal (Just "approval_1")
        ]



-- KeyDown


keyDownTests : Test
keyDownTests =
    describe "KeyDown"
        [ test "Delete で選択中のステップが削除される" <|
            \_ ->
                let
                    selectedModel =
                        { modelWithOneStep | selectedStepId = Just "approval_1" }

                    ( newModel, _ ) =
                        Designer.update (KeyDown "Delete") selectedModel
                in
                Expect.all
                    [ \m -> Dict.size m.steps |> Expect.equal 0
                    , \m -> m.selectedStepId |> Expect.equal Nothing
                    ]
                    newModel
        , test "Delete で選択中ステップがない場合は何も起きない" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (KeyDown "Delete") modelWithOneStep
                in
                Dict.size newModel.steps |> Expect.equal 1
        , test "Backspace でも選択中ステップが削除される" <|
            \_ ->
                let
                    selectedModel =
                        { modelWithOneStep | selectedStepId = Just "approval_1" }

                    ( newModel, _ ) =
                        Designer.update (KeyDown "Backspace") selectedModel
                in
                Expect.all
                    [ \m -> Dict.size m.steps |> Expect.equal 0
                    , \m -> m.selectedStepId |> Expect.equal Nothing
                    ]
                    newModel
        ]
