module Page.WorkflowDefinition.Designer.UpdatePersistence exposing
    ( handleCancelPublish
    , handleConfirmPublish
    , handleDismissMessage
    , handlePublishClicked
    , handlePublishResult
    , handleReconnectionDrop
    , handleSave
    , handleSaveResult
    , handleValidate
    , handleValidationResult
    )

{-| Designer ページの永続化ハンドラ + 接続線付け替えヘルパー

Save → Validate → Publish の連鎖的ワークフローと、
接続線端点の付け替え処理を提供する。

永続化チェーンは pendingPublish フラグで制御される:

  - 通常保存: SaveClicked → GotSaveResult（完了）
  - 公開フロー: PublishClicked → ConfirmPublish → SaveClicked → GotSaveResult → ValidateClicked → GotValidationResult → GotPublishResult

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Component.ConfirmDialog as ConfirmDialog
import Data.DesignerCanvas as DesignerCanvas exposing (ReconnectEnd(..))
import Data.WorkflowDefinition as WorkflowDefinition exposing (ValidationResult, WorkflowDefinition)
import Dict
import Form.DirtyState as DirtyState
import List.Extra
import Page.WorkflowDefinition.Designer.Types exposing (CanvasState, Msg(..))
import Ports
import Shared exposing (Shared)



-- 永続化ハンドラ


{-| 保存ボタンクリック
-}
handleSave : Shared -> String -> CanvasState -> ( CanvasState, Cmd Msg )
handleSave shared definitionId canvas =
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


{-| 保存結果の処理

pendingPublish が True の場合、バリデーション API を呼び出して公開チェーンを継続する。

-}
handleSaveResult : Shared -> String -> Result ApiError WorkflowDefinition -> CanvasState -> ( CanvasState, Cmd Msg )
handleSaveResult shared definitionId result canvas =
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


{-| バリデーション実行
-}
handleValidate : Shared -> CanvasState -> ( CanvasState, Cmd Msg )
handleValidate shared canvas =
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


{-| バリデーション結果の処理

pendingPublish が True かつバリデーション成功の場合、公開 API を呼び出す。

-}
handleValidationResult : Shared -> String -> Result ApiError ValidationResult -> CanvasState -> ( CanvasState, Cmd Msg )
handleValidationResult shared definitionId result canvas =
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


{-| 公開ボタンクリック（確認ダイアログ表示）
-}
handlePublishClicked : CanvasState -> ( CanvasState, Cmd Msg )
handlePublishClicked canvas =
    ( { canvas | pendingPublish = True, successMessage = Nothing, errorMessage = Nothing }
    , Ports.showModalDialog ConfirmDialog.dialogId
    )


{-| 公開確認

dirty なら先に保存、そうでなければ直接バリデーションへ。

-}
handleConfirmPublish : Shared -> String -> CanvasState -> ( CanvasState, Cmd Msg )
handleConfirmPublish shared definitionId canvas =
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


{-| 公開キャンセル
-}
handleCancelPublish : CanvasState -> ( CanvasState, Cmd Msg )
handleCancelPublish canvas =
    ( { canvas | pendingPublish = False }
    , Cmd.none
    )


{-| 公開結果の処理
-}
handlePublishResult : Result ApiError WorkflowDefinition -> CanvasState -> ( CanvasState, Cmd Msg )
handlePublishResult result canvas =
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


{-| メッセージ非表示
-}
handleDismissMessage : CanvasState -> ( CanvasState, Cmd Msg )
handleDismissMessage canvas =
    ( { canvas | successMessage = Nothing, errorMessage = Nothing }
    , Cmd.none
    )



-- キャンバスヘルパー


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
