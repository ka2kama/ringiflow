module Page.WorkflowDefinition.DesignerTest exposing (suite)

{-| Designer ページの update ロジックテスト

ドラッグ&ドロップ、選択、移動、削除の状態遷移を検証する。

-}

import Api exposing (ApiError(..))
import Data.DesignerCanvas as DesignerCanvas exposing (DraggingState(..), ReconnectEnd(..), StepType(..))
import Data.WorkflowDefinition exposing (ValidationResult, WorkflowDefinition)
import Dict
import Expect
import Json.Encode as Encode
import Page.WorkflowDefinition.Designer as Designer exposing (CanvasState, Model, Msg(..), PageState(..))
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
        , connectionPortMouseDownTests
        , transitionClickedTests
        , connectionKeyDownTests
        , reconnectionTests
        , propertyPanelTests
        , dragBoundsTests
        , deleteSelectedStepTests
        , apiIntegrationTests
        , validationAndPublishTests
        ]



-- テスト用ヘルパー


testShared : Shared
testShared =
    Shared.init { apiBaseUrl = "", timezoneOffsetMinutes = 540 }


{-| Loading 状態の初期モデル
-}
initModel : Model
initModel =
    let
        ( model, _ ) =
            Designer.init testShared "test-def-id"
    in
    model


{-| 全フィールドデフォルト値の CanvasState
-}
defaultCanvas : CanvasState
defaultCanvas =
    { steps = Dict.empty
    , transitions = []
    , selectedStepId = Nothing
    , selectedTransitionIndex = Nothing
    , dragging = Nothing
    , canvasBounds = Nothing
    , nextStepNumber = 1
    , propertyName = ""
    , propertyEndStatus = ""
    , name = ""
    , description = ""
    , version = 0
    , isSaving = False
    , successMessage = Nothing
    , errorMessage = Nothing
    , isDirty_ = False
    , validationResult = Nothing
    , isValidating = False
    , isPublishing = False
    , pendingPublish = False
    }


{-| Bounds を設定した CanvasState（座標変換が機能する状態）
-}
canvasWithBounds : CanvasState
canvasWithBounds =
    { defaultCanvas
        | canvasBounds = Just { x = 0, y = 0, width = 800, height = 600 }
    }


{-| ステップが1つ配置済みの CanvasState
-}
canvasWithOneStep : CanvasState
canvasWithOneStep =
    let
        step =
            { id = "approval_1"
            , stepType = Approval
            , name = "承認"
            , position = { x = 200, y = 100 }
            , assignee = Nothing
            , endStatus = Nothing
            }
    in
    { canvasWithBounds
        | steps = Dict.singleton "approval_1" step
        , nextStepNumber = 2
    }


{-| End ステップ付きの CanvasState
-}
canvasWithEndStep : CanvasState
canvasWithEndStep =
    let
        endStep =
            { id = "end_1"
            , stepType = End
            , name = "終了"
            , position = { x = 400, y = 100 }
            , assignee = Nothing
            , endStatus = Just "approved"
            }
    in
    { canvasWithBounds
        | steps = Dict.singleton "end_1" endStep
        , nextStepNumber = 2
    }


{-| Loaded 状態の基本モデル
-}
baseModel : Model
baseModel =
    { shared = testShared
    , definitionId = "test-def-id"
    , state = Loaded defaultCanvas
    }


{-| Bounds を設定したモデル（座標変換が機能する状態）
-}
modelWithBounds : Model
modelWithBounds =
    { baseModel | state = Loaded canvasWithBounds }


{-| ステップが1つ配置済みのモデル
-}
modelWithOneStep : Model
modelWithOneStep =
    { baseModel | state = Loaded canvasWithOneStep }


{-| End ステップ付きのモデル
-}
modelWithEndStep : Model
modelWithEndStep =
    { baseModel | state = Loaded canvasWithEndStep }


{-| Loaded 状態の CanvasState に対してアサーションを実行するヘルパー
-}
expectLoaded : (CanvasState -> Expect.Expectation) -> Model -> Expect.Expectation
expectLoaded assertion model =
    case model.state of
        Loaded canvas ->
            assertion canvas

        _ ->
            Expect.fail "Expected Loaded state"



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
                newModel
                    |> expectLoaded
                        (\canvas ->
                            case canvas.dragging of
                                Just (DraggingNewStep Start _) ->
                                    Expect.pass

                                _ ->
                                    Expect.fail "Expected DraggingNewStep Start"
                        )
        ]



-- CanvasMouseUp


