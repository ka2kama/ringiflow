module Page.User.Detail exposing (Model, Msg, init, update, updateShared, view)

{-| ユーザー詳細画面

ユーザーの基本情報、ロール、権限を表示する。
ステータス変更（有効化/無効化）操作を提供する。
自己無効化防止: ログイン中のユーザー自身は無効化できない。

-}

import Api exposing (ApiError)
import Api.AdminUser as AdminUserApi
import Api.ErrorMessage as ErrorMessage
import Component.Badge as Badge
import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.AdminUser as AdminUser exposing (UserDetail, UserResponse)
import Html exposing (..)
import Html.Attributes exposing (..)
import Json.Encode as Encode
import Ports
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , displayNumber : Int
    , user : RemoteData ApiError UserDetail
    , successMessage : Maybe String
    , errorMessage : Maybe String
    , confirmAction : Maybe ConfirmAction
    , isSubmitting : Bool
    }


{-| 確認ダイアログで保留中のアクション
-}
type ConfirmAction
    = ConfirmDeactivate
    | ConfirmActivate


init : Shared -> Int -> ( Model, Cmd Msg )
init shared displayNumber =
    ( { shared = shared
      , displayNumber = displayNumber
      , user = Loading
      , successMessage = Nothing
      , errorMessage = Nothing
      , confirmAction = Nothing
      , isSubmitting = False
      }
    , fetchUserDetail shared displayNumber
    )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = GotUserDetail (Result ApiError UserDetail)
    | ClickDeactivate
    | ClickActivate
    | ConfirmStatusChange
    | CancelStatusChange
    | GotStatusChangeResult (Result ApiError UserResponse)
    | DismissMessage
    | Refresh


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotUserDetail result ->
            case result of
                Ok userDetail ->
                    ( { model | user = Success userDetail }, Cmd.none )

                Err err ->
                    ( { model | user = Failure err }, Cmd.none )

        ClickDeactivate ->
            ( { model | confirmAction = Just ConfirmDeactivate }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ClickActivate ->
            ( { model | confirmAction = Just ConfirmActivate }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ConfirmStatusChange ->
            case model.confirmAction of
                Just action ->
                    let
                        newStatus =
                            case action of
                                ConfirmDeactivate ->
                                    "inactive"

                                ConfirmActivate ->
                                    "active"

                        body =
                            Encode.object
                                [ ( "status", Encode.string newStatus ) ]
                    in
                    ( { model | isSubmitting = True }
                    , AdminUserApi.updateUserStatus
                        { config = Shared.toRequestConfig model.shared
                        , displayNumber = model.displayNumber
                        , body = body
                        , toMsg = GotStatusChangeResult
                        }
                    )

                Nothing ->
                    ( model, Cmd.none )

        CancelStatusChange ->
            ( { model | confirmAction = Nothing }, Cmd.none )

        GotStatusChangeResult result ->
            case result of
                Ok _ ->
                    let
                        successMsg =
                            case model.confirmAction of
                                Just ConfirmDeactivate ->
                                    "ユーザーを無効化しました。"

                                Just ConfirmActivate ->
                                    "ユーザーを有効化しました。"

                                Nothing ->
                                    "ステータスを変更しました。"
                    in
                    ( { model
                        | isSubmitting = False
                        , confirmAction = Nothing
                        , successMessage = Just successMsg
                        , errorMessage = Nothing
                      }
                    , fetchUserDetail model.shared model.displayNumber
                    )

                Err err ->
                    ( { model
                        | isSubmitting = False
                        , confirmAction = Nothing
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ユーザー" } err)
                      }
                    , Cmd.none
                    )

        DismissMessage ->
            ( { model | successMessage = Nothing, errorMessage = Nothing }
            , Cmd.none
            )

        Refresh ->
            ( { model | user = Loading }
            , fetchUserDetail model.shared model.displayNumber
            )


fetchUserDetail : Shared -> Int -> Cmd Msg
fetchUserDetail shared displayNumber =
    AdminUserApi.getUserDetail
        { config = Shared.toRequestConfig shared
        , displayNumber = displayNumber
        , toMsg = GotUserDetail
        }



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewBreadcrumb model.displayNumber
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = model.successMessage
            , errorMessage = model.errorMessage
            }
        , viewContent model
        , viewConfirmDialog model.confirmAction
        ]


{-| パンくずリスト
-}
viewBreadcrumb : Int -> Html Msg
viewBreadcrumb displayNumber =
    nav [ class "mb-4 text-sm text-secondary-500" ]
        [ a [ href (Route.toString Route.Users), class "hover:text-primary-600" ] [ text "ユーザー管理" ]
        , span [ class "mx-2" ] [ text ">" ]
        , span [] [ text ("#" ++ String.fromInt displayNumber) ]
        ]


{-| コンテンツ（RemoteData パターン）
-}
viewContent : Model -> Html Msg
viewContent model =
    case model.user of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure err ->
            div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
                [ p [] [ text (ErrorMessage.toUserMessage { entityName = "ユーザー" } err) ]
                , Button.view
                    { variant = Button.Outline
                    , disabled = False
                    , onClick = Refresh
                    }
                    [ text "再読み込み" ]
                ]

        Success userDetail ->
            viewUserDetail model userDetail


