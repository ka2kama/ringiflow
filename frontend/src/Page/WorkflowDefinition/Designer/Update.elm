module Page.WorkflowDefinition.Designer.Update exposing (handleGotDefinition, updateLoaded)

{-| Designer ページの Update ロジック

updateLoaded（Loaded 状態でのメッセージ処理）と
ヘルパー関数を集約する。

永続化操作（Save/Validate/Publish チェーン）と接続線付け替え処理は
UpdatePersistence モジュールに委譲する。

-}

import Api exposing (ApiError)
import Data.DesignerCanvas as DesignerCanvas exposing (DraggingState(..))
import Data.WorkflowDefinition exposing (WorkflowDefinition)
import Dict
import Form.DirtyState as DirtyState
import List.Extra
import Page.WorkflowDefinition.Designer.Types exposing (CanvasState, Model, Msg(..), PageState(..), canvasElementId)
import Page.WorkflowDefinition.Designer.UpdatePersistence as UpdatePersistence
import Ports
import Shared exposing (Shared)


{-| GotDefinition メッセージを処理し、Loading → Loaded/Failed に遷移する
-}
handleGotDefinition : Result ApiError WorkflowDefinition -> Model -> ( Model, Cmd Msg )
handleGotDefinition result model =
    case result of
        Ok def ->
            let
                steps =
                    DesignerCanvas.loadStepsFromDefinition def.definition
                        |> Result.withDefault Dict.empty

                transitions =
                    DesignerCanvas.loadTransitionsFromDefinition def.definition
                        |> Result.withDefault []

                nextNumber =
                    Dict.size steps + 1
            in
            ( { model
                | state =
                    Loaded
                        { steps = steps
                        , transitions = transitions
                        , selectedStepId = Nothing
                        , selectedTransitionIndex = Nothing
                        , dragging = Nothing
                        , canvasBounds = Nothing
                        , nextStepNumber = nextNumber
                        , propertyName = ""
                        , propertyEndStatus = ""
                        , name = def.name
                        , description = def.description |> Maybe.withDefault ""
                        , version = def.version
                        , isSaving = False
                        , successMessage = Nothing
                        , errorMessage = Nothing
                        , isDirty_ = False
                        , validationResult = Nothing
                        , isValidating = False
                        , isPublishing = False
                        , pendingPublish = False
                        }
              }
            , Ports.requestCanvasBounds canvasElementId
            )

        Err err ->
            ( { model | state = Failed err }
            , Cmd.none
            )