canvasMouseUpTests : Test
canvasMouseUpTests =
    describe "CanvasMouseUp"
        [ test "DraggingNewStep 時に新しい StepNode が steps に追加される" <|
            \_ ->
                let
                    draggingModel =
                        { baseModel
                            | state = Loaded { canvasWithBounds | dragging = Just (DraggingNewStep Approval { x = 200, y = 100 }) }
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasMouseUp draggingModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> Dict.size c.steps |> Expect.equal 1
                                , \c -> c.dragging |> Expect.equal Nothing
                                , \c -> c.nextStepNumber |> Expect.equal 2
                                ]
                                canvas
                        )
        , test "dragging が Nothing にリセットされる" <|
            \_ ->
                let
                    draggingModel =
                        { baseModel
                            | state = Loaded { canvasWithBounds | dragging = Just (DraggingNewStep Start { x = 100, y = 100 }) }
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasMouseUp draggingModel
                in
                newModel
                    |> expectLoaded
                        (\canvas -> canvas.dragging |> Expect.equal Nothing)
        , test "DraggingExistingStep 時に dragging が Nothing になり位置が確定する" <|
            \_ ->
                let
                    draggingModel =
                        { baseModel
                            | state = Loaded { canvasWithOneStep | dragging = Just (DraggingExistingStep "approval_1" { x = 10, y = 10 }) }
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasMouseUp draggingModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.dragging |> Expect.equal Nothing
                                , \c -> Dict.size c.steps |> Expect.equal 1
                                ]
                                canvas
                        )
        ]



-- CanvasMouseMove


canvasMouseMoveTests : Test
canvasMouseMoveTests =
    describe "CanvasMouseMove"
        [ test "DraggingNewStep の位置が更新される" <|
            \_ ->
                let
                    draggingModel =
                        { baseModel
                            | state = Loaded { canvasWithBounds | dragging = Just (DraggingNewStep Start { x = 100, y = 100 }) }
                        }

                    ( newModel, _ ) =
                        Designer.update (CanvasMouseMove 300 200) draggingModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            case canvas.dragging of
                                Just (DraggingNewStep Start pos) ->
                                    Expect.all
                                        [ \p -> p.x |> Expect.within (Expect.Absolute 0.1) 300
                                        , \p -> p.y |> Expect.within (Expect.Absolute 0.1) 200
                                        ]
                                        pos

                                _ ->
                                    Expect.fail "Expected DraggingNewStep with updated position"
                        )
        , test "DraggingExistingStep 時にステップ位置がグリッドスナップで更新される" <|
            \_ ->
                let
                    draggingModel =
                        { baseModel
                            | state = Loaded { canvasWithOneStep | dragging = Just (DraggingExistingStep "approval_1" { x = 10, y = 10 }) }
                        }

                    ( newModel, _ ) =
                        Designer.update (CanvasMouseMove 400 300) draggingModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            case Dict.get "approval_1" canvas.steps of
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
                        )
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
                newModel
                    |> expectLoaded
                        (\canvas -> canvas.selectedStepId |> Expect.equal (Just "approval_1"))
        ]



-- CanvasBackgroundClicked


canvasBackgroundClickedTests : Test
canvasBackgroundClickedTests =
    describe "CanvasBackgroundClicked"
        [ test "selectedStepId が Nothing になる" <|
            \_ ->
                let
                    selectedModel =
                        { baseModel | state = Loaded { canvasWithOneStep | selectedStepId = Just "approval_1" } }

                    ( newModel, _ ) =
                        Designer.update CanvasBackgroundClicked selectedModel
                in
                newModel
                    |> expectLoaded
                        (\canvas -> canvas.selectedStepId |> Expect.equal Nothing)
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
                newModel
                    |> expectLoaded
                        (\canvas ->
                            case canvas.dragging of
                                Just (DraggingExistingStep stepId offset) ->
                                    Expect.all
                                        [ \_ -> stepId |> Expect.equal "approval_1"
                                        , \_ -> offset.x |> Expect.within (Expect.Absolute 0.1) 30
                                        , \_ -> offset.y |> Expect.within (Expect.Absolute 0.1) 20
                                        ]
                                        ()

                                _ ->
                                    Expect.fail "Expected DraggingExistingStep"
                        )
        , test "selectedStepId も設定される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (StepMouseDown "approval_1" 230 120) modelWithOneStep
                in
                newModel
                    |> expectLoaded
                        (\canvas -> canvas.selectedStepId |> Expect.equal (Just "approval_1"))
        ]



-- KeyDown


