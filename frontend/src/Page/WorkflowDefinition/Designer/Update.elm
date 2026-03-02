module Page.WorkflowDefinition.Designer.Update exposing (handleGotDefinition, updateLoaded)

{-| Designer ページの Update ロジック

updateLoaded（Loaded 状態でのメッセージ処理）と
ヘルパー関数を集約する。

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Component.ConfirmDialog as ConfirmDialog
import Data.DesignerCanvas as DesignerCanvas exposing (DraggingState(..), ReconnectEnd(..), StepType(..))
import Data.WorkflowDefinition as WorkflowDefinition exposing (WorkflowDefinition)
import Dict
import Form.DirtyState as DirtyState
import List.Extra
import Page.WorkflowDefinition.Designer.Types exposing (CanvasState, Model, Msg(..), PageState(..), canvasElementId)
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
                    handleReconnectionDrop index end mousePos canvas

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
            let
                definition =
                    DesignerCanvas.encodeDefinition canvas.steps canvas.transitions

                body =
                    WorkflowDefinition.encodeUpdateRequest
                        { name = canvas.name
                        , description = canvas.description
                        , definition = definition
                        , version = canvas.version
                        }
            in
            ( { canvas | isSaving = True, successMessage = Nothing, errorMessage = Nothing }
            , WorkflowDefinitionApi.updateDefinition
                { config = Shared.toRequestConfig shared
                , id = definitionId
                , body = body
                , toMsg = GotSaveResult
                }
            )

        GotSaveResult result ->
            case result of
                Ok def ->
                    let
                        ( cleanCanvas, cleanCmd ) =
                            DirtyState.clearDirty canvas
                    in
                    if canvas.pendingPublish then
                        -- 公開チェーン: 保存成功 → バリデーション
                        let
                            definition =
                                DesignerCanvas.encodeDefinition cleanCanvas.steps cleanCanvas.transitions
                        in
                        ( { cleanCanvas
                            | isSaving = False
                            , version = def.version
                            , isValidating = True
                            , validationResult = Nothing
                          }
                        , Cmd.batch
                            [ cleanCmd
                            , WorkflowDefinitionApi.validateDefinition
                                { config = Shared.toRequestConfig shared
                                , body = definition
                                , toMsg = GotValidationResult
                                }
                            ]
                        )

                    else
                        ( { cleanCanvas
                            | isSaving = False
                            , version = def.version
                            , successMessage = Just "保存しました"
                            , errorMessage = Nothing
                          }
                        , cleanCmd
                        )

                Err err ->
                    ( { canvas
                        | isSaving = False
                        , pendingPublish = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err)
                        , successMessage = Nothing
                      }
                    , Cmd.none
                    )

        ValidateClicked ->
            let
                definition =
                    DesignerCanvas.encodeDefinition canvas.steps canvas.transitions
            in
            ( { canvas | isValidating = True, validationResult = Nothing, errorMessage = Nothing }
            , WorkflowDefinitionApi.validateDefinition
                { config = Shared.toRequestConfig shared
                , body = WorkflowDefinition.encodeValidationRequest { definition = definition }
                , toMsg = GotValidationResult
                }
            )

        GotValidationResult result ->
            case result of
                Ok validResult ->
                    if canvas.pendingPublish && validResult.valid then
                        -- 公開チェーン: バリデーション成功 → 公開 API 呼び出し
                        ( { canvas
                            | isValidating = False
                            , validationResult = Just validResult
                            , isPublishing = True
                          }
                        , WorkflowDefinitionApi.publishDefinition
                            { config = Shared.toRequestConfig shared
                            , id = definitionId
                            , body = WorkflowDefinition.encodeVersionRequest { version = canvas.version }
                            , toMsg = GotPublishResult
                            }
                        )

                    else
                        -- 通常バリデーション結果、または公開チェーンでバリデーション失敗
                        ( { canvas
                            | isValidating = False
                            , validationResult = Just validResult
                            , pendingPublish = False
                          }
                        , Cmd.none
                        )

                Err err ->
                    ( { canvas
                        | isValidating = False
                        , pendingPublish = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err)
                      }
                    , Cmd.none
                    )

        PublishClicked ->
            ( { canvas | pendingPublish = True, successMessage = Nothing, errorMessage = Nothing }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ConfirmPublish ->
            if canvas.isDirty_ then
                -- dirty なら先に保存
                let
                    definition =
                        DesignerCanvas.encodeDefinition canvas.steps canvas.transitions

                    body =
                        WorkflowDefinition.encodeUpdateRequest
                            { name = canvas.name
                            , description = canvas.description
                            , definition = definition
                            , version = canvas.version
                            }
                in
                ( { canvas | isSaving = True }
                , WorkflowDefinitionApi.updateDefinition
                    { config = Shared.toRequestConfig shared
                    , id = definitionId
                    , body = body
                    , toMsg = GotSaveResult
                    }
                )

            else
                -- dirty でなければ直接バリデーション
                let
                    definition =
                        DesignerCanvas.encodeDefinition canvas.steps canvas.transitions
                in
                ( { canvas | isValidating = True, validationResult = Nothing }
                , WorkflowDefinitionApi.validateDefinition
                    { config = Shared.toRequestConfig shared
                    , body = WorkflowDefinition.encodeValidationRequest { definition = definition }
                    , toMsg = GotValidationResult
                    }
                )

        CancelPublish ->
            ( { canvas | pendingPublish = False }
            , Cmd.none
            )

        GotPublishResult result ->
            case result of
                Ok def ->
                    let
                        ( cleanCanvas, cleanCmd ) =
                            DirtyState.clearDirty canvas
                    in
                    ( { cleanCanvas
                        | isPublishing = False
                        , pendingPublish = False
                        , version = def.version
                        , successMessage = Just "公開しました"
                        , errorMessage = Nothing
                      }
                    , cleanCmd
                    )

                Err err ->
                    ( { canvas
                        | isPublishing = False
                        , pendingPublish = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err)
                        , successMessage = Nothing
                      }
                    , Cmd.none
                    )

        DismissMessage ->
            ( { canvas | successMessage = Nothing, errorMessage = Nothing }
            , Cmd.none
            )

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


