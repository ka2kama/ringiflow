module Page.WorkflowDefinition.Designer exposing (Model, Msg(..), init, isDirty, subscriptions, update, updateShared, view)

{-| ワークフローデザイナー画面

SVG キャンバス上にワークフローのステップを配置・操作するビジュアルエディタ。
ADR-053 で決定した SVG + Elm 直接レンダリング方式に基づく。

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
import Data.DesignerCanvas as DesignerCanvas exposing (Bounds, DraggingState(..), StepNode, StepType(..), Transition, viewBoxHeight, viewBoxWidth)
import Data.WorkflowDefinition as WorkflowDefinition exposing (ValidationError, ValidationResult, WorkflowDefinition)
import Dict exposing (Dict)
import Form.DirtyState as DirtyState
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onMouseDown)
import Json.Decode as Decode
import Json.Encode as Encode
import Ports
import RemoteData exposing (RemoteData(..))
import Shared exposing (Shared)
import Svg exposing (svg)
import Svg.Attributes as SvgAttr
import Svg.Events



-- CONSTANTS


{-| キャンバス SVG 要素の HTML id
-}
canvasElementId : String
canvasElementId =
    "designer-canvas"



-- MODEL


type alias Model =
    { shared : Shared
    , definitionId : String
    , loadState : RemoteData ApiError WorkflowDefinition
    , steps : Dict String StepNode
    , transitions : List Transition
    , selectedStepId : Maybe String
    , selectedTransitionIndex : Maybe Int
    , dragging : Maybe DraggingState
    , canvasBounds : Maybe Bounds
    , nextStepNumber : Int
    , propertyName : String
    , propertyEndStatus : String
    , name : String
    , description : String
    , version : Int
    , isSaving : Bool
    , successMessage : Maybe String
    , errorMessage : Maybe String
    , isDirty_ : Bool
    , validationResult : Maybe ValidationResult
    , isValidating : Bool
    , isPublishing : Bool
    , pendingPublish : Bool
    }