keyDownTests : Test
keyDownTests =
    describe "KeyDown"
        [ test "Delete で選択中のステップが削除される" <|
            \_ ->
                let
                    selectedModel =
                        { baseModel | state = Loaded { canvasWithOneStep | selectedStepId = Just "approval_1" } }

                    ( newModel, _ ) =
                        Designer.update (KeyDown "Delete") selectedModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> Dict.size c.steps |> Expect.equal 0
                                , \c -> c.selectedStepId |> Expect.equal Nothing
                                ]
                                canvas
                        )
        , test "Delete で選択中ステップがない場合は何も起きない" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (KeyDown "Delete") modelWithOneStep
                in
                newModel
                    |> expectLoaded
                        (\canvas -> Dict.size canvas.steps |> Expect.equal 1)
        , test "Backspace でも選択中ステップが削除される" <|
            \_ ->
                let
                    selectedModel =
                        { baseModel | state = Loaded { canvasWithOneStep | selectedStepId = Just "approval_1" } }

                    ( newModel, _ ) =
                        Designer.update (KeyDown "Backspace") selectedModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> Dict.size c.steps |> Expect.equal 0
                                , \c -> c.selectedStepId |> Expect.equal Nothing
                                ]
                                canvas
                        )
        ]



-- ConnectionPortMouseDown


connectionPortMouseDownTests : Test
connectionPortMouseDownTests =
    describe "ConnectionPortMouseDown"
        [ test "DraggingConnection に遷移する" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (ConnectionPortMouseDown "approval_1" 320 130) modelWithOneStep
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            case canvas.dragging of
                                Just (DraggingConnection sourceId _) ->
                                    sourceId |> Expect.equal "approval_1"

                                _ ->
                                    Expect.fail "Expected DraggingConnection"
                        )
        ]



-- TransitionClicked


transitionClickedTests : Test
transitionClickedTests =
    describe "TransitionClicked"
        [ test "selectedTransitionIndex が設定される" <|
            \_ ->
                let
                    modelWithTransitions =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithOneStep
                                        | transitions =
                                            [ { from = "start_1", to = "approval_1", trigger = Nothing } ]
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update (TransitionClicked 0) modelWithTransitions
                in
                newModel
                    |> expectLoaded
                        (\canvas -> canvas.selectedTransitionIndex |> Expect.equal (Just 0))
        , test "CanvasBackgroundClicked で selectedTransitionIndex が Nothing になる" <|
            \_ ->
                let
                    modelWithSelection =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithOneStep
                                        | transitions =
                                            [ { from = "start_1", to = "approval_1", trigger = Nothing } ]
                                        , selectedTransitionIndex = Just 0
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasBackgroundClicked modelWithSelection
                in
                newModel
                    |> expectLoaded
                        (\canvas -> canvas.selectedTransitionIndex |> Expect.equal Nothing)
        ]



-- Connection KeyDown


connectionKeyDownTests : Test
connectionKeyDownTests =
    describe "KeyDown (connections)"
        [ test "Delete でステップ削除時に関連 transitions も削除される" <|
            \_ ->
                let
                    startStep =
                        { id = "start_1"
                        , stepType = Start
                        , name = "開始"
                        , position = { x = 100, y = 100 }
                        , assignee = Nothing
                        , endStatus = Nothing
                        }

                    modelWithTransitions =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithOneStep
                                        | steps =
                                            Dict.fromList
                                                [ ( "start_1", startStep )
                                                , ( "approval_1"
                                                  , { id = "approval_1"
                                                    , stepType = Approval
                                                    , name = "承認"
                                                    , position = { x = 300, y = 100 }
                                                    , assignee = Nothing
                                                    , endStatus = Nothing
                                                    }
                                                  )
                                                ]
                                        , transitions =
                                            [ { from = "start_1", to = "approval_1", trigger = Nothing } ]
                                        , selectedStepId = Just "start_1"
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update (KeyDown "Delete") modelWithTransitions
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> Dict.member "start_1" c.steps |> Expect.equal False
                                , \c -> List.length c.transitions |> Expect.equal 0
                                ]
                                canvas
                        )
        , test "Delete で selectedTransitionIndex 時に該当 transition が削除される" <|
            \_ ->
                let
                    modelWithTransitions =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithOneStep
                                        | transitions =
                                            [ { from = "start_1", to = "approval_1", trigger = Nothing }
                                            , { from = "approval_1", to = "end_1", trigger = Just "approve" }
                                            ]
                                        , selectedTransitionIndex = Just 0
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update (KeyDown "Delete") modelWithTransitions
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> List.length c.transitions |> Expect.equal 1
                                , \c ->
                                    List.head c.transitions
                                        |> Maybe.map .from
                                        |> Expect.equal (Just "approval_1")
                                , \c -> c.selectedTransitionIndex |> Expect.equal Nothing
                                ]
                                canvas
                        )
        ]



