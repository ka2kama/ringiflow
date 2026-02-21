module Page.User.Edit exposing (Model, Msg, init, isDirty, update, updateShared, view)

{-| ユーザー編集画面

既存ユーザーの名前とロールを編集する。
メールアドレスは表示のみ。
保存成功後はユーザー詳細画面に遷移する。

-}

import Api exposing (ApiError)
import Api.AdminUser as AdminUserApi
import Api.ErrorMessage as ErrorMessage
import Api.Role as RoleApi
import Browser.Navigation as Nav
import Component.Button as Button
import Component.ErrorState as ErrorState
import Component.FormField as FormField
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.AdminUser exposing (UserDetail, UserResponse)
import Data.Role exposing (RoleItem)
import Dict exposing (Dict)
import Form.DirtyState as DirtyState
import Form.Validation as Validation
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onSubmit)
import Json.Encode as Encode
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , key : Nav.Key
    , displayNumber : Int
    , user : RemoteData ApiError UserDetail
    , name : String
    , selectedRoleId : String
    , roles : RemoteData ApiError (List RoleItem)
    , validationErrors : Dict String String
    , submitting : Bool
    , errorMessage : Maybe String
    , isDirty_ : Bool
    }


init : Shared -> Nav.Key -> Int -> ( Model, Cmd Msg )
init shared key displayNumber =
    ( { shared = shared
      , key = key
      , displayNumber = displayNumber
      , user = Loading
      , name = ""
      , selectedRoleId = ""
      , roles = Loading
      , validationErrors = Dict.empty
      , submitting = False
      , errorMessage = Nothing
      , isDirty_ = False
      }
    , Cmd.batch
        [ AdminUserApi.getUserDetail
            { config = Shared.toRequestConfig shared
            , displayNumber = displayNumber
            , toMsg = GotUserDetail
            }
        , RoleApi.listRoles
            { config = Shared.toRequestConfig shared
            , toMsg = GotRoles
            }
        ]
    )


isDirty : Model -> Bool
isDirty =
    DirtyState.isDirty


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = GotUserDetail (Result ApiError UserDetail)
    | GotRoles (Result ApiError (List RoleItem))
    | UpdateName String
    | UpdateRole String
    | SubmitForm
    | GotUpdateResult (Result ApiError UserResponse)
    | DismissMessage


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotUserDetail result ->
            case result of
                Ok userDetail ->
                    let
                        firstRoleName =
                            List.head userDetail.roles
                                |> Maybe.withDefault ""
                    in
                    ( { model
                        | user = Success userDetail
                        , name = userDetail.name
                        , selectedRoleId = firstRoleName
                      }
                    , Cmd.none
                    )

                Err err ->
                    ( { model | user = Failure err }, Cmd.none )

        GotRoles result ->
            case result of
                Ok roles ->
                    let
                        newModel =
                            { model | roles = Success roles }
                    in
                    -- ユーザー詳細が先に読み込まれていた場合、ロール名を設定する
                    case model.user of
                        Success userDetail ->
                            let
                                firstRoleName =
                                    List.head userDetail.roles
                                        |> Maybe.withDefault ""
                            in
                            ( { newModel | selectedRoleId = firstRoleName }, Cmd.none )

                        _ ->
                            ( newModel, Cmd.none )

                Err err ->
                    ( { model | roles = Failure err }, Cmd.none )

        UpdateName value ->
            let
                ( dirtyModel, dirtyCmd ) =
                    DirtyState.markDirty model
            in
            ( { dirtyModel | name = value }, dirtyCmd )

        UpdateRole value ->
            let
                ( dirtyModel, dirtyCmd ) =
                    DirtyState.markDirty model
            in
            ( { dirtyModel | selectedRoleId = value }, dirtyCmd )

        SubmitForm ->
            let
                errors =
                    validateForm model
            in
            if Dict.isEmpty errors then
                let
                    body =
                        Encode.object
                            [ ( "name", Encode.string model.name )
                            , ( "role_name", Encode.string model.selectedRoleId )
                            ]
                in
                ( { model | submitting = True, validationErrors = Dict.empty }
                , AdminUserApi.updateUser
                    { config = Shared.toRequestConfig model.shared
                    , displayNumber = model.displayNumber
                    , body = body
                    , toMsg = GotUpdateResult
                    }
                )

            else
                ( { model | validationErrors = errors }, Cmd.none )

        GotUpdateResult result ->
            case result of
                Ok _ ->
                    let
                        ( cleanModel, cleanCmd ) =
                            DirtyState.clearDirty model
                    in
                    ( { cleanModel | submitting = False }
                    , Cmd.batch
                        [ cleanCmd
                        , Nav.pushUrl model.key
                            (Route.toString (Route.UserDetail model.displayNumber))
                        ]
                    )

                Err err ->
                    ( { model
                        | submitting = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ユーザー" } err)
                      }
                    , Cmd.none
                    )

        DismissMessage ->
            ( { model | errorMessage = Nothing }, Cmd.none )



