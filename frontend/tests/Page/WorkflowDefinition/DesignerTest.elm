module Page.WorkflowDefinition.DesignerTest exposing (suite)

{-| Designer ページの update ロジックテスト

ドラッグ&ドロップ、選択、移動、削除の状態遷移を検証する。

-}

import Api exposing (ApiError(..))
import Data.DesignerCanvas exposing (DraggingState(..), StepType(..))
import Data.WorkflowDefinition exposing (ValidationResult, WorkflowDefinition)
import Dict
import Expect
import Json.Encode as Encode
import Page.WorkflowDefinition.Designer as Designer exposing (Model, Msg(..))
import RemoteData exposing (RemoteData(..))
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
        , propertyPanelTests
        , apiIntegrationTests
        , validationAndPublishTests
        ]



-- テスト用ヘルパー


testShared : Shared
testShared =
    Shared.init { apiBaseUrl = "", timezoneOffsetMinutes = 540 }


initModel : Model
initModel =
    let
        ( model, _ ) =
            Designer.init testShared "test-def-id"
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
            , assignee = Nothing
            , endStatus = Nothing
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
                case newModel.dragging of
                    Just (DraggingConnection sourceId _) ->
                        sourceId |> Expect.equal "approval_1"

                    _ ->
                        Expect.fail "Expected DraggingConnection"
        ]



-- TransitionClicked