-- Reconnection（接続線端点の付け替え）


{-| 3 ステップ + 1 transition の CanvasState（再接続テスト用）

    start_1 ( 100, 100 ) --[approve]--> approval_1 (300,100)

    end_1 ( 500, 100 )

-}
canvasWithTransition : CanvasState
canvasWithTransition =
    let
        startStep =
            { id = "start_1"
            , stepType = Start
            , name = "開始"
            , position = { x = 100, y = 100 }
            , assignee = Nothing
            , endStatus = Nothing
            }

        approvalStep =
            { id = "approval_1"
            , stepType = Approval
            , name = "承認"
            , position = { x = 300, y = 100 }
            , assignee = Nothing
            , endStatus = Nothing
            }

        endStep =
            { id = "end_1"
            , stepType = End
            , name = "終了"
            , position = { x = 500, y = 100 }
            , assignee = Nothing
            , endStatus = Just "approved"
            }
    in
    { canvasWithBounds
        | steps =
            Dict.fromList
                [ ( "start_1", startStep )
                , ( "approval_1", approvalStep )
                , ( "end_1", endStep )
                ]
        , transitions =
            [ { from = "start_1", to = "approval_1", trigger = Just "approve" } ]
        , nextStepNumber = 4
    }


reconnectionTests : Test
reconnectionTests =
    describe "Reconnection（接続線端点の付け替え）"
        [ describe "TransitionEndpointMouseDown"
            [ test "SourceEnd で DraggingReconnection に遷移する" <|
                \_ ->
                    let
                        model =
                            { baseModel | state = Loaded canvasWithTransition }

                        ( newModel, _ ) =
                            Designer.update (TransitionEndpointMouseDown 0 SourceEnd 200 130) model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas ->
                                case canvas.dragging of
                                    Just (DraggingReconnection idx end _) ->
                                        Expect.all
                                            [ \_ -> idx |> Expect.equal 0
                                            , \_ -> end |> Expect.equal SourceEnd
                                            ]
                                            ()

                                    _ ->
                                        Expect.fail "Expected DraggingReconnection with SourceEnd"
                            )
            , test "TargetEnd で DraggingReconnection に遷移する" <|
                \_ ->
                    let
                        model =
                            { baseModel | state = Loaded canvasWithTransition }

                        ( newModel, _ ) =
                            Designer.update (TransitionEndpointMouseDown 0 TargetEnd 300 130) model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas ->
                                case canvas.dragging of
                                    Just (DraggingReconnection idx end _) ->
                                        Expect.all
                                            [ \_ -> idx |> Expect.equal 0
                                            , \_ -> end |> Expect.equal TargetEnd
                                            ]
                                            ()

                                    _ ->
                                        Expect.fail "Expected DraggingReconnection with TargetEnd"
                            )
            ]
        , describe "CanvasMouseMove（DraggingReconnection）"
            [ test "Position が更新される" <|
                \_ ->
                    let
                        model =
                            { baseModel
                                | state =
                                    Loaded
                                        { canvasWithTransition
                                            | dragging = Just (DraggingReconnection 0 SourceEnd { x = 100, y = 100 })
                                        }
                            }

                        ( newModel, _ ) =
                            Designer.update (CanvasMouseMove 400 300) model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas ->
                                case canvas.dragging of
                                    Just (DraggingReconnection 0 SourceEnd pos) ->
                                        Expect.all
                                            [ \p -> p.x |> Expect.within (Expect.Absolute 0.1) 400
                                            , \p -> p.y |> Expect.within (Expect.Absolute 0.1) 300
                                            ]
                                            pos

                                    _ ->
                                        Expect.fail "Expected DraggingReconnection with updated position"
                            )
            ]
        , describe "CanvasMouseUp（DraggingReconnection）"
            [ test "SourceEnd で有効なステップにドロップ → from が更新される" <|
                \_ ->
                    let
                        -- end_1 の矩形内（500,100 - 680,190）にドロップ
                        model =
                            { baseModel
                                | state =
                                    Loaded
                                        { canvasWithTransition
                                            | dragging = Just (DraggingReconnection 0 SourceEnd { x = 550, y = 140 })
                                        }
                            }

                        ( newModel, _ ) =
                            Designer.update CanvasMouseUp model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas ->
                                case List.head canvas.transitions of
                                    Just t ->
                                        Expect.all
                                            [ \_ -> t.from |> Expect.equal "end_1"
                                            , \_ -> t.to |> Expect.equal "approval_1"
                                            , \_ -> canvas.dragging |> Expect.equal Nothing
                                            ]
                                            ()

                                    Nothing ->
                                        Expect.fail "Expected transition to exist"
                            )
            , test "TargetEnd で有効なステップにドロップ → to が更新される" <|
                \_ ->
                    let
                        -- end_1 の矩形内にドロップ
                        model =
                            { baseModel
                                | state =
                                    Loaded
                                        { canvasWithTransition
                                            | dragging = Just (DraggingReconnection 0 TargetEnd { x = 550, y = 140 })
                                        }
                            }

                        ( newModel, _ ) =
                            Designer.update CanvasMouseUp model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas ->
                                case List.head canvas.transitions of
                                    Just t ->
                                        Expect.all
                                            [ \_ -> t.from |> Expect.equal "start_1"
                                            , \_ -> t.to |> Expect.equal "end_1"
                                            , \_ -> canvas.dragging |> Expect.equal Nothing
                                            ]
                                            ()

                                    Nothing ->
                                        Expect.fail "Expected transition to exist"
                            )
            , test "空白領域にドロップ → transitions が変更されない" <|
                \_ ->
                    let
                        -- ステップがない領域（50,50）にドロップ
                        model =
                            { baseModel
                                | state =
                                    Loaded
                                        { canvasWithTransition
                                            | dragging = Just (DraggingReconnection 0 TargetEnd { x = 50, y = 50 })
                                        }
                            }

                        ( newModel, _ ) =
                            Designer.update CanvasMouseUp model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas ->
                                Expect.all
                                    [ \c ->
                                        List.head c.transitions
                                            |> Maybe.map (\t -> ( t.from, t.to ))
                                            |> Expect.equal (Just ( "start_1", "approval_1" ))
                                    , \c -> c.dragging |> Expect.equal Nothing
                                    ]
                                    canvas
                            )
            , test "反対端のステップにドロップ → transitions が変更されない（自己ループ防止）" <|
                \_ ->
                    let
                        -- SourceEnd ドラッグ中に to 側のステップ（approval_1: 300,100）にドロップ
                        model =
                            { baseModel
                                | state =
                                    Loaded
                                        { canvasWithTransition
                                            | dragging = Just (DraggingReconnection 0 SourceEnd { x = 350, y = 140 })
                                        }
                            }

                        ( newModel, _ ) =
                            Designer.update CanvasMouseUp model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas ->
                                Expect.all
                                    [ \c ->
                                        List.head c.transitions
                                            |> Maybe.map (\t -> ( t.from, t.to ))
                                            |> Expect.equal (Just ( "start_1", "approval_1" ))
                                    , \c -> c.dragging |> Expect.equal Nothing
                                    ]
                                    canvas
                            )
            , test "付け替え後、trigger が元の値を維持する" <|
                \_ ->
                    let
                        model =
                            { baseModel
                                | state =
                                    Loaded
                                        { canvasWithTransition
                                            | dragging = Just (DraggingReconnection 0 TargetEnd { x = 550, y = 140 })
                                        }
                            }

                        ( newModel, _ ) =
                            Designer.update CanvasMouseUp model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas ->
                                List.head canvas.transitions
                                    |> Maybe.andThen .trigger
                                    |> Expect.equal (Just "approve")
                            )
            , test "付け替え成功時に isDirty がマークされる" <|
                \_ ->
                    let
                        model =
                            { baseModel
                                | state =
                                    Loaded
                                        { canvasWithTransition
                                            | dragging = Just (DraggingReconnection 0 TargetEnd { x = 550, y = 140 })
                                        }
                            }

                        ( newModel, _ ) =
                            Designer.update CanvasMouseUp model
                    in
                    newModel
                        |> expectLoaded
                            (\canvas -> canvas.isDirty_ |> Expect.equal True)
            ]
        ]