{-| 接続線端点の付け替えドロップ処理

ドロップ先のステップを判定し、有効な場合は transition の from/to を更新する。
trigger は維持する（付け替え時に自動変更しない）。

-}
handleReconnectionDrop : Int -> ReconnectEnd -> DesignerCanvas.Position -> CanvasState -> ( CanvasState, Cmd Msg )
handleReconnectionDrop index end mousePos canvas =
    case List.Extra.getAt index canvas.transitions of
        Just transition ->
            let
                -- 付け替えない側のステップ ID（自己ループ防止に使用）
                fixedStepId =
                    case end of
                        SourceEnd ->
                            transition.to

                        TargetEnd ->
                            transition.from

                -- ドロップ先のステップを判定（固定端と同じステップは除外 = 自己ループ防止）
                droppedStep =
                    canvas.steps
                        |> Dict.values
                        |> List.filter (\s -> s.id /= fixedStepId)
                        |> List.filter (DesignerCanvas.stepContainsPoint mousePos)
                        |> List.head
            in
            case droppedStep of
                Just target ->
                    let
                        updatedTransition =
                            case end of
                                SourceEnd ->
                                    { transition | from = target.id }

                                TargetEnd ->
                                    { transition | to = target.id }

                        ( dirtyCanvas, dirtyCmd ) =
                            DirtyState.markDirty canvas
                    in
                    ( { dirtyCanvas
                        | transitions = List.Extra.updateAt index (\_ -> updatedTransition) dirtyCanvas.transitions
                        , dragging = Nothing
                        , selectedTransitionIndex = Nothing
                      }
                    , dirtyCmd
                    )

                Nothing ->
                    ( { canvas | dragging = Nothing }
                    , Cmd.none
                    )

        Nothing ->
            ( { canvas | dragging = Nothing }
            , Cmd.none
            )