transitionClickedTests : Test
transitionClickedTests =
    describe "TransitionClicked"
        [ test "selectedTransitionIndex が設定される" <|
            \_ ->
                let
                    modelWithTransitions =
                        { modelWithOneStep
                            | transitions =
                                [ { from = "start_1", to = "approval_1", trigger = Nothing } ]
                        }

                    ( newModel, _ ) =
                        Designer.update (TransitionClicked 0) modelWithTransitions
                in
                newModel.selectedTransitionIndex |> Expect.equal (Just 0)
        , test "CanvasBackgroundClicked で selectedTransitionIndex が Nothing になる" <|
            \_ ->
                let
                    modelWithSelection =
                        { modelWithOneStep
                            | transitions =
                                [ { from = "start_1", to = "approval_1", trigger = Nothing } ]
                            , selectedTransitionIndex = Just 0
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasBackgroundClicked modelWithSelection
                in
                newModel.selectedTransitionIndex |> Expect.equal Nothing
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
                        { modelWithOneStep
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

                    ( newModel, _ ) =
                        Designer.update (KeyDown "Delete") modelWithTransitions
                in
                Expect.all
                    [ \m -> Dict.member "start_1" m.steps |> Expect.equal False
                    , \m -> List.length m.transitions |> Expect.equal 0
                    ]
                    newModel
        , test "Delete で selectedTransitionIndex 時に該当 transition が削除される" <|
            \_ ->
                let
                    modelWithTransitions =
                        { modelWithOneStep
                            | transitions =
                                [ { from = "start_1", to = "approval_1", trigger = Nothing }
                                , { from = "approval_1", to = "end_1", trigger = Just "approve" }
                                ]
                            , selectedTransitionIndex = Just 0
                        }

                    ( newModel, _ ) =
                        Designer.update (KeyDown "Delete") modelWithTransitions
                in
                Expect.all
                    [ \m -> List.length m.transitions |> Expect.equal 1
                    , \m ->
                        List.head m.transitions
                            |> Maybe.map .from
                            |> Expect.equal (Just "approval_1")
                    , \m -> m.selectedTransitionIndex |> Expect.equal Nothing
                    ]
                    newModel
        ]



-- Property Panel


{-| End ステップ付きのモデル
-}
modelWithEndStep : Model
modelWithEndStep =
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
    { modelWithBounds
        | steps = Dict.singleton "end_1" endStep
        , nextStepNumber = 2
    }


propertyPanelTests : Test
propertyPanelTests =
    describe "Property Panel"
        [ test "StepClicked 後に propertyName がステップの name に同期される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (StepClicked "approval_1") modelWithOneStep
                in
                newModel.propertyName |> Expect.equal "承認"
        , test "StepClicked 後に propertyEndStatus が endStatus の値に同期される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        Designer.update (StepClicked "end_1") modelWithEndStep
                in
                newModel.propertyEndStatus |> Expect.equal "approved"
        , test "UpdatePropertyName でステップの name がリアルタイム更新される" <|
            \_ ->
                let
                    selectedModel =
                        { modelWithOneStep
                            | selectedStepId = Just "approval_1"
                            , propertyName = "承認"
                        }

                    ( newModel, _ ) =
                        Designer.update (UpdatePropertyName "レビュー") selectedModel
                in
                Expect.all
                    [ \m -> m.propertyName |> Expect.equal "レビュー"
                    , \m ->
                        Dict.get "approval_1" m.steps
                            |> Maybe.map .name
                            |> Expect.equal (Just "レビュー")
                    ]
                    newModel
        , test "UpdatePropertyEndStatus \"approved\" で endStatus が Just \"approved\" になる" <|
            \_ ->
                let
                    selectedModel =
                        { modelWithEndStep
                            | selectedStepId = Just "end_1"
                            , propertyEndStatus = ""
                        }

                    ( newModel, _ ) =
                        Designer.update (UpdatePropertyEndStatus "approved") selectedModel
                in
                Expect.all
                    [ \m -> m.propertyEndStatus |> Expect.equal "approved"
                    , \m ->
                        Dict.get "end_1" m.steps
                            |> Maybe.andThen .endStatus
                            |> Expect.equal (Just "approved")
                    ]
                    newModel
        , test "UpdatePropertyEndStatus \"\" で endStatus が Nothing になる" <|
            \_ ->
                let
                    selectedModel =
                        { modelWithEndStep
                            | selectedStepId = Just "end_1"
                            , propertyEndStatus = "approved"
                        }

                    ( newModel, _ ) =
                        Designer.update (UpdatePropertyEndStatus "") selectedModel
                in
                Expect.all
                    [ \m -> m.propertyEndStatus |> Expect.equal ""
                    , \m ->
                        Dict.get "end_1" m.steps
                            |> Maybe.andThen .endStatus
                            |> Expect.equal Nothing
                    ]
                    newModel
        , test "CanvasBackgroundClicked で propertyName がクリアされる" <|
            \_ ->
                let
                    selectedModel =
                        { modelWithOneStep
                            | selectedStepId = Just "approval_1"
                            , propertyName = "承認"
                        }

                    ( newModel, _ ) =
                        Designer.update CanvasBackgroundClicked selectedModel
                in
                Expect.all
                    [ \m -> m.propertyName |> Expect.equal ""
                    , \m -> m.selectedStepId |> Expect.equal Nothing
                    ]
                    newModel
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
-}
loadedModel : Model
loadedModel =
    let
        ( model, _ ) =
            Designer.update (GotDefinition (Ok testDefinition)) initModel
    in
    model


apiIntegrationTests : Test
apiIntegrationTests =
    describe "API Integration"
        [ test "GotDefinition Ok でロード状態が Success になる" <|
            \_ ->
                case loadedModel.loadState of
                    Success _ ->
                        Expect.pass

                    _ ->
                        Expect.fail "Expected Success"
        , test "GotDefinition Ok で name と version が設定される" <|
            \_ ->
                Expect.all
                    [ \m -> m.name |> Expect.equal "テスト定義"
                    , \m -> m.description |> Expect.equal "テスト説明"
                    , \m -> m.version |> Expect.equal 1
                    ]
                    loadedModel
        , test "GotDefinition Err でロード状態が Failure になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update
                            (GotDefinition (Err NetworkError))
                            initModel
                in
                case model.loadState of
                    Failure _ ->
                        Expect.pass

                    _ ->
                        Expect.fail "Expected Failure"
        , test "SaveClicked で isSaving が True になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update SaveClicked loadedModel
                in
                model.isSaving |> Expect.equal True
        , test "GotSaveResult Ok で isSaving が False になり version が更新される" <|
            \_ ->
                let
                    savingModel =
                        { loadedModel | isSaving = True, isDirty_ = True }

                    updatedDef =
                        { testDefinition | version = 2 }

                    ( model, _ ) =
                        Designer.update (GotSaveResult (Ok updatedDef)) savingModel
                in
                Expect.all
                    [ \m -> m.isSaving |> Expect.equal False
                    , \m -> m.version |> Expect.equal 2
                    , \m -> m.successMessage |> Expect.equal (Just "保存しました")
                    , \m -> m.isDirty_ |> Expect.equal False
                    ]
                    model
        , test "GotSaveResult Err で errorMessage が設定される" <|
            \_ ->
                let
                    savingModel =
                        { loadedModel | isSaving = True }

                    ( model, _ ) =
                        Designer.update
                            (GotSaveResult (Err NetworkError))
                            savingModel
                in
                Expect.all
                    [ \m -> m.isSaving |> Expect.equal False
                    , \m -> m.errorMessage |> Expect.notEqual Nothing
                    ]
                    model
        , test "DismissMessage で successMessage と errorMessage がクリアされる" <|
            \_ ->
                let
                    modelWithMessages =
                        { loadedModel
                            | successMessage = Just "保存しました"
                            , errorMessage = Just "エラー"
                        }

                    ( model, _ ) =
                        Designer.update DismissMessage modelWithMessages
                in
                Expect.all
                    [ \m -> m.successMessage |> Expect.equal Nothing
                    , \m -> m.errorMessage |> Expect.equal Nothing
                    ]
                    model
        , test "UpdateDefinitionName で name が更新され isDirty_ が True になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update (UpdateDefinitionName "新しい名前") loadedModel
                in
                Expect.all
                    [ \m -> m.name |> Expect.equal "新しい名前"
                    , \m -> m.isDirty_ |> Expect.equal True
                    ]
                    model
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
                model.isValidating |> Expect.equal True
        , test "GotValidationResult Ok (valid=true) で validationResult が設定される" <|
            \_ ->
                let
                    validatingModel =
                        { loadedModel | isValidating = True }

                    validResult : ValidationResult
                    validResult =
                        { valid = True, errors = [] }

                    ( model, _ ) =
                        Designer.update (GotValidationResult (Ok validResult)) validatingModel
                in
                Expect.all
                    [ \m -> m.isValidating |> Expect.equal False
                    , \m -> m.validationResult |> Expect.equal (Just validResult)
                    ]
                    model
        , test "GotValidationResult Ok (valid=false) でエラー情報が設定される" <|
            \_ ->
                let
                    validatingModel =
                        { loadedModel | isValidating = True }

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
                Expect.all
                    [ \m -> m.isValidating |> Expect.equal False
                    , \m ->
                        m.validationResult
                            |> Maybe.map .valid
                            |> Expect.equal (Just False)
                    , \m ->
                        m.validationResult
                            |> Maybe.map .errors
                            |> Maybe.map List.length
                            |> Expect.equal (Just 1)
                    ]
                    model
        , test "GotValidationResult Err で errorMessage が設定される" <|
            \_ ->
                let
                    validatingModel =
                        { loadedModel | isValidating = True }

                    ( model, _ ) =
                        Designer.update (GotValidationResult (Err NetworkError)) validatingModel
                in
                Expect.all
                    [ \m -> m.isValidating |> Expect.equal False
                    , \m -> m.errorMessage |> Expect.notEqual Nothing
                    ]
                    model
        , test "PublishClicked で pendingPublish が True になる" <|
            \_ ->
                let
                    ( model, _ ) =
                        Designer.update PublishClicked loadedModel
                in
                model.pendingPublish |> Expect.equal True
        , test "GotPublishResult Ok で successMessage が設定される" <|
            \_ ->
                let
                    publishingModel =
                        { loadedModel | isPublishing = True }

                    publishedDef =
                        { testDefinition | status = "published", version = 2 }

                    ( model, _ ) =
                        Designer.update (GotPublishResult (Ok publishedDef)) publishingModel
                in
                Expect.all
                    [ \m -> m.isPublishing |> Expect.equal False
                    , \m -> m.successMessage |> Expect.equal (Just "公開しました")
                    , \m -> m.version |> Expect.equal 2
                    ]
                    model
        ]
