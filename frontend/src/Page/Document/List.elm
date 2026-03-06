module Page.Document.List exposing (Model, Msg, init, subscriptions, update, updateShared, view)

{-| ドキュメント管理画面

フォルダツリー + ファイル一覧のレイアウト。
フォルダ選択でファイル一覧が切り替わる。

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.Folder as FolderApi
import Component.ErrorState as ErrorState
import Component.LoadingSpinner as LoadingSpinner
import Data.Folder exposing (Folder)
import Html exposing (..)
import Html.Attributes exposing (..)
import RemoteData exposing (RemoteData(..))
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , folders : RemoteData ApiError (List Folder)
    }


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , folders = Loading
      }
    , FolderApi.listFolders
        { config = Shared.toRequestConfig shared
        , toMsg = GotFolders
        }
    )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = GotFolders (Result ApiError (List Folder))
    | Refresh


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotFolders result ->
            case result of
                Ok folders ->
                    ( { model | folders = Success folders }, Cmd.none )

                Err err ->
                    ( { model | folders = Failure err }, Cmd.none )

        Refresh ->
            ( { model | folders = Loading }
            , FolderApi.listFolders
                { config = Shared.toRequestConfig model.shared
                , toMsg = GotFolders
                }
            )



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions _ =
    Sub.none



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewHeader
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "mb-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ]
            [ text "ドキュメント管理" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.folders of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure err ->
            ErrorState.view
                { message = ErrorMessage.toUserMessage { entityName = "フォルダ" } err
                , onRefresh = Refresh
                }

        Success _ ->
            div [ class "text-sm text-secondary-500" ]
                [ text "フォルダツリーとファイル一覧をここに表示（Phase 3 で実装）" ]
