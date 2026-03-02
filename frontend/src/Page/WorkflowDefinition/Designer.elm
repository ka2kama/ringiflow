module Page.WorkflowDefinition.Designer exposing (init, isDirty, subscriptions, update, updateShared, view)

{-| ワークフローデザイナー画面

SVG キャンバス上にワークフローのステップを配置・操作するビジュアルエディタ。
ADR-053 で決定した SVG + Elm 直接レンダリング方式に基づく。

Model は型安全ステートマシンで管理する（ADR-054）。
Loading 中はキャンバス関連フィールドが型レベルで存在しないため、
不正な状態（Loading 中のキャンバス操作）を表現不可能にしている。

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
import Data.DesignerCanvas as DesignerCanvas exposing (StepNode, StepType(..), Transition)
import Data.WorkflowDefinition as WorkflowDefinition exposing (ValidationError, ValidationResult, WorkflowDefinition)
import Dict exposing (Dict)
import Form.DirtyState as DirtyState
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onMouseDown)
import Json.Decode as Decode
import Json.Encode as Encode
import List.Extra
import Maybe.Extra
import Page.WorkflowDefinition.Designer.Canvas as Canvas
import Page.WorkflowDefinition.Designer.Palette as Palette
import Page.WorkflowDefinition.Designer.PropertyPanel as PropertyPanel
import Page.WorkflowDefinition.Designer.Types as Types exposing (CanvasState, Model, Msg(..), PageState(..), canvasElementId)
import Page.WorkflowDefinition.Designer.Update as DesignerUpdate
import Ports
import Shared exposing (Shared)
import Svg exposing (svg)
import Svg.Attributes as SvgAttr


init : Shared -> String -> ( Model, Cmd Msg )
init shared definitionId =
    ( { shared = shared
      , definitionId = definitionId
      , state = Loading
      }
    , WorkflowDefinitionApi.getDefinition
        { config = Shared.toRequestConfig shared
        , id = definitionId
        , toMsg = GotDefinition
        }
    )


isDirty : Model -> Bool
isDirty model =
    case model.state of
        Loaded canvas ->
            DirtyState.isDirty canvas

        _ ->
            False


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotDefinition result ->
            DesignerUpdate.handleGotDefinition result model

        _ ->
            case model.state of
                Loaded canvas ->
                    let
                        ( newCanvas, cmd ) =
                            DesignerUpdate.updateLoaded msg model.shared model.definitionId canvas
                    in
                    ( { model | state = Loaded newCanvas }, cmd )

                _ ->
                    ( model, Cmd.none )



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch
        [ Ports.receiveCanvasBounds GotCanvasBounds
        , case model.state of
            Loaded canvas ->
                Sub.batch
                    [ if canvas.dragging /= Nothing then
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

            _ ->
                Sub.none
        ]



-- VIEW


view : Model -> Html Msg
view model =
    case model.state of
        Loading ->
            div [ class "flex items-center justify-center", style "height" "calc(100vh - 8rem)" ]
                [ LoadingSpinner.view ]

        Failed err ->
            div [ class "flex items-center justify-center", style "height" "calc(100vh - 8rem)" ]
                [ ErrorState.viewSimple (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } err) ]

        Loaded canvas ->
            viewLoaded canvas


{-| Loaded 状態の view
-}
viewLoaded : CanvasState -> Html Msg
viewLoaded canvas =
    div [ class "flex flex-col", style "height" "calc(100vh - 8rem)" ]
        [ viewToolbar canvas
        , viewMessages canvas
        , div [ class "flex flex-1 overflow-hidden" ]
            [ Palette.viewPalette
            , Canvas.viewCanvasArea canvas
            , PropertyPanel.viewPropertyPanel canvas
            ]
        , viewValidationPanel canvas
        , viewStatusBar canvas
        , viewPublishDialog canvas
        ]


{-| ツールバー（定義名 + バリデーション・保存・公開ボタン）
-}
viewToolbar : CanvasState -> Html Msg
viewToolbar canvas =
    div [ class "flex items-center gap-4 border-b border-secondary-200 bg-white px-4 py-2" ]
        [ h1 [ class "shrink-0 text-base font-semibold text-secondary-800" ]
            [ text "ワークフローデザイナー" ]
        , input
            [ type_ "text"
            , class "min-w-0 flex-1 rounded border border-secondary-300 px-3 py-1 text-sm focus:border-primary-500 focus:ring-1 focus:ring-primary-500"
            , value canvas.name
            , Html.Events.onInput UpdateDefinitionName
            , placeholder "定義名を入力"
            ]
            []
        , div [ class "flex shrink-0 gap-2" ]
            [ Button.view
                { variant = Button.Outline
                , disabled = canvas.isValidating
                , onClick = ValidateClicked
                }
                [ text
                    (if canvas.isValidating then
                        "検証中..."

                     else
                        "検証"
                    )
                ]
            , Button.view
                { variant = Button.Primary
                , disabled = canvas.isSaving || not canvas.isDirty_
                , onClick = SaveClicked
                }
                [ text
                    (if canvas.isSaving then
                        "保存中..."

                     else
                        "保存"
                    )
                ]
            , Button.view
                { variant = Button.Success
                , disabled = canvas.isPublishing || canvas.isSaving || canvas.isValidating
                , onClick = PublishClicked
                }
                [ text
                    (if canvas.isPublishing then
                        "公開中..."

                     else
                        "公開"
                    )
                ]
            ]
        ]


{-| 成功・エラーメッセージ表示
-}
viewMessages : CanvasState -> Html Msg
viewMessages canvas =
    MessageAlert.view
        { onDismiss = DismissMessage
        , successMessage = canvas.successMessage
        , errorMessage = canvas.errorMessage
        }


{-| バリデーション結果パネル

キャンバス下部に表示。valid なら緑、invalid ならエラー一覧。

-}
viewValidationPanel : CanvasState -> Html Msg
viewValidationPanel canvas =
    case canvas.validationResult of
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
viewStatusBar : CanvasState -> Html Msg
viewStatusBar canvas =
    div [ class "border-t border-secondary-200 bg-white px-4 py-1.5 text-xs text-secondary-500" ]
        [ text
            (String.fromInt (Dict.size canvas.steps)
                ++ " ステップ / "
                ++ String.fromInt (List.length canvas.transitions)
                ++ " 接続"
            )
        ]


{-| 公開確認ダイアログ
-}
viewPublishDialog : CanvasState -> Html Msg
viewPublishDialog canvas =
    if canvas.pendingPublish then
        ConfirmDialog.view
            { title = "ワークフロー定義を公開"
            , message = "「" ++ canvas.name ++ "」を公開しますか？公開後はユーザーが申請に使用できるようになります。"
            , confirmLabel = "公開する"
            , cancelLabel = "キャンセル"
            , onConfirm = ConfirmPublish
            , onCancel = CancelPublish
            , actionStyle = ConfirmDialog.Positive
            }

    else
        text ""