-- Property Panel


propertyPanelTests : Test
propertyPanelTests =
    describe "Property Panel"
        [ test "StepClicked 後に propertyName がステップの name に同期される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (StepClicked "approval_1") modelWithOneStep
                in
                newModel
                    |> expectLoaded
                        (\canvas -> canvas.propertyName |> Expect.equal "承認")
        , test "StepClicked 後に propertyEndStatus が endStatus の値に同期される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (StepClicked "end_1") modelWithEndStep
                in
                newModel
                    |> expectLoaded
                        (\canvas -> canvas.propertyEndStatus |> Expect.equal "approved")
        , test "UpdatePropertyName でステップの name がリアルタイム更新される" <|
            \_ ->
                let
                    selectedModel =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithOneStep
                                        | selectedStepId = Just "approval_1"
                                        , propertyName = "承認"
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update (UpdatePropertyName "レビュー") selectedModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.propertyName |> Expect.equal "レビュー"
                                , \c ->
                                    Dict.get "approval_1" c.steps
                                        |> Maybe.map .name
                                        |> Expect.equal (Just "レビュー")
                                ]
                                canvas
                        )
        , test "UpdatePropertyEndStatus \"approved\" で endStatus が Just \"approved\" になる" <|
            \_ ->
                let
                    selectedModel =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithEndStep
                                        | selectedStepId = Just "end_1"
                                        , propertyEndStatus = ""
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update (UpdatePropertyEndStatus "approved") selectedModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.propertyEndStatus |> Expect.equal "approved"
                                , \c ->
                                    Dict.get "end_1" c.steps
                                        |> Maybe.andThen .endStatus
                                        |> Expect.equal (Just "approved")
                                ]
                                canvas
                        )
        , test "UpdatePropertyEndStatus \"\" で endStatus が Nothing になる" <|
            \_ ->
                let
                    selectedModel =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithEndStep
                                        | selectedStepId = Just "end_1"
                                        , propertyEndStatus = "approved"
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update (UpdatePropertyEndStatus "") selectedModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.propertyEndStatus |> Expect.equal ""
                                , \c ->
                                    Dict.get "end_1" c.steps
                                        |> Maybe.andThen .endStatus
                                        |> Expect.equal Nothing
                                ]
                                canvas
                        )
        , test "CanvasBackgroundClicked で propertyName がクリアされる" <|
            \_ ->
                let
                    selectedModel =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithOneStep
                                        | selectedStepId = Just "approval_1"
                                        , propertyName = "承認"
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasBackgroundClicked selectedModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.propertyName |> Expect.equal ""
                                , \c -> c.selectedStepId |> Expect.equal Nothing
                                ]
                                canvas
                        )
        ]



