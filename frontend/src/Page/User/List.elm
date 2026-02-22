module Page.User.List exposing (Model, Msg, init, update, updateShared, view)

{-| ユーザー一覧画面

テナント内のユーザー一覧を表示する管理画面。
ステータスフィルタ（すべて / アクティブ / 非アクティブ）に対応。

-}

import Api exposing (ApiError)
import Api.AdminUser as AdminUserApi
import Api.ErrorMessage as ErrorMessage
import Component.Badge as Badge
import Component.Button as Button
import Component.EmptyState as EmptyState
import Component.ErrorState as ErrorState
import Component.LoadingSpinner as LoadingSpinner
import Data.AdminUser as AdminUser exposing (AdminUserItem)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , users : RemoteData ApiError (List AdminUserItem)
    , statusFilter : Maybe String
    }


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , users = Loading
      , statusFilter = Nothing
      }
    , fetchUsers shared Nothing
    )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = GotUsers (Result ApiError (List AdminUserItem))
    | ChangeStatusFilter String
    | Refresh


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotUsers result ->
            case result of
                Ok users ->
                    ( { model | users = Success users }, Cmd.none )

                Err err ->
                    ( { model | users = Failure err }, Cmd.none )

        ChangeStatusFilter value ->
            let
                newFilter =
                    if value == "" then
                        Nothing

                    else
                        Just value
            in
            ( { model | statusFilter = newFilter, users = Loading }
            , fetchUsers model.shared newFilter
            )

        Refresh ->
            ( { model | users = Loading }
            , fetchUsers model.shared model.statusFilter
            )


fetchUsers : Shared -> Maybe String -> Cmd Msg
fetchUsers shared statusFilter =
    AdminUserApi.listAdminUsers
        { config = Shared.toRequestConfig shared
        , statusFilter = statusFilter
        , toMsg = GotUsers
        }



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewHeader
        , viewFilters model.statusFilter
        , viewContent model.users
        ]


{-| ヘッダー（タイトル + ユーザー追加ボタン）
-}
viewHeader : Html Msg
viewHeader =
    div [ class "flex items-center justify-between mb-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ]
            [ text "ユーザー管理" ]
        , Button.link
            { variant = Button.Primary
            , href = Route.toString Route.UserNew
            }
            [ text "ユーザーを追加" ]
        ]


{-| ステータスフィルタ
-}
viewFilters : Maybe String -> Html Msg
viewFilters statusFilter =
    div [ class "mb-4" ]
        [ select
            [ class "rounded-lg border border-secondary-300 px-3 py-2 text-sm"
            , onInput ChangeStatusFilter
            , value (Maybe.withDefault "" statusFilter)
            ]
            [ option [ value "" ] [ text "すべてのステータス" ]
            , option [ value "active" ] [ text "アクティブ" ]
            , option [ value "inactive" ] [ text "非アクティブ" ]
            ]
        ]


{-| コンテンツ（RemoteData パターン）
-}
viewContent : RemoteData ApiError (List AdminUserItem) -> Html Msg
viewContent remoteUsers =
    case remoteUsers of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure err ->
            ErrorState.view
                { message = ErrorMessage.toUserMessage { entityName = "ユーザー" } err
                , onRefresh = Refresh
                }

        Success users ->
            viewUserList users


{-| ユーザー一覧
-}
viewUserList : List AdminUserItem -> Html Msg
viewUserList users =
    if List.isEmpty users then
        EmptyState.view
            { message = "ユーザーが見つかりません。"
            , description = Nothing
            }

    else
        div []
            [ viewUserTable users
            , div [ class "mt-2 text-sm text-secondary-500" ]
                [ text (String.fromInt (List.length users) ++ " 件のユーザー") ]
            ]


{-| ユーザーテーブル
-}
viewUserTable : List AdminUserItem -> Html Msg
viewUserTable users =
    div [ class "overflow-x-auto rounded-lg border border-secondary-200" ]
        [ table [ class "w-full" ]
            [ thead [ class "bg-secondary-50" ]
                [ tr []
                    [ th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "No." ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "名前" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "メール" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "ロール" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "ステータス" ]
                    ]
                ]
            , tbody [ class "divide-y divide-secondary-200 bg-white" ]
                (List.map viewUserRow users)
            ]
        ]


{-| ユーザー行

行全体をクリック可能にするため、各セルの内容を <a> で囲んでいる。
<tr> に onClick を付ける方法は右クリックや Cmd+クリックが効かなくなるため不採用。

-}
viewUserRow : AdminUserItem -> Html Msg
viewUserRow user =
    let
        detailUrl =
            Route.toString (Route.UserDetail user.displayNumber)

        cellLink attrs children =
            a (href detailUrl :: class "block px-4 py-3" :: attrs) children
    in
    tr [ class "hover:bg-secondary-50 transition-colors cursor-pointer" ]
        [ td [ class "text-sm" ]
            [ cellLink [ class "font-medium text-primary-600" ] [ text (String.fromInt user.displayNumber) ] ]
        , td [ class "text-sm text-secondary-900" ]
            [ cellLink [] [ text user.name ] ]
        , td [ class "text-sm text-secondary-500" ]
            [ cellLink [] [ text user.email ] ]
        , td [ class "text-sm text-secondary-500" ]
            [ cellLink [] [ text (String.join ", " user.roles) ] ]
        , td [ class "text-sm" ]
            [ cellLink [] [ Badge.view (AdminUser.statusToBadge user.status) ] ]
        ]