{-| Loaded 状態でのメッセージ処理

API 呼び出しに必要な shared / definitionId は外側 Model のフィールドを
パラメータとして受け取る（CanvasState が API 接続情報を持たない責務分離）。

-}
updateLoaded : Msg -> Shared -> String -> CanvasState -> ( CanvasState, Cmd Msg )
updateLoaded msg shared definitionId canvas =
    case msg of
        PaletteMouseDown stepType ->
            ( { canvas
                | dragging = Just (DraggingNewStep stepType { x = 0, y = 0 })
              }
            , Ports.requestCanvasBounds canvasElementId
            )

        CanvasMouseMove clientX clientY ->
            case canvas.dragging of
                Just (DraggingNewStep stepType _) ->
                    case DesignerCanvas.clientToCanvas canvas.canvasBounds clientX clientY of
                        Just canvasPos ->
                            ( { canvas
                                | dragging = Just (DraggingNewStep stepType canvasPos)
                              }
                            , Cmd.none
                            )

                        Nothing ->
                            ( canvas, Cmd.none )

                Just (DraggingExistingStep stepId offset) ->
                    case DesignerCanvas.clientToCanvas canvas.canvasBounds clientX clientY of
                        Just canvasPos ->
                            let
                                newPos =
                                    { x = DesignerCanvas.snapToGrid (canvasPos.x - offset.x)
                                    , y = DesignerCanvas.snapToGrid (canvasPos.y - offset.y)
                                    }
                                        |> DesignerCanvas.clampToViewBox

                                updatedSteps =
                                    Dict.update stepId
                                        (Maybe.map (\step -> { step | position = newPos }))
                                        canvas.steps
                            in
                            ( { canvas | steps = updatedSteps }
                            , Cmd.none
                            )

                        Nothing ->
                            ( canvas, Cmd.none )

                Just (DraggingConnection sourceId _) ->
                    case DesignerCanvas.clientToCanvas canvas.canvasBounds clientX clientY of
                        Just canvasPos ->
                            ( { canvas | dragging = Just (DraggingConnection sourceId canvasPos) }
                            , Cmd.none
                            )

                        Nothing ->
                            ( canvas, Cmd.none )

                Just (DraggingReconnection index end _) ->
                    case DesignerCanvas.clientToCanvas canvas.canvasBounds clientX clientY of
                        Just canvasPos ->
                            ( { canvas | dragging = Just (DraggingReconnection index end canvasPos) }
                            , Cmd.none
                            )

                        Nothing ->
                            ( canvas, Cmd.none )

                Nothing ->
                    ( canvas, Cmd.none )

        CanvasMouseUp ->
            case canvas.dragging of
                Just (DraggingNewStep stepType dropPos) ->
                    let
                        newStep =
                            DesignerCanvas.createStepFromDrop stepType canvas.nextStepNumber dropPos
                                |> (\s -> { s | position = DesignerCanvas.clampToViewBox s.position })

                        ( dirtyCanvas, dirtyCmd ) =
                            DirtyState.markDirty canvas
                    in
                    ( { dirtyCanvas
                        | steps = Dict.insert newStep.id newStep dirtyCanvas.steps
                        , dragging = Nothing
                        , nextStepNumber = dirtyCanvas.nextStepNumber + 1
                        , selectedStepId = Just newStep.id
                      }
                    , dirtyCmd
                    )

                Just (DraggingExistingStep _ _) ->
                    let
                        ( dirtyCanvas, dirtyCmd ) =
                            DirtyState.markDirty canvas
                    in
                    ( { dirtyCanvas | dragging = Nothing }
                    , dirtyCmd
                    )

                Just (DraggingConnection sourceId mousePos) ->
                    let
                        -- ドロップ先のステップを判定
                        targetStep =
                            canvas.steps
                                |> Dict.values
                                |> List.filter (\s -> s.id /= sourceId)
                                |> List.filter (DesignerCanvas.stepContainsPoint mousePos)
                                |> List.head
                    in
                    case targetStep of
                        Just target ->
                            let
                                sourceStep =
                                    Dict.get sourceId canvas.steps

                                trigger =
                                    case sourceStep of
                                        Just src ->
                                            DesignerCanvas.autoTrigger src.stepType sourceId canvas.transitions

                                        Nothing ->
                                            Nothing

                                newTransition =
                                    { from = sourceId, to = target.id, trigger = trigger }

                                ( dirtyCanvas, dirtyCmd ) =
                                    DirtyState.markDirty canvas
                            in
                            ( { dirtyCanvas
                                | transitions = dirtyCanvas.transitions ++ [ newTransition ]
                                , dragging = Nothing
                              }
                            , dirtyCmd
                            )

                        Nothing ->
                            ( { canvas | dragging = Nothing }
                            , Cmd.none
                            )

                Just (DraggingReconnection index end mousePos) ->
                    UpdatePersistence.handleReconnectionDrop index end mousePos canvas

                Nothing ->
                    ( canvas, Cmd.none )

        StepClicked stepId ->
            ( syncPropertyFields stepId { canvas | selectedStepId = Just stepId, selectedTransitionIndex = Nothing }
            , Cmd.none
            )

        CanvasBackgroundClicked ->
            ( { canvas | selectedStepId = Nothing, selectedTransitionIndex = Nothing, propertyName = "", propertyEndStatus = "" }
            , Cmd.none
            )

        ConnectionPortMouseDown sourceStepId clientX clientY ->
            case DesignerCanvas.clientToCanvas canvas.canvasBounds clientX clientY of
                Just canvasPos ->
                    ( { canvas | dragging = Just (DraggingConnection sourceStepId canvasPos) }
                    , Cmd.none
                    )

                Nothing ->
                    ( canvas, Cmd.none )

        TransitionEndpointMouseDown index end clientX clientY ->
            case DesignerCanvas.clientToCanvas canvas.canvasBounds clientX clientY of
                Just canvasPos ->
                    ( { canvas | dragging = Just (DraggingReconnection index end canvasPos) }
                    , Cmd.none
                    )

                Nothing ->
                    ( canvas, Cmd.none )

        TransitionClicked index ->
            ( { canvas | selectedTransitionIndex = Just index, selectedStepId = Nothing, propertyName = "", propertyEndStatus = "" }
            , Cmd.none
            )

        UpdatePropertyName newName ->
            case canvas.selectedStepId of
                Just stepId ->
                    let
                        ( dirtyCanvas, dirtyCmd ) =
                            DirtyState.markDirty canvas
                    in
                    ( { dirtyCanvas
                        | propertyName = newName
                        , steps =
                            Dict.update stepId
                                (Maybe.map (\step -> { step | name = newName }))
                                dirtyCanvas.steps
                      }
                    , dirtyCmd
                    )

                Nothing ->
                    ( canvas, Cmd.none )

        UpdatePropertyEndStatus newStatus ->
            case canvas.selectedStepId of
                Just stepId ->
                    let
                        endStatus =
                            if newStatus == "" then
                                Nothing

                            else
                                Just newStatus

                        ( dirtyCanvas, dirtyCmd ) =
                            DirtyState.markDirty canvas
                    in
                    ( { dirtyCanvas
                        | propertyEndStatus = newStatus
                        , steps =
                            Dict.update stepId
                                (Maybe.map (\step -> { step | endStatus = endStatus }))
                                dirtyCanvas.steps
                      }
                    , dirtyCmd
                    )

                Nothing ->
                    ( canvas, Cmd.none )

        UpdateDefinitionName newName ->
            let
                ( dirtyCanvas, dirtyCmd ) =
                    DirtyState.markDirty canvas
            in
            ( { dirtyCanvas | name = newName }, dirtyCmd )

        SaveClicked ->
            UpdatePersistence.handleSave shared definitionId canvas

        GotSaveResult result ->
            UpdatePersistence.handleSaveResult shared result canvas

        ValidateClicked ->
            UpdatePersistence.handleValidate shared canvas

        GotValidationResult result ->
            UpdatePersistence.handleValidationResult shared definitionId result canvas

        PublishClicked ->
            UpdatePersistence.handlePublishClicked canvas

        ConfirmPublish ->
            UpdatePersistence.handleConfirmPublish shared definitionId canvas

        CancelPublish ->
            UpdatePersistence.handleCancelPublish canvas

        GotPublishResult result ->
            UpdatePersistence.handlePublishResult result canvas

        DismissMessage ->
            UpdatePersistence.handleDismissMessage canvas

        StepMouseDown stepId clientX clientY ->
            case ( Dict.get stepId canvas.steps, DesignerCanvas.clientToCanvas canvas.canvasBounds clientX clientY ) of
                ( Just step, Just canvasPos ) ->
                    let
                        offset =
                            { x = canvasPos.x - step.position.x
                            , y = canvasPos.y - step.position.y
                            }
                    in
                    ( syncPropertyFields stepId
                        { canvas
                            | dragging = Just (DraggingExistingStep stepId offset)
                            , selectedStepId = Just stepId
                        }
                    , Cmd.none
                    )

                _ ->
                    ( syncPropertyFields stepId { canvas | selectedStepId = Just stepId }
                    , Cmd.none
                    )

        DeleteSelectedStep ->
            deleteSelectedStep canvas

        DeleteSelectedTransition ->
            deleteSelectedTransition canvas

        KeyDown key ->
            if key == "Delete" || key == "Backspace" then
                case ( canvas.selectedTransitionIndex, canvas.selectedStepId ) of
                    ( Just _, _ ) ->
                        deleteSelectedTransition canvas

                    ( Nothing, _ ) ->
                        deleteSelectedStep canvas

            else
                ( canvas, Cmd.none )

        GotCanvasBounds value ->
            case DesignerCanvas.decodeBounds value of
                Ok bounds ->
                    ( { canvas | canvasBounds = Just bounds }
                    , Cmd.none
                    )

                Err _ ->
                    ( canvas, Cmd.none )

        -- GotDefinition は外側の update で処理済み（ここには到達しない）
        GotDefinition _ ->
            ( canvas, Cmd.none )



