module Page.User.New exposing (Model, Msg, init, isDirty, update, updateShared, view)

{-| ユーザー作成画面

新規ユーザーを作成するフォーム。
作成成功後は初期パスワードを表示する。

-}

import Api exposing (ApiError)
import Api.AdminUser as AdminUserApi
import Api.ErrorMessage as ErrorMessage
import Api.Role as RoleApi
import Component.Button as Button
import Component.FormField as FormField
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.AdminUser exposing (CreateUserResponse)
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
    , email : String
    , name : String
    , selectedRoleId : String
    , roles : RemoteData ApiError (List RoleItem)
    , validationErrors : Dict String String
    , submitting : Bool
    , createdUser : Maybe CreateUserResponse
    , errorMessage : Maybe String
    , isDirty_ : Bool
    }


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , email = ""
      , name = ""
      , selectedRoleId = ""
      , roles = Loading
      , validationErrors = Dict.empty
      , submitting = False
      , createdUser = Nothing
      , errorMessage = Nothing
      , isDirty_ = False
      }
    , RoleApi.listRoles
        { config = Shared.toRequestConfig shared
        , toMsg = GotRoles
        }
    )


isDirty : Model -> Bool
isDirty =
    DirtyState.isDirty


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = UpdateEmail String
    | UpdateName String
    | UpdateRole String
    | SubmitForm
    | GotRoles (Result ApiError (List RoleItem))
    | GotCreateResult (Result ApiError CreateUserResponse)
    | DismissMessage


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        UpdateEmail value ->
            let
                ( dirtyModel, dirtyCmd ) =
                    DirtyState.markDirty model
            in
            ( { dirtyModel | email = value }, dirtyCmd )

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
                            [ ( "email", Encode.string model.email )
                            , ( "name", Encode.string model.name )
                            , ( "role_id", Encode.string model.selectedRoleId )
                            ]
                in
                ( { model | submitting = True, validationErrors = Dict.empty }
                , AdminUserApi.createUser
                    { config = Shared.toRequestConfig model.shared
                    , body = body
                    , toMsg = GotCreateResult
                    }
                )

            else
                ( { model | validationErrors = errors }, Cmd.none )

        GotRoles result ->
            case result of
                Ok roles ->
                    ( { model | roles = Success roles }, Cmd.none )

                Err err ->
                    ( { model | roles = Failure err }, Cmd.none )

        GotCreateResult result ->
            case result of
                Ok createdUser ->
                    let
                        ( cleanModel, cleanCmd ) =
                            DirtyState.clearDirty model
                    in
                    ( { cleanModel
                        | submitting = False
                        , createdUser = Just createdUser
                        , errorMessage = Nothing
                      }
                    , cleanCmd
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
        |> validateEmail model.email
        |> Validation.validateRequiredString
            { fieldKey = "name", fieldLabel = "名前", maxLength = 100 }
            model.name
        |> validateRole model.selectedRoleId


validateEmail : String -> Dict String String -> Dict String String
validateEmail email errors =
    let
        trimmed =
            String.trim email
    in
    if String.isEmpty trimmed then
        Dict.insert "email" "メールアドレスを入力してください。" errors

    else if not (String.contains "@" trimmed) then
        Dict.insert "email" "有効なメールアドレスを入力してください。" errors

    else
        errors


validateRole : String -> Dict String String -> Dict String String
validateRole roleId errors =
    if String.isEmpty roleId then
        Dict.insert "role" "ロールを選択してください。" errors

    else
        errors



-- VIEW


view : Model -> Html Msg
view model =
    case model.createdUser of
        Just createdUser ->
            viewCreatedResult createdUser

        Nothing ->
            viewForm model


{-| ユーザー作成成功画面（初期パスワード表示）
-}
viewCreatedResult : CreateUserResponse -> Html Msg
viewCreatedResult createdUser =
    div [ class "mx-auto max-w-lg" ]
        [ div [ class "rounded-lg border border-success-200 bg-success-50 p-6" ]
            [ h2 [ class "text-xl font-bold text-success-800 mb-4" ]
                [ text "ユーザーを作成しました" ]
            , dl [ class "space-y-3" ]
                [ viewResultField "名前" createdUser.name
                , viewResultField "メール" createdUser.email
                , viewResultField "ロール" createdUser.role
                , div []
                    [ dt [ class "text-sm font-medium text-secondary-500" ] [ text "初期パスワード" ]
                    , dd [ class "mt-1 rounded-lg bg-white p-3 font-mono text-lg text-secondary-900 border border-secondary-200 select-all" ]
                        [ text createdUser.initialPassword ]
                    ]
                ]
            , p [ class "mt-4 text-sm text-secondary-600" ]
                [ text "このパスワードは再表示できません。必ず控えてください。" ]
            , div [ class "mt-6" ]
                [ Button.link
                    { variant = Button.Primary
                    , href = Route.toString Route.Users
                    }
                    [ text "ユーザー一覧に戻る" ]
                ]
            ]
        ]


viewResultField : String -> String -> Html msg
viewResultField label fieldValue =
    div []
        [ dt [ class "text-sm font-medium text-secondary-500" ] [ text label ]
        , dd [ class "mt-1 text-sm text-secondary-900" ] [ text fieldValue ]
        ]


{-| ユーザー作成フォーム
-}
viewForm : Model -> Html Msg
viewForm model =
    div []
        [ viewBreadcrumb
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = Nothing
            , errorMessage = model.errorMessage
            }
        , case model.roles of
            Loading ->
                LoadingSpinner.view

            Failure _ ->
                div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
                    [ text "ロール情報の取得に失敗しました。" ]

            _ ->
                viewFormContent model
        ]


viewBreadcrumb : Html Msg
viewBreadcrumb =
    nav [ class "mb-4 text-sm text-secondary-500" ]
        [ a [ href (Route.toString Route.Users), class "hover:text-primary-600" ] [ text "ユーザー管理" ]
        , span [ class "mx-2" ] [ text ">" ]
        , span [] [ text "新規作成" ]
        ]


viewFormContent : Model -> Html Msg
viewFormContent model =
    let
        roles =
            RemoteData.withDefault [] model.roles
    in
    Html.form [ onSubmit SubmitForm, class "mx-auto max-w-lg space-y-6" ]
        [ h2 [ class "text-2xl font-bold text-secondary-900" ] [ text "ユーザーを作成" ]
        , FormField.viewTextField
            { label = "メールアドレス"
            , value = model.email
            , onInput = UpdateEmail
            , error = Dict.get "email" model.validationErrors
            , inputType = "email"
            , placeholder = "user@example.com"
            }
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
            , options = List.map (\role -> { value = role.id, label = role.name }) roles
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
                        "作成中..."

                     else
                        "作成"
                    )
                ]
            , Button.link
                { variant = Button.Outline
                , href = Route.toString Route.Users
                }
                [ text "キャンセル" ]
            ]
        ]
