module Page.WorkflowDefinition.Designer exposing (init, isDirty, subscriptions, update, updateShared, view)

{-| ワークフローデザイナー画面

SVG キャンバス上にワークフローのステップを配置・操作するビジュアルエディタ。
ADR-053 で決定した SVG + Elm 直接レンダリング方式に基づく。

Model は型安全ステートマシンで管理する（ADR-054）。
Loading 中はキャンバス関連フィールドが型レベルで存在しないため、
不正な状態（Loading 中のキャンバス操作）を表現不可能にしている。

-}

import Api.ErrorMessage as ErrorMessage
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Browser.Events
import Component.ErrorState as ErrorState
import Component.LoadingSpinner as LoadingSpinner
import Form.DirtyState as DirtyState
import Html exposing (..)
import Html.Attributes exposing (..)
import Json.Decode as Decode
import Page.WorkflowDefinition.Designer.Canvas as Canvas
import Page.WorkflowDefinition.Designer.Palette as Palette
import Page.WorkflowDefinition.Designer.PropertyPanel as PropertyPanel
import Page.WorkflowDefinition.Designer.Toolbar as Toolbar
import Page.WorkflowDefinition.Designer.Types as Types exposing (CanvasState, Model, Msg(..), PageState(..), canvasElementId)
import Page.WorkflowDefinition.Designer.Update as DesignerUpdate
import Ports
import Shared exposing (Shared)


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
        [ Toolbar.viewToolbar canvas
        , Toolbar.viewMessages canvas
        , div [ class "flex flex-1 overflow-hidden" ]
            [ Palette.viewPalette
            , Canvas.viewCanvasArea canvas
            , PropertyPanel.viewPropertyPanel canvas
            ]
        , Toolbar.viewValidationPanel canvas
        , Toolbar.viewStatusBar canvas
        , Toolbar.viewPublishDialog canvas
        ]