{-| ユーザー詳細の表示
-}
viewUserDetail : Model -> UserDetail -> Html Msg
viewUserDetail model userDetail =
    div [ class "space-y-6" ]
        [ viewBasicInfo model userDetail
        , viewRolesAndPermissions userDetail
        ]


{-| 基本情報セクション
-}
viewBasicInfo : Model -> UserDetail -> Html Msg
viewBasicInfo model userDetail =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6" ]
        [ div [ class "flex items-center justify-between mb-4" ]
            [ h3 [ class "text-lg font-semibold text-secondary-900" ] [ text "基本情報" ]
            , viewActions model userDetail
            ]
        , dl [ class "grid grid-cols-1 gap-4 sm:grid-cols-2" ]
            [ viewField "表示番号" (String.fromInt userDetail.displayNumber)
            , viewField "名前" userDetail.name
            , viewField "メール" userDetail.email
            , viewFieldWithBadge "ステータス" (AdminUser.statusToBadge userDetail.status)
            , viewField "テナント" userDetail.tenantName
            ]
        ]


{-| アクションボタン群
-}
viewActions : Model -> UserDetail -> Html Msg
viewActions model userDetail =
    let
        isSelf =
            Shared.getUserId model.shared == Just userDetail.id
    in
    div [ class "flex gap-2" ]
        [ Button.link
            { variant = Button.Outline
            , href = Route.toString (Route.UserEdit userDetail.displayNumber)
            }
            [ text "編集" ]
        , if not isSelf then
            viewStatusToggleButton model.isSubmitting userDetail.status

          else
            text ""
        ]


{-| ステータス変更ボタン（自分以外のみ表示）
-}
viewStatusToggleButton : Bool -> String -> Html Msg
viewStatusToggleButton isSubmitting status =
    case status of
        "active" ->
            Button.view
                { variant = Button.Error
                , disabled = isSubmitting
                , onClick = ClickDeactivate
                }
                [ text
                    (if isSubmitting then
                        "処理中..."

                     else
                        "無効化"
                    )
                ]

        "inactive" ->
            Button.view
                { variant = Button.Success
                , disabled = isSubmitting
                , onClick = ClickActivate
                }
                [ text
                    (if isSubmitting then
                        "処理中..."

                     else
                        "有効化"
                    )
                ]

        _ ->
            text ""


{-| ロール・権限セクション
-}
viewRolesAndPermissions : UserDetail -> Html Msg
viewRolesAndPermissions userDetail =
    div [ class "rounded-lg border border-secondary-200 bg-white p-6" ]
        [ h3 [ class "text-lg font-semibold text-secondary-900 mb-4" ] [ text "ロール・権限" ]
        , div [ class "space-y-3" ]
            [ div []
                [ dt [ class "text-sm font-medium text-secondary-500" ] [ text "ロール" ]
                , dd [ class "mt-1 flex flex-wrap gap-2" ]
                    (if List.isEmpty userDetail.roles then
                        [ span [ class "text-sm text-secondary-400" ] [ text "なし" ] ]

                     else
                        List.map
                            (\role ->
                                Badge.view
                                    { colorClass = "bg-primary-100 text-primary-800"
                                    , label = role
                                    }
                            )
                            userDetail.roles
                    )
                ]
            , div []
                [ dt [ class "text-sm font-medium text-secondary-500" ] [ text "権限" ]
                , dd [ class "mt-1" ]
                    (if List.isEmpty userDetail.permissions then
                        [ span [ class "text-sm text-secondary-400" ] [ text "なし" ] ]

                     else
                        [ ul [ class "list-disc list-inside text-sm text-secondary-700" ]
                            (List.map (\p -> li [] [ text p ]) userDetail.permissions)
                        ]
                    )
                ]
            ]
        ]


{-| フィールド表示
-}
viewField : String -> String -> Html msg
viewField label fieldValue =
    div []
        [ dt [ class "text-sm font-medium text-secondary-500" ] [ text label ]
        , dd [ class "mt-1 text-sm text-secondary-900" ] [ text fieldValue ]
        ]


{-| Badge 付きフィールド表示
-}
viewFieldWithBadge : String -> { colorClass : String, label : String } -> Html msg
viewFieldWithBadge label badgeConfig =
    div []
        [ dt [ class "text-sm font-medium text-secondary-500" ] [ text label ]
        , dd [ class "mt-1" ] [ Badge.view badgeConfig ]
        ]


{-| 確認ダイアログ
-}
viewConfirmDialog : Maybe ConfirmAction -> Html Msg
viewConfirmDialog maybeAction =
    case maybeAction of
        Just ConfirmDeactivate ->
            ConfirmDialog.view
                { title = "ユーザーを無効化"
                , message = "このユーザーを無効化しますか？無効化されたユーザーはログインできなくなります。"
                , confirmLabel = "無効化する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmStatusChange
                , onCancel = CancelStatusChange
                , actionStyle = ConfirmDialog.Destructive
                }

        Just ConfirmActivate ->
            ConfirmDialog.view
                { title = "ユーザーを有効化"
                , message = "このユーザーを有効化しますか？有効化されたユーザーは再びログインできるようになります。"
                , confirmLabel = "有効化する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmStatusChange
                , onCancel = CancelStatusChange
                , actionStyle = ConfirmDialog.Positive
                }

        Nothing ->
            text ""