-- ヘルパー関数


{-| 選択されたステップのプロパティをフォームフィールドに同期する
-}
syncPropertyFields : String -> CanvasState -> CanvasState
syncPropertyFields stepId canvas =
    case Dict.get stepId canvas.steps of
        Just step ->
            { canvas
                | propertyName = step.name
                , propertyEndStatus = step.endStatus |> Maybe.withDefault ""
            }

        Nothing ->
            canvas


{-| 選択中のステップと関連する接続線を削除する
-}
deleteSelectedStep : CanvasState -> ( CanvasState, Cmd Msg )
deleteSelectedStep canvas =
    case canvas.selectedStepId of
        Just stepId ->
            let
                ( dirtyCanvas, dirtyCmd ) =
                    DirtyState.markDirty canvas
            in
            ( { dirtyCanvas
                | steps = Dict.remove stepId dirtyCanvas.steps
                , transitions =
                    List.filter
                        (\t -> t.from /= stepId && t.to /= stepId)
                        dirtyCanvas.transitions
                , selectedStepId = Nothing
              }
            , dirtyCmd
            )

        Nothing ->
            ( canvas, Cmd.none )


{-| 選択中の接続線を削除する
-}
deleteSelectedTransition : CanvasState -> ( CanvasState, Cmd Msg )
deleteSelectedTransition canvas =
    case canvas.selectedTransitionIndex of
        Just index ->
            let
                ( dirtyCanvas, dirtyCmd ) =
                    DirtyState.markDirty canvas
            in
            ( { dirtyCanvas
                | transitions = List.Extra.removeAt index dirtyCanvas.transitions
                , selectedTransitionIndex = Nothing
              }
            , dirtyCmd
            )

        Nothing ->
            ( canvas, Cmd.none )
