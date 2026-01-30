module Page.Task.Detail exposing (Model, Msg, init, update, updateShared, view)

{-| タスク詳細ページ（スタブ）

Phase 7 で本実装に置き換える。

-}

import Html exposing (..)
import Shared exposing (Shared)


type alias Model =
    { shared : Shared
    , taskId : String
    }


type Msg
    = NoOp


init : Shared -> String -> ( Model, Cmd Msg )
init shared taskId =
    ( { shared = shared
      , taskId = taskId
      }
    , Cmd.none
    )


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        NoOp ->
            ( model, Cmd.none )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }


view : Model -> Html Msg
view _ =
    div []
        [ h2 [] [ text "タスク詳細" ]
        , p [] [ text "読み込み中..." ]
        ]