-- Drag Bounds


dragBoundsTests : Test
dragBoundsTests =
    describe "Drag Bounds"
        [ test "DraggingExistingStep でドラッグ中の位置が viewBox 内に制約される" <|
            \_ ->
                let
                    -- ステップ (200, 100)、offset (10, 10) でドラッグ中
                    draggingModel =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithOneStep
                                        | dragging = Just (DraggingExistingStep "approval_1" { x = 10, y = 10 })
                                    }
                        }

                    -- clientX=750, clientY=550 → canvas (750, 550)（1:1 変換）
                    -- snap(750-10, 550-10) = snap(740, 540) = (740, 540)
                    -- clamp: x=620 (740>620), y=510 (540>510)
                    ( newModel, _ ) =
                        Designer.update (CanvasMouseMove 750 550) draggingModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            case Dict.get "approval_1" canvas.steps of
                                Just step ->
                                    Expect.all
                                        [ \s ->
                                            s.position.x
                                                |> Expect.within (Expect.Absolute 0.1)
                                                    (DesignerCanvas.viewBoxWidth - DesignerCanvas.stepDimensions.width)
                                        , \s ->
                                            s.position.y
                                                |> Expect.within (Expect.Absolute 0.1)
                                                    (DesignerCanvas.viewBoxHeight - DesignerCanvas.stepDimensions.height)
                                        ]
                                        step

                                Nothing ->
                                    Expect.fail "Step not found"
                        )
        , test "DraggingNewStep のドロップ位置が viewBox 内に制約される" <|
            \_ ->
                let
                    -- viewBox 外にドラッグ
                    draggingModel =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithBounds
                                        | dragging = Just (DraggingNewStep Approval { x = 750, y = 550 })
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasMouseUp draggingModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            case Dict.values canvas.steps |> List.head of
                                Just step ->
                                    Expect.all
                                        [ \s ->
                                            s.position.x
                                                |> Expect.atMost (DesignerCanvas.viewBoxWidth - DesignerCanvas.stepDimensions.width)
                                        , \s ->
                                            s.position.y
                                                |> Expect.atMost (DesignerCanvas.viewBoxHeight - DesignerCanvas.stepDimensions.height)
                                        ]
                                        step

                                Nothing ->
                                    Expect.fail "No step created"
                        )
        ]



-- DeleteSelectedStep