init : Shared -> String -> ( Model, Cmd Msg )
init shared definitionId =
    ( { shared = shared
      , definitionId = definitionId
      , loadState = Loading
      , steps = Dict.empty
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
    , Cmd.batch
        [ Ports.requestCanvasBounds canvasElementId
        , WorkflowDefinitionApi.getDefinition
            { config = Shared.toRequestConfig shared
            , id = definitionId
            , toMsg = GotDefinition
            }
        ]
    )


isDirty : Model -> Bool
isDirty =
    DirtyState.isDirty


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = PaletteMouseDown StepType
    | CanvasMouseMove Float Float
    | CanvasMouseUp
    | StepClicked String
    | CanvasBackgroundClicked
    | StepMouseDown String Float Float -- stepId, clientX, clientY
    | ConnectionPortMouseDown String Float Float -- sourceStepId, clientX, clientY
    | TransitionClicked Int
    | UpdatePropertyName String
    | UpdatePropertyEndStatus String
    | UpdateDefinitionName String
    | SaveClicked
    | GotDefinition (Result ApiError WorkflowDefinition)
    | GotSaveResult (Result ApiError WorkflowDefinition)
    | ValidateClicked
    | GotValidationResult (Result ApiError ValidationResult)
    | PublishClicked
    | ConfirmPublish
    | CancelPublish
    | GotPublishResult (Result ApiError WorkflowDefinition)
    | DismissMessage
    | KeyDown String
    | GotCanvasBounds Encode.Value


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        PaletteMouseDown stepType ->
            ( { model
                | dragging = Just (DraggingNewStep stepType { x = 0, y = 0 })
              }
            , Ports.requestCanvasBounds canvasElementId
            )

        CanvasMouseMove clientX clientY ->
            case model.dragging of
                Just (DraggingNewStep stepType _) ->
                    case DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY of
                        Just canvasPos ->
                            ( { model
                                | dragging = Just (DraggingNewStep stepType canvasPos)
                              }
                            , Cmd.none
                            )

                        Nothing ->
                            ( model, Cmd.none )

                Just (DraggingExistingStep stepId offset) ->
                    case DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY of
                        Just canvasPos ->
                            let
                                newPos =
                                    { x = DesignerCanvas.snapToGrid (canvasPos.x - offset.x)
                                    , y = DesignerCanvas.snapToGrid (canvasPos.y - offset.y)
                                    }

                                updatedSteps =
                                    Dict.update stepId
                                        (Maybe.map (\step -> { step | position = newPos }))
                                        model.steps
                            in
                            ( { model | steps = updatedSteps }
                            , Cmd.none
                            )

                        Nothing ->
                            ( model, Cmd.none )

                Just (DraggingConnection sourceId _) ->
                    -- Phase 2 で接続線プレビュー更新を実装
                    case DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY of
                        Just canvasPos ->
                            ( { model | dragging = Just (DraggingConnection sourceId canvasPos) }
                            , Cmd.none
                            )

                        Nothing ->
                            ( model, Cmd.none )

                Nothing ->
                    ( model, Cmd.none )

        CanvasMouseUp ->
            case model.dragging of
                Just (DraggingNewStep stepType dropPos) ->
                    let
                        newStep =
                            DesignerCanvas.createStepFromDrop stepType model.nextStepNumber dropPos

                        ( dirtyModel, dirtyCmd ) =
                            DirtyState.markDirty model
                    in
                    ( { dirtyModel
                        | steps = Dict.insert newStep.id newStep dirtyModel.steps
                        , dragging = Nothing
                        , nextStepNumber = dirtyModel.nextStepNumber + 1
                        , selectedStepId = Just newStep.id
                      }
                    , dirtyCmd
                    )

                Just (DraggingExistingStep _ _) ->
                    let
                        ( dirtyModel, dirtyCmd ) =
                            DirtyState.markDirty model
                    in
                    ( { dirtyModel | dragging = Nothing }
                    , dirtyCmd
                    )

                Just (DraggingConnection sourceId mousePos) ->
                    let
                        -- ドロップ先のステップを判定
                        targetStep =
                            model.steps
                                |> Dict.values
                                |> List.filter (\s -> s.id /= sourceId)
                                |> List.filter (DesignerCanvas.stepContainsPoint mousePos)
                                |> List.head
                    in
                    case targetStep of
                        Just target ->
                            let
                                sourceStep =
                                    Dict.get sourceId model.steps

                                trigger =
                                    case sourceStep of
                                        Just src ->
                                            DesignerCanvas.autoTrigger src.stepType sourceId model.transitions

                                        Nothing ->
                                            Nothing

                                newTransition =
                                    { from = sourceId, to = target.id, trigger = trigger }

                                ( dirtyModel, dirtyCmd ) =
                                    DirtyState.markDirty model
                            in
                            ( { dirtyModel
                                | transitions = dirtyModel.transitions ++ [ newTransition ]
                                , dragging = Nothing
                              }
                            , dirtyCmd
                            )

                        Nothing ->
                            ( { model | dragging = Nothing }
                            , Cmd.none
                            )

                Nothing ->
                    ( model, Cmd.none )

        StepClicked stepId ->
            ( syncPropertyFields stepId { model | selectedStepId = Just stepId, selectedTransitionIndex = Nothing }
            , Cmd.none
            )

        CanvasBackgroundClicked ->
            ( { model | selectedStepId = Nothing, selectedTransitionIndex = Nothing, propertyName = "", propertyEndStatus = "" }
            , Cmd.none
            )

        ConnectionPortMouseDown sourceStepId clientX clientY ->
            case DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY of
                Just canvasPos ->
                    ( { model | dragging = Just (DraggingConnection sourceStepId canvasPos) }
                    , Cmd.none
                    )

                Nothing ->
                    ( model, Cmd.none )

        TransitionClicked index ->
            ( { model | selectedTransitionIndex = Just index, selectedStepId = Nothing, propertyName = "", propertyEndStatus = "" }
            , Cmd.none
            )

        UpdatePropertyName newName ->
            case model.selectedStepId of
                Just stepId ->
                    let
                        ( dirtyModel, dirtyCmd ) =
                            DirtyState.markDirty model
                    in
                    ( { dirtyModel
                        | propertyName = newName
                        , steps =
                            Dict.update stepId
                                (Maybe.map (\step -> { step | name = newName }))
                                dirtyModel.steps
                      }
                    , dirtyCmd
                    )

                Nothing ->
                    ( model, Cmd.none )

        UpdatePropertyEndStatus newStatus ->
            case model.selectedStepId of
                Just stepId ->
                    let
                        endStatus =
                            if newStatus == "" then
                                Nothing

                            else
                                Just newStatus

                        ( dirtyModel, dirtyCmd ) =
                            DirtyState.markDirty model
                    in
                    ( { dirtyModel
                        | propertyEndStatus = newStatus
                        , steps =
                            Dict.update stepId
                                (Maybe.map (\step -> { step | endStatus = endStatus }))
                                dirtyModel.steps
                      }
                    , dirtyCmd
                    )

                Nothing ->
                    ( model, Cmd.none )

        UpdateDefinitionName newName ->
            let
                ( dirtyModel, dirtyCmd ) =
                    DirtyState.markDirty model
            in
            ( { dirtyModel | name = newName }, dirtyCmd )

        SaveClicked ->
            let
                definition =
                    DesignerCanvas.encodeDefinition model.steps model.transitions

                body =
                    WorkflowDefinition.encodeUpdateRequest
                        { name = model.name
                        , description = model.description
                        , definition = definition
                        , version = model.version
                        }
            in
            ( { model | isSaving = True, successMessage = Nothing, errorMessage = Nothing }
            , WorkflowDefinitionApi.updateDefinition
                { config = Shared.toRequestConfig model.shared
                , id = model.definitionId
                , body = body
                , toMsg = GotSaveResult
                }
            )

        GotDefinition result ->
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
                        | loadState = Success def
                        , steps = steps
                        , transitions = transitions
                        , nextStepNumber = nextNumber
                        , name = def.name
                        , description = def.description |> Maybe.withDefault ""
                        , version = def.version
                      }
                    , Cmd.none
                    )

                Err err ->
                    ( { model | loadState = Failure err }
                    , Cmd.none
                    )

        GotSaveResult result ->
            case result of
                Ok def ->
                    let
                        ( cleanModel, cleanCmd ) =
                            DirtyState.clearDirty model
                    in
                    if model.pendingPublish then
                        -- 公開チェーン: 保存成功 → バリデーション
                        let
                            definition =
                                DesignerCanvas.encodeDefinition cleanModel.steps cleanModel.transitions
                        in
                        ( { cleanModel
                            | isSaving = False
                            , version = def.version
                            , isValidating = True
                            , validationResult = Nothing
                          }
                        , Cmd.batch
                            [ cleanCmd
                            , WorkflowDefinitionApi.validateDefinition
                                { config = Shared.toRequestConfig cleanModel.shared
                                , body = definition
                                , toMsg = GotValidationResult
                                }
                            ]
                        )

                    else
                        ( { cleanModel
                            | isSaving = False
                            , version = def.version
                            , successMessage = Just "保存しました"
                            , errorMessage = Nothing
                          }
                        , cleanCmd
                        )

                Err err ->
                    ( { model
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
                    DesignerCanvas.encodeDefinition model.steps model.transitions
            in
            ( { model | isValidating = True, validationResult = Nothing, errorMessage = Nothing }
            , WorkflowDefinitionApi.validateDefinition
                { config = Shared.toRequestConfig model.shared
                , body = definition
                , toMsg = GotValidationResult
                }
            )

        GotValidationResult result ->
            case result of
                Ok validResult ->
                    if model.pendingPublish && validResult.valid then
                        -- 公開チェーン: バリデーション成功 → 公開 API 呼び出し
                        ( { model
                            | isValidating = False
                            , validationResult = Just validResult
                            , isPublishing = True
                          }
                        , WorkflowDefinitionApi.publishDefinition
                            { config = Shared.toRequestConfig model.shared
                            , id = model.definitionId
                            , body = WorkflowDefinition.encodeVersionRequest { version = model.version }
                            , toMsg = GotPublishResult
                            }
                        )

                    else
                        -- 通常バリデーション結果、または公開チェーンでバリデーション失敗
                        ( { model
                            | isValidating = False
                            , validationResult = Just validResult
                            , pendingPublish = False
                          }
                        , Cmd.none
                        )

                Err err ->
                    ( { model
                        | isValidating = False
                        , pendingPublish = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err)
                      }
                    , Cmd.none
                    )

        PublishClicked ->
            ( { model | pendingPublish = True, successMessage = Nothing, errorMessage = Nothing }
            , Ports.showModalDialog "designer-publish-dialog"
            )

        ConfirmPublish ->
            if model.isDirty_ then
                -- dirty なら先に保存
                let
                    definition =
                        DesignerCanvas.encodeDefinition model.steps model.transitions

                    body =
                        WorkflowDefinition.encodeUpdateRequest
                            { name = model.name
                            , description = model.description
                            , definition = definition
                            , version = model.version
                            }
                in
                ( { model | isSaving = True }
                , WorkflowDefinitionApi.updateDefinition
                    { config = Shared.toRequestConfig model.shared
                    , id = model.definitionId
                    , body = body
                    , toMsg = GotSaveResult
                    }
                )

            else
                -- dirty でなければ直接バリデーション
                let
                    definition =
                        DesignerCanvas.encodeDefinition model.steps model.transitions
                in
                ( { model | isValidating = True, validationResult = Nothing }
                , WorkflowDefinitionApi.validateDefinition
                    { config = Shared.toRequestConfig model.shared
                    , body = definition
                    , toMsg = GotValidationResult
                    }
                )

        CancelPublish ->
            ( { model | pendingPublish = False }
            , Cmd.none
            )

        GotPublishResult result ->
            case result of
                Ok def ->
                    let
                        ( cleanModel, cleanCmd ) =
                            DirtyState.clearDirty model
                    in
                    ( { cleanModel
                        | isPublishing = False
                        , pendingPublish = False
                        , version = def.version
                        , successMessage = Just "公開しました"
                        , errorMessage = Nothing
                      }
                    , cleanCmd
                    )

                Err err ->
                    ( { model
                        | isPublishing = False
                        , pendingPublish = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err)
                        , successMessage = Nothing
                      }
                    , Cmd.none
                    )

        DismissMessage ->
            ( { model | successMessage = Nothing, errorMessage = Nothing }
            , Cmd.none
            )

        StepMouseDown stepId clientX clientY ->
            case ( Dict.get stepId model.steps, DesignerCanvas.clientToCanvas model.canvasBounds clientX clientY ) of
                ( Just step, Just canvasPos ) ->
                    let
                        offset =
                            { x = canvasPos.x - step.position.x
                            , y = canvasPos.y - step.position.y
                            }
                    in
                    ( syncPropertyFields stepId
                        { model
                            | dragging = Just (DraggingExistingStep stepId offset)
                            , selectedStepId = Just stepId
                        }
                    , Cmd.none
                    )

                _ ->
                    ( syncPropertyFields stepId { model | selectedStepId = Just stepId }
                    , Cmd.none
                    )

        KeyDown key ->
            if key == "Delete" || key == "Backspace" then
                case ( model.selectedTransitionIndex, model.selectedStepId ) of
                    ( Just index, _ ) ->
                        -- 選択中の接続線を削除
                        let
                            ( dirtyModel, dirtyCmd ) =
                                DirtyState.markDirty model
                        in
                        ( { dirtyModel
                            | transitions = removeAt index dirtyModel.transitions
                            , selectedTransitionIndex = Nothing
                          }
                        , dirtyCmd
                        )

                    ( Nothing, Just stepId ) ->
                        -- 選択中のステップと関連 transitions を削除
                        let
                            ( dirtyModel, dirtyCmd ) =
                                DirtyState.markDirty model
                        in
                        ( { dirtyModel
                            | steps = Dict.remove stepId dirtyModel.steps
                            , transitions =
                                List.filter
                                    (\t -> t.from /= stepId && t.to /= stepId)
                                    dirtyModel.transitions
                            , selectedStepId = Nothing
                          }
                        , dirtyCmd
                        )

                    _ ->
                        ( model, Cmd.none )

            else
                ( model, Cmd.none )

        GotCanvasBounds value ->
            case DesignerCanvas.decodeBounds value of
                Ok bounds ->
                    ( { model | canvasBounds = Just bounds }
                    , Cmd.none
                    )

                Err _ ->
                    ( model, Cmd.none )


