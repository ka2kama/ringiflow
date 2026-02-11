module Page.Role.Edit exposing (Model, Msg, init, update, updateShared, view)

{-| ロール編集画面（スタブ）

Phase 4 で実装予定。DJ-3: 詳細と編集を統合。

-}

import Html exposing (..)
import Html.Attributes exposing (..)
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , roleId : String
    }


init : Shared -> String -> ( Model, Cmd Msg )
init shared roleId =
    ( { shared = shared
      , roleId = roleId
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
            [ text ("ロール編集: " ++ model.roleId) ]
        , p [ class "text-secondary-500" ]
            [ text "準備中です。" ]
        ]