deleteSelectedStepTests : Test
deleteSelectedStepTests =
    describe "DeleteSelectedStep"
        [ test "選択中のステップが削除される" <|
            \_ ->
                let
                    selectedModel =
                        { baseModel | state = Loaded { canvasWithOneStep | selectedStepId = Just "approval_1" } }

                    ( newModel, _ ) =
                        Designer.update DeleteSelectedStep selectedModel
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> Dict.size c.steps |> Expect.equal 0
                                , \c -> c.selectedStepId |> Expect.equal Nothing
                                ]
                                canvas
                        )
        , test "関連する接続線も削除される" <|
            \_ ->
                let
                    startStep =
                        { id = "start_1"
                        , stepType = Start
                        , name = "開始"
                        , position = { x = 100, y = 100 }
                        , assignee = Nothing
                        , endStatus = Nothing
                        }

                    modelWithTransitions =
                        { baseModel
                            | state =
                                Loaded
                                    { canvasWithOneStep
                                        | steps =
                                            Dict.fromList
                                                [ ( "start_1", startStep )
                                                , ( "approval_1"
                                                  , { id = "approval_1"
                                                    , stepType = Approval
                                                    , name = "承認"
                                                    , position = { x = 300, y = 100 }
                                                    , assignee = Nothing
                                                    , endStatus = Nothing
                                                    }
                                                  )
                                                ]
                                        , transitions =
                                            [ { from = "start_1", to = "approval_1", trigger = Nothing } ]
                                        , selectedStepId = Just "approval_1"
                                    }
                        }

                    ( newModel, _ ) =
                        Designer.update DeleteSelectedStep modelWithTransitions
                in
                newModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> Dict.member "approval_1" c.steps |> Expect.equal False
                                , \c -> List.length c.transitions |> Expect.equal 0
                                ]
                                canvas
                        )
        , test "ステップ未選択時は何もしない" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update DeleteSelectedStep modelWithOneStep
                in
                newModel
                    |> expectLoaded
                        (\canvas -> Dict.size canvas.steps |> Expect.equal 1)
        ]



-- API Integration


{-| テスト用のワークフロー定義データ
-}
testDefinition : WorkflowDefinition
testDefinition =
    { id = "test-def-id"
    , name = "テスト定義"
    , description = Just "テスト説明"
    , version = 1
    , definition = Encode.object [ ( "steps", Encode.list identity [] ), ( "transitions", Encode.list identity [] ) ]
    , status = "draft"
    , createdBy = "user-1"
    , createdAt = "2026-01-01T00:00:00"
    , updatedAt = "2026-01-01T00:00:00"
    }


{-| ロード完了済みモデル（API テストの基盤）

GotDefinition 経由で構築された canvas を持つ。

-}
loadedModel : Model
loadedModel =
    let
        ( model, _ ) =
            Designer.update (GotDefinition (Ok testDefinition)) initModel
    in
    model


{-| ロード完了済みの CanvasState
-}
loadedCanvas : CanvasState
loadedCanvas =
    case loadedModel.state of
        Loaded canvas ->
            canvas

        _ ->
            defaultCanvas


apiIntegrationTests : Test
apiIntegrationTests =
    describe "API Integration"
        [ test "GotDefinition Ok でロード状態が Loaded になる" <|
            \_ ->
                case loadedModel.state of
                    Loaded _ ->
                        Expect.pass

                    _ ->
                        Expect.fail "Expected Loaded"
        , test "GotDefinition Ok で name と version が設定される" <|
            \_ ->
                loadedModel
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.name |> Expect.equal "テスト定義"
                                , \c -> c.description |> Expect.equal "テスト説明"
                                , \c -> c.version |> Expect.equal 1
                                ]
                                canvas
                        )
        , test "GotDefinition Err でロード状態が Failed になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update
                            (GotDefinition (Err NetworkError))
                            initModel
                in
                case model.state of
                    Failed _ ->
                        Expect.pass

                    _ ->
                        Expect.fail "Expected Failed"
        , test "SaveClicked で isSaving が True になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update SaveClicked loadedModel
                in
                model
                    |> expectLoaded
                        (\canvas -> canvas.isSaving |> Expect.equal True)
        , test "GotSaveResult Ok で isSaving が False になり version が更新される" <|
            \_ ->
                let
                    savingModel =
                        { baseModel | state = Loaded { loadedCanvas | isSaving = True, isDirty_ = True } }

                    updatedDef =
                        { testDefinition | version = 2 }

                    ( model, _ ) =
                        Designer.update (GotSaveResult (Ok updatedDef)) savingModel
                in
                model
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.isSaving |> Expect.equal False
                                , \c -> c.version |> Expect.equal 2
                                , \c -> c.successMessage |> Expect.equal (Just "保存しました")
                                , \c -> c.isDirty_ |> Expect.equal False
                                ]
                                canvas
                        )
        , test "GotSaveResult Err で errorMessage が設定される" <|
            \_ ->
                let
                    savingModel =
                        { baseModel | state = Loaded { loadedCanvas | isSaving = True } }

                    ( model, _ ) =
                        Designer.update
                            (GotSaveResult (Err NetworkError))
                            savingModel
                in
                model
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.isSaving |> Expect.equal False
                                , \c -> c.errorMessage |> Expect.notEqual Nothing
                                ]
                                canvas
                        )
        , test "DismissMessage で successMessage と errorMessage がクリアされる" <|
            \_ ->
                let
                    modelWithMessages =
                        { baseModel
                            | state =
                                Loaded
                                    { loadedCanvas
                                        | successMessage = Just "保存しました"
                                        , errorMessage = Just "エラー"
                                    }
                        }

                    ( model, _ ) =
                        Designer.update DismissMessage modelWithMessages
                in
                model
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.successMessage |> Expect.equal Nothing
                                , \c -> c.errorMessage |> Expect.equal Nothing
                                ]
                                canvas
                        )
        , test "UpdateDefinitionName で name が更新され isDirty_ が True になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update (UpdateDefinitionName "新しい名前") loadedModel
                in
                model
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.name |> Expect.equal "新しい名前"
                                , \c -> c.isDirty_ |> Expect.equal True
                                ]
                                canvas
                        )
        ]