{-| 選択されたステップのプロパティをフォームフィールドに同期する
-}
syncPropertyFields : String -> Model -> Model
syncPropertyFields stepId model =
    case Dict.get stepId model.steps of
        Just step ->
            { model
                | propertyName = step.name
                , propertyEndStatus = step.endStatus |> Maybe.withDefault ""
            }

        Nothing ->
            model


{-| リストの指定インデックスの要素を除去する
-}
removeAt : Int -> List a -> List a
removeAt index list =
    List.take index list ++ List.drop (index + 1) list



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch
        [ Ports.receiveCanvasBounds GotCanvasBounds
        , if model.dragging /= Nothing then
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



-- VIEW


view : Model -> Html Msg
view model =
    case model.loadState of
        Loading ->
            div [ class "flex items-center justify-center", style "height" "calc(100vh - 8rem)" ]
                [ LoadingSpinner.view ]

        Failure err ->
            div [ class "flex items-center justify-center", style "height" "calc(100vh - 8rem)" ]
                [ ErrorState.viewSimple (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err) ]

        _ ->
            div [ class "flex flex-col", style "height" "calc(100vh - 8rem)" ]
                [ viewToolbar model
                , viewMessages model
                , div [ class "flex flex-1 overflow-hidden" ]
                    [ viewPalette
                    , viewCanvasArea model
                    , viewPropertyPanel model
                    ]
                , viewValidationPanel model
                , viewStatusBar model
                , viewPublishDialog model
                ]


{-| ツールバー（定義名 + バリデーション・保存・公開ボタン）
-}
viewToolbar : Model -> Html Msg
viewToolbar model =
    div [ class "flex items-center gap-4 border-b border-secondary-200 bg-white px-4 py-2" ]
        [ h1 [ class "shrink-0 text-base font-semibold text-secondary-800" ]
            [ text "ワークフローデザイナー" ]
        , input
            [ type_ "text"
            , class "min-w-0 flex-1 rounded border border-secondary-300 px-3 py-1 text-sm focus:border-primary-500 focus:ring-1 focus:ring-primary-500"
            , value model.name
            , Html.Events.onInput UpdateDefinitionName
            , placeholder "定義名を入力"
            ]
            []
        , div [ class "flex shrink-0 gap-2" ]
            [ Button.view
                { variant = Button.Outline
                , disabled = model.isValidating
                , onClick = ValidateClicked
                }
                [ text
                    (if model.isValidating then
                        "検証中..."

                     else
                        "検証"
                    )
                ]
            , Button.view
                { variant = Button.Primary
                , disabled = model.isSaving || not model.isDirty_
                , onClick = SaveClicked
                }
                [ text
                    (if model.isSaving then
                        "保存中..."

                     else
                        "保存"
                    )
                ]
            , Button.view
                { variant = Button.Success
                , disabled = model.isPublishing || model.isSaving || model.isValidating
                , onClick = PublishClicked
                }
                [ text
                    (if model.isPublishing then
                        "公開中..."

                     else
                        "公開"
                    )
                ]
            ]
        ]


{-| 成功・エラーメッセージ表示
-}
viewMessages : Model -> Html Msg
viewMessages model =
    MessageAlert.view
        { onDismiss = DismissMessage
        , successMessage = model.successMessage
        , errorMessage = model.errorMessage
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
viewCanvasArea : Model -> Html Msg
viewCanvasArea model =
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
            , viewTransitions model
            , viewConnectionDragPreview model
            , viewSteps model
            , viewDragPreview model
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
viewSteps : Model -> Svg.Svg Msg
viewSteps model =
    let
        errorStepIds =
            model.validationResult
                |> Maybe.map .errors
                |> Maybe.withDefault []
                |> List.filterMap .stepId
    in
    Svg.g []
        (model.steps
            |> Dict.values
            |> List.map (viewStepNode model.selectedStepId errorStepIds)
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
            , SvgAttr.rx "8"
            , SvgAttr.fill colors.fill
            , SvgAttr.stroke strokeColor
            , SvgAttr.strokeWidth strokeWidth
            ]
            []

        -- 選択ハイライト（外枠リング）
        , if isSelected then
            Svg.rect
                [ SvgAttr.x "-4"
                , SvgAttr.y "-4"
                , SvgAttr.width (String.fromFloat (dim.width + 8))
                , SvgAttr.height (String.fromFloat (dim.height + 8))
                , SvgAttr.rx "12"
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
            , SvgAttr.fontSize "13"
            , SvgAttr.fontWeight "500"
            , SvgAttr.class "pointer-events-none select-none"
            ]
            [ Svg.text step.name ]

        -- 出力ポート（右端中央の円）
        , Svg.circle
            [ SvgAttr.cx (String.fromFloat dim.width)
            , SvgAttr.cy (String.fromFloat (dim.height / 2))
            , SvgAttr.r "5"
            , SvgAttr.fill colors.stroke
            , SvgAttr.stroke "white"
            , SvgAttr.strokeWidth "1.5"
            , SvgAttr.class "cursor-crosshair"
            , Html.Events.stopPropagationOn "mousedown"
                (Decode.map2 (\cx cy -> ( ConnectionPortMouseDown step.id cx cy, True ))
                    (Decode.field "clientX" Decode.float)
                    (Decode.field "clientY" Decode.float)
                )
            ]
            []

        -- 入力ポート（左端中央の円）
        , Svg.circle
            [ SvgAttr.cx "0"
            , SvgAttr.cy (String.fromFloat (dim.height / 2))
            , SvgAttr.r "5"
            , SvgAttr.fill colors.stroke
            , SvgAttr.stroke "white"
            , SvgAttr.strokeWidth "1.5"
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
        , SvgAttr.markerWidth "8"
        , SvgAttr.markerHeight "8"
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
viewTransitions : Model -> Svg.Svg Msg
viewTransitions model =
    Svg.g []
        (model.transitions
            |> List.indexedMap (viewTransitionLine model)
        )


{-| 個別の接続線描画
-}
viewTransitionLine : Model -> Int -> Transition -> Svg.Svg Msg
viewTransitionLine model index transition =
    let
        fromStep =
            Dict.get transition.from model.steps

        toStep =
            Dict.get transition.to model.steps
    in
    case ( fromStep, toStep ) of
        ( Just from, Just to ) ->
            let
                startPos =
                    DesignerCanvas.stepOutputPortPosition from

                endPos =
                    DesignerCanvas.stepInputPortPosition to

                -- ベジェ曲線の制御点（水平方向に 1/3 オフセット）
                dx =
                    abs (endPos.x - startPos.x) / 3

                pathData =
                    "M "
                        ++ String.fromFloat startPos.x
                        ++ " "
                        ++ String.fromFloat startPos.y
                        ++ " C "
                        ++ String.fromFloat (startPos.x + dx)
                        ++ " "
                        ++ String.fromFloat startPos.y
                        ++ ", "
                        ++ String.fromFloat (endPos.x - dx)
                        ++ " "
                        ++ String.fromFloat endPos.y
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
                    model.selectedTransitionIndex == Just index

                strokeWidth =
                    if isSelected then
                        "3"

                    else
                        "2"
            in
            Svg.g []
                [ -- クリック判定用の透明な太いパス
                  Svg.path
                    [ SvgAttr.d pathData
                    , SvgAttr.fill "none"
                    , SvgAttr.stroke "transparent"
                    , SvgAttr.strokeWidth "12"
                    , SvgAttr.class "cursor-pointer"
                    , Svg.Events.onClick (TransitionClicked index)
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
                ]

        _ ->
            Svg.text ""


{-| 接続線ドラッグ中のプレビュー

DraggingConnection 中、接続元の出力ポートから現在のマウス位置まで破線を描画する。

-}
viewConnectionDragPreview : Model -> Svg.Svg Msg
viewConnectionDragPreview model =
    case model.dragging of
        Just (DraggingConnection sourceId mousePos) ->
            case Dict.get sourceId model.steps of
                Just sourceStep ->
                    let
                        startPos =
                            DesignerCanvas.stepOutputPortPosition sourceStep

                        dx =
                            abs (mousePos.x - startPos.x) / 3

                        pathData =
                            "M "
                                ++ String.fromFloat startPos.x
                                ++ " "
                                ++ String.fromFloat startPos.y
                                ++ " C "
                                ++ String.fromFloat (startPos.x + dx)
                                ++ " "
                                ++ String.fromFloat startPos.y
                                ++ ", "
                                ++ String.fromFloat (mousePos.x - dx)
                                ++ " "
                                ++ String.fromFloat mousePos.y
                                ++ ", "
                                ++ String.fromFloat mousePos.x
                                ++ " "
                                ++ String.fromFloat mousePos.y
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

                Nothing ->
                    Svg.text ""

        _ ->
            Svg.text ""


{-| ドラッグ中のプレビュー表示

パレットからの新規配置時、マウス位置にゴーストステップを表示する。

-}
viewDragPreview : Model -> Svg.Svg Msg
viewDragPreview model =
    case model.dragging of
        Just (DraggingNewStep stepType pos) ->
            let
                colors =
                    DesignerCanvas.stepColors stepType

                dim =
                    DesignerCanvas.stepDimensions

                snappedX =
                    DesignerCanvas.snapToGrid pos.x

                snappedY =
                    DesignerCanvas.snapToGrid pos.y
            in
            Svg.g
                [ SvgAttr.transform
                    ("translate("
                        ++ String.fromFloat snappedX
                        ++ ","
                        ++ String.fromFloat snappedY
                        ++ ")"
                    )
                , SvgAttr.opacity "0.6"
                , SvgAttr.class "pointer-events-none"
                ]
                [ Svg.rect
                    [ SvgAttr.width (String.fromFloat dim.width)
                    , SvgAttr.height (String.fromFloat dim.height)
                    , SvgAttr.rx "8"
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
                    , SvgAttr.fontSize "13"
                    , SvgAttr.fontWeight "500"
                    ]
                    [ Svg.text (DesignerCanvas.defaultStepName stepType) ]
                ]

        _ ->
            Svg.text ""


{-| プロパティパネル（右サイドバー）

選択中のステップの属性を編集するパネル。
ステップ種別に応じて表示するフィールドが変わる:

  - Start: ステップ名
  - Approval: ステップ名 + 承認者指定方式（読み取り専用）
  - End: ステップ名 + 終了ステータス

-}
viewPropertyPanel : Model -> Html Msg
viewPropertyPanel model =
    div [ class "w-64 shrink-0 border-l border-secondary-200 bg-white p-4 overflow-y-auto" ]
        [ h2 [ class "mb-3 text-xs font-semibold uppercase tracking-wider text-secondary-500" ]
            [ text "プロパティ" ]
        , case model.selectedStepId of
            Nothing ->
                p [ class "text-sm text-secondary-400" ]
                    [ text "ステップを選択してください" ]

            Just stepId ->
                case Dict.get stepId model.steps of
                    Just step ->
                        viewStepProperties model step

                    Nothing ->
                        p [ class "text-sm text-secondary-400" ]
                            [ text "ステップを選択してください" ]
        ]


{-| ステップ種別に応じたプロパティフィールド
-}
viewStepProperties : Model -> StepNode -> Html Msg
viewStepProperties model step =
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
            , value = model.propertyName
            , onInput = UpdatePropertyName
            , error = Nothing
            , inputType = "text"
            , placeholder = "ステップ名を入力"
            }
         ]
            ++ viewStepTypeSpecificFields model step
        )


{-| ステップ種別固有のフィールド
-}
viewStepTypeSpecificFields : Model -> StepNode -> List (Html Msg)
viewStepTypeSpecificFields model step =
    case step.stepType of
        Start ->
            []

        Approval ->
            [ FormField.viewReadOnlyField "承認者指定" "申請時にユーザーを選択" ]

        End ->
            [ FormField.viewSelectField
                { label = "終了ステータス"
                , value = model.propertyEndStatus
                , onInput = UpdatePropertyEndStatus
                , error = Nothing
                , options =
                    [ { value = "approved", label = "承認" }
                    , { value = "rejected", label = "却下" }
                    ]
                , placeholder = "選択してください"
                }
            ]


{-| バリデーション結果パネル

キャンバス下部に表示。valid なら緑、invalid ならエラー一覧。

-}
viewValidationPanel : Model -> Html Msg
viewValidationPanel model =
    case model.validationResult of
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
viewStatusBar : Model -> Html Msg
viewStatusBar model =
    div [ class "border-t border-secondary-200 bg-white px-4 py-1.5 text-xs text-secondary-500" ]
        [ text
            (String.fromInt (Dict.size model.steps)
                ++ " ステップ / "
                ++ String.fromInt (List.length model.transitions)
                ++ " 接続"
            )
        ]


{-| 公開確認ダイアログ
-}
viewPublishDialog : Model -> Html Msg
viewPublishDialog model =
    if model.pendingPublish then
        ConfirmDialog.view
            { title = "ワークフロー定義を公開"
            , message = "「" ++ model.name ++ "」を公開しますか？公開後はユーザーが申請に使用できるようになります。"
            , confirmLabel = "公開する"
            , cancelLabel = "キャンセル"
            , onConfirm = ConfirmPublish
            , onCancel = CancelPublish
            , actionStyle = ConfirmDialog.Positive
            }

    else
        text ""
