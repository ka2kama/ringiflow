module Page.Task.List exposing (Model, Msg, init, update, updateShared, view)

{-| タスク一覧ページ（スタブ）

Phase 6 で本実装に置き換える。

-}

import Html exposing (..)
import Shared exposing (Shared)


type alias Model =
    { shared : Shared
    }


type Msg
    = NoOp


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared }
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
        [ h2 [] [ text "タスク一覧" ]
        , p [] [ text "読み込み中..." ]
        ]