-- VALIDATION


validateForm : Model -> Dict String String
validateForm model =
    Dict.empty
        |> Validation.validateRequiredString
            { fieldKey = "name", fieldLabel = "名前", maxLength = 100 }
            model.name
        |> validateRole model.selectedRoleId


validateRole : String -> Dict String String -> Dict String String
validateRole roleId errors =
    if String.isEmpty roleId then
        Dict.insert "role" "ロールを選択してください。" errors

    else
        errors



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewBreadcrumb model.displayNumber
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = Nothing
            , errorMessage = model.errorMessage
            }
        , viewContent model
        ]


viewBreadcrumb : Int -> Html Msg
viewBreadcrumb displayNumber =
    nav [ class "mb-4 flex items-center gap-2 text-sm" ]
        [ a [ href (Route.toString Route.Users), class "text-secondary-500 hover:text-primary-600 transition-colors" ] [ text "ユーザー管理" ]
        , span [ class "text-secondary-400" ] [ text "/" ]
        , a [ href (Route.toString (Route.UserDetail displayNumber)), class "text-secondary-500 hover:text-primary-600 transition-colors" ]
            [ text ("#" ++ String.fromInt displayNumber) ]
        , span [ class "text-secondary-400" ] [ text "/" ]
        , span [ class "text-secondary-900 font-medium" ] [ text "編集" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case ( model.user, model.roles ) of
        ( Loading, _ ) ->
            LoadingSpinner.view

        ( _, Loading ) ->
            LoadingSpinner.view

        ( Failure err, _ ) ->
            ErrorState.viewSimple (ErrorMessage.toUserMessage { entityName = "ユーザー" } err)

        ( _, Failure _ ) ->
            ErrorState.viewSimple "ロール情報の取得に失敗しました。"

        ( Success userDetail, Success roles ) ->
            viewFormContent model userDetail roles

        _ ->
            text ""


viewFormContent : Model -> UserDetail -> List RoleItem -> Html Msg
viewFormContent model userDetail roles =
    Html.form [ onSubmit SubmitForm, class "mx-auto max-w-lg space-y-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ] [ text "ユーザーを編集" ]
        , FormField.viewReadOnlyField "メールアドレス" userDetail.email
        , FormField.viewTextField
            { label = "名前"
            , value = model.name
            , onInput = UpdateName
            , error = Dict.get "name" model.validationErrors
            , inputType = "text"
            , placeholder = "山田 太郎"
            }
        , FormField.viewSelectField
            { label = "ロール"
            , value = model.selectedRoleId
            , onInput = UpdateRole
            , error = Dict.get "role" model.validationErrors
            , options = List.map (\role -> { value = role.name, label = role.name }) roles
            , placeholder = "-- ロールを選択 --"
            }
        , div [ class "flex gap-3" ]
            [ Button.view
                { variant = Button.Primary
                , disabled = model.submitting
                , onClick = SubmitForm
                }
                [ text
                    (if model.submitting then
                        "保存中..."

                     else
                        "保存"
                    )
                ]
            , Button.link
                { variant = Button.Outline
                , href = Route.toString (Route.UserDetail model.displayNumber)
                }
                [ text "キャンセル" ]
            ]
        ]
