module Page.User.New exposing (Model, Msg, init, update, updateShared, view)

{-| ユーザー作成画面（スタブ）

Phase 3 で実装予定。

-}

import Html exposing (..)
import Html.Attributes exposing (..)
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    }


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared }
    , Cmd.none
    )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = NoOp


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        NoOp ->
            ( model, Cmd.none )



-- VIEW


view : Model -> Html Msg
view _ =
    div [ class "py-12 text-center" ]
        [ h2 [ class "mb-4 text-2xl font-bold text-secondary-900" ]
            [ text "ユーザー作成" ]
        , p [ class "text-secondary-500" ]
            [ text "準備中です。" ]
        ]