-- Validation & Publish


validationAndPublishTests : Test
validationAndPublishTests =
    describe "Validation & Publish"
        [ test "ValidateClicked で isValidating が True になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update ValidateClicked loadedModel
                in
                model
                    |> expectLoaded
                        (\canvas -> canvas.isValidating |> Expect.equal True)
        , test "GotValidationResult Ok (valid=true) で validationResult が設定される" <|
            \_ ->
                let
                    validatingModel =
                        { baseModel | state = Loaded { loadedCanvas | isValidating = True } }

                    validResult : ValidationResult
                    validResult =
                        { valid = True, errors = [] }

                    ( model, _ ) =
                        Designer.update (GotValidationResult (Ok validResult)) validatingModel
                in
                model
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.isValidating |> Expect.equal False
                                , \c -> c.validationResult |> Expect.equal (Just validResult)
                                ]
                                canvas
                        )
        , test "GotValidationResult Ok (valid=false) でエラー情報が設定される" <|
            \_ ->
                let
                    validatingModel =
                        { baseModel | state = Loaded { loadedCanvas | isValidating = True } }

                    invalidResult : ValidationResult
                    invalidResult =
                        { valid = False
                        , errors =
                            [ { code = "missing_start"
                              , message = "開始ステップが必要です"
                              , stepId = Nothing
                              }
                            ]
                        }

                    ( model, _ ) =
                        Designer.update (GotValidationResult (Ok invalidResult)) validatingModel
                in
                model
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.isValidating |> Expect.equal False
                                , \c ->
                                    c.validationResult
                                        |> Maybe.map .valid
                                        |> Expect.equal (Just False)
                                , \c ->
                                    c.validationResult
                                        |> Maybe.map .errors
                                        |> Maybe.map List.length
                                        |> Expect.equal (Just 1)
                                ]
                                canvas
                        )
        , test "GotValidationResult Err で errorMessage が設定される" <|
            \_ ->
                let
                    validatingModel =
                        { baseModel | state = Loaded { loadedCanvas | isValidating = True } }

                    ( model, _ ) =
                        Designer.update (GotValidationResult (Err NetworkError)) validatingModel
                in
                model
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.isValidating |> Expect.equal False
                                , \c -> c.errorMessage |> Expect.notEqual Nothing
                                ]
                                canvas
                        )
        , test "PublishClicked で pendingPublish が True になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update PublishClicked loadedModel
                in
                model
                    |> expectLoaded
                        (\canvas -> canvas.pendingPublish |> Expect.equal True)
        , test "GotPublishResult Ok で successMessage が設定される" <|
            \_ ->
                let
                    publishingModel =
                        { baseModel | state = Loaded { loadedCanvas | isPublishing = True } }

                    publishedDef =
                        { testDefinition | status = "published", version = 2 }

                    ( model, _ ) =
                        Designer.update (GotPublishResult (Ok publishedDef)) publishingModel
                in
                model
                    |> expectLoaded
                        (\canvas ->
                            Expect.all
                                [ \c -> c.isPublishing |> Expect.equal False
                                , \c -> c.successMessage |> Expect.equal (Just "公開しました")
                                , \c -> c.version |> Expect.equal 2
                                ]
                                canvas
                        )
        ]
