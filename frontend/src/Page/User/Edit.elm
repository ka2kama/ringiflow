module Page.User.Edit exposing (Model, Msg, init, update, updateShared, view)

{-| ユーザー編集画面（スタブ）

Phase 3 で実装予定。

-}

import Html exposing (..)
import Html.Attributes exposing (..)
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , displayNumber : Int
    }


init : Shared -> Int -> ( Model, Cmd Msg )
init shared displayNumber =
    ( { shared = shared
      , displayNumber = displayNumber
      }
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
view model =
    div [ class "py-12 text-center" ]
        [ h2 [ class "mb-4 text-2xl font-bold text-secondary-900" ]
            [ text ("ユーザー編集: #" ++ String.fromInt model.displayNumber) ]
        , p [ class "text-secondary-500" ]
            [ text "準備中です。" ]
        ]
