module Page.Role.List exposing (Model, Msg, init, update, updateShared, view)

{-| ロール一覧画面

システムロールとカスタムロールをセクション分けして表示。
カスタムロールの削除（ConfirmDialog 付き）に対応。

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.Role as RoleApi
import Component.Badge as Badge
import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Component.ErrorState as ErrorState
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.Role exposing (RoleItem)
import Html exposing (..)
import Html.Attributes exposing (..)
import Ports
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , roles : RemoteData ApiError (List RoleItem)
    , deleteTarget : Maybe RoleItem
    , isDeleting : Bool
    , successMessage : Maybe String
    , errorMessage : Maybe String
    }


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , roles = Loading
      , deleteTarget = Nothing
      , isDeleting = False
      , successMessage = Nothing
      , errorMessage = Nothing
      }
    , RoleApi.listRoles
        { config = Shared.toRequestConfig shared
        , toMsg = GotRoles
        }
    )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = GotRoles (Result ApiError (List RoleItem))
    | ClickDelete RoleItem
    | ConfirmDelete
    | CancelDelete
    | GotDeleteResult (Result ApiError ())
    | DismissMessage
    | Refresh


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotRoles result ->
            case result of
                Ok roles ->
                    ( { model | roles = Success roles }, Cmd.none )

                Err err ->
                    ( { model | roles = Failure err }, Cmd.none )

        ClickDelete role ->
            ( { model | deleteTarget = Just role }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ConfirmDelete ->
            case model.deleteTarget of
                Just role ->
                    ( { model | deleteTarget = Nothing, isDeleting = True }
                    , RoleApi.deleteRole
                        { config = Shared.toRequestConfig model.shared
                        , roleId = role.id
                        , toMsg = GotDeleteResult
                        }
                    )

                Nothing ->
                    ( model, Cmd.none )

        CancelDelete ->
            ( { model | deleteTarget = Nothing }, Cmd.none )

        GotDeleteResult result ->
            case result of
                Ok () ->
                    ( { model
                        | isDeleting = False
                        , successMessage = Just "ロールを削除しました。"
                        , errorMessage = Nothing
                        , roles = Loading
                      }
                    , RoleApi.listRoles
                        { config = Shared.toRequestConfig model.shared
                        , toMsg = GotRoles
                        }
                    )

                Err err ->
                    ( { model
                        | isDeleting = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ロール" } err)
                      }
                    , Cmd.none
                    )

        DismissMessage ->
            ( { model | successMessage = Nothing, errorMessage = Nothing }, Cmd.none )

        Refresh ->
            ( { model | roles = Loading }
            , RoleApi.listRoles
                { config = Shared.toRequestConfig model.shared
                , toMsg = GotRoles
                }
            )



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewHeader
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = model.successMessage
            , errorMessage = model.errorMessage
            }
        , viewContent model
        , viewConfirmDialog model.deleteTarget
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "flex items-center justify-between mb-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ]
            [ text "ロール管理" ]
        , Button.link
            { variant = Button.Primary
            , href = Route.toString Route.RoleNew
            }
            [ text "ロールを追加" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.roles of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure err ->
            ErrorState.view
                { message = ErrorMessage.toUserMessage { entityName = "ロール" } err
                , onRefresh = Refresh
                }

        Success roles ->
            viewRoleSections roles


viewRoleSections : List RoleItem -> Html Msg
viewRoleSections roles =
    let
        systemRoles =
            List.filter .isSystem roles

        customRoles =
            List.filter (not << .isSystem) roles
    in
    div [ class "space-y-8" ]
        [ if not (List.isEmpty systemRoles) then
            viewRoleSection "システムロール" systemRoles False

          else
            text ""
        , if not (List.isEmpty customRoles) then
            viewRoleSection "カスタムロール" customRoles True

          else
            div [ class "py-8 text-center text-secondary-500" ]
                [ text "カスタムロールがまだありません。" ]
        ]


viewRoleSection : String -> List RoleItem -> Bool -> Html Msg
viewRoleSection sectionTitle roles showActions =
    div []
        [ h3 [ class "mb-4 text-lg font-semibold text-secondary-900" ] [ text sectionTitle ]
        , viewRoleTable roles showActions
        ]


viewRoleTable : List RoleItem -> Bool -> Html Msg
viewRoleTable roles showActions =
    div [ class "overflow-x-auto rounded-lg border border-secondary-200" ]
        [ table [ class "w-full" ]
            [ thead [ class "bg-secondary-50" ]
                [ tr []
                    [ th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "ロール名" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "説明" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "種別" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "ユーザー数" ]
                    , if showActions then
                        th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "操作" ]

                      else
                        text ""
                    ]
                ]
            , tbody [ class "divide-y divide-secondary-200 bg-white" ]
                (List.map (viewRoleRow showActions) roles)
            ]
        ]


viewRoleRow : Bool -> RoleItem -> Html Msg
viewRoleRow showActions role =
    tr [ class "hover:bg-secondary-50 transition-colors" ]
        [ td [ class "px-4 py-3 text-sm" ]
            [ if not role.isSystem then
                a
                    [ href (Route.toString (Route.RoleEdit role.id))
                    , class "font-medium text-primary-600 hover:text-primary-800"
                    ]
                    [ text role.name ]

              else
                span [ class "font-medium text-secondary-900" ] [ text role.name ]
            ]
        , td [ class "px-4 py-3 text-sm text-secondary-500" ]
            [ text (Maybe.withDefault "—" role.description) ]
        , td [ class "px-4 py-3 text-sm" ]
            [ Badge.view (typeToBadge role.isSystem) ]
        , td [ class "px-4 py-3 text-sm text-secondary-500" ]
            [ text (String.fromInt role.userCount) ]
        , if showActions then
            td [ class "px-4 py-3 text-sm" ]
                [ Button.view
                    { variant = Button.Error
                    , disabled = False
                    , onClick = ClickDelete role
                    }
                    [ text "削除" ]
                ]

          else
            text ""
        ]


typeToBadge : Bool -> { colorClass : String, label : String }
typeToBadge isSystem =
    if isSystem then
        { colorClass = "bg-primary-100 text-primary-800 border-primary-200", label = "システム" }

    else
        { colorClass = "bg-secondary-100 text-secondary-800 border-secondary-200", label = "カスタム" }


viewConfirmDialog : Maybe RoleItem -> Html Msg
viewConfirmDialog maybeRole =
    case maybeRole of
        Just role ->
            ConfirmDialog.view
                { title = "ロールを削除"
                , message = "「" ++ role.name ++ "」を削除しますか？この操作は取り消せません。"
                , confirmLabel = "削除する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmDelete
                , onCancel = CancelDelete
                , actionStyle = ConfirmDialog.Destructive
                }

        Nothing ->
            text ""
