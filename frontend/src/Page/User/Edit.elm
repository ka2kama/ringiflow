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
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.AdminUser exposing (UserDetail, UserResponse)
import Data.Role exposing (RoleItem)
import Dict exposing (Dict)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput, onSubmit)
import Json.Encode as Encode
import Ports
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
isDirty model =
    model.isDirty_


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
                        firstRoleId =
                            List.head userDetail.roles
                                |> Maybe.andThen (\roleName -> findRoleIdByName roleName model.roles)
                                |> Maybe.withDefault ""
                    in
                    ( { model
                        | user = Success userDetail
                        , name = userDetail.name
                        , selectedRoleId = firstRoleId
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
                    -- ユーザー詳細が先に読み込まれていた場合、ロール ID を解決する
                    case model.user of
                        Success userDetail ->
                            let
                                firstRoleId =
                                    List.head userDetail.roles
                                        |> Maybe.andThen (\roleName -> findRoleIdByName roleName (Success roles))
                                        |> Maybe.withDefault ""
                            in
                            ( { newModel | selectedRoleId = firstRoleId }, Cmd.none )

                        _ ->
                            ( newModel, Cmd.none )

                Err err ->
                    ( { model | roles = Failure err }, Cmd.none )

        UpdateName value ->
            let
                ( dirtyModel, dirtyCmd ) =
                    markDirty model
            in
            ( { dirtyModel | name = value }, dirtyCmd )

        UpdateRole value ->
            let
                ( dirtyModel, dirtyCmd ) =
                    markDirty model
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
                            , ( "role_id", Encode.string model.selectedRoleId )
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
                    ( { model | submitting = False, isDirty_ = False }
                    , Cmd.batch
                        [ Ports.setBeforeUnloadEnabled False
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


{-| ロール名から一覧内のロール ID を検索する
-}
findRoleIdByName : String -> RemoteData ApiError (List RoleItem) -> Maybe String
findRoleIdByName roleName rolesData =
    case rolesData of
        Success roles ->
            roles
                |> List.filter (\r -> r.name == roleName)
                |> List.head
                |> Maybe.map .id

        _ ->
            Nothing


{-| Dirty フラグを立てる（最初の変更時のみ Port を呼び出す）
-}
markDirty : Model -> ( Model, Cmd Msg )
markDirty model =
    if model.isDirty_ then
        ( model, Cmd.none )

    else
        ( { model | isDirty_ = True }
        , Ports.setBeforeUnloadEnabled True
        )



-- VALIDATION


validateForm : Model -> Dict String String
validateForm model =
    Dict.empty
        |> validateName model.name
        |> validateRole model.selectedRoleId


validateName : String -> Dict String String -> Dict String String
validateName name errors =
    let
        trimmed =
            String.trim name
    in
    if String.isEmpty trimmed then
        Dict.insert "name" "名前を入力してください。" errors

    else if String.length trimmed > 100 then
        Dict.insert "name" "名前は100文字以内で入力してください。" errors

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
    nav [ class "mb-4 text-sm text-secondary-500" ]
        [ a [ href (Route.toString Route.Users), class "hover:text-primary-600" ] [ text "ユーザー管理" ]
        , span [ class "mx-2" ] [ text ">" ]
        , a [ href (Route.toString (Route.UserDetail displayNumber)), class "hover:text-primary-600" ]
            [ text ("#" ++ String.fromInt displayNumber) ]
        , span [ class "mx-2" ] [ text ">" ]
        , span [] [ text "編集" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case ( model.user, model.roles ) of
        ( Loading, _ ) ->
            LoadingSpinner.view

        ( _, Loading ) ->
            LoadingSpinner.view

        ( Failure err, _ ) ->
            div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
                [ text (ErrorMessage.toUserMessage { entityName = "ユーザー" } err) ]

        ( _, Failure _ ) ->
            div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
                [ text "ロール情報の取得に失敗しました。" ]

        ( Success userDetail, Success roles ) ->
            viewFormContent model userDetail roles

        _ ->
            text ""


viewFormContent : Model -> UserDetail -> List RoleItem -> Html Msg
viewFormContent model userDetail roles =
    Html.form [ onSubmit SubmitForm, class "mx-auto max-w-lg space-y-6" ]
        [ h2 [ class "text-2xl font-bold text-secondary-900" ] [ text "ユーザーを編集" ]
        , viewReadOnlyField "メールアドレス" userDetail.email
        , viewTextField
            { label = "名前"
            , value = model.name
            , onInput = UpdateName
            , error = Dict.get "name" model.validationErrors
            , inputType = "text"
            , placeholder = "山田 太郎"
            }
        , viewRoleSelect roles model.selectedRoleId (Dict.get "role" model.validationErrors)
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


{-| 読み取り専用フィールド
-}
viewReadOnlyField : String -> String -> Html msg
viewReadOnlyField labelText fieldValue =
    div []
        [ label [ class "block text-sm font-medium text-secondary-700 mb-1" ] [ text labelText ]
        , div [ class "w-full rounded-lg border border-secondary-200 bg-secondary-50 px-3 py-2 text-sm text-secondary-500" ]
            [ text fieldValue ]
        ]


viewTextField :
    { label : String
    , value : String
    , onInput : String -> Msg
    , error : Maybe String
    , inputType : String
    , placeholder : String
    }
    -> Html Msg
viewTextField config =
    div []
        [ label [ class "block text-sm font-medium text-secondary-700 mb-1" ] [ text config.label ]
        , input
            [ type_ config.inputType
            , value config.value
            , onInput config.onInput
            , placeholder config.placeholder
            , class
                ("w-full rounded-lg border px-3 py-2 text-sm "
                    ++ (case config.error of
                            Just _ ->
                                "border-error-300 focus:border-error-500 focus:ring-error-500"

                            Nothing ->
                                "border-secondary-300 focus:border-primary-500 focus:ring-primary-500"
                       )
                )
            ]
            []
        , case config.error of
            Just errorMsg ->
                p [ class "mt-1 text-sm text-error-600" ] [ text errorMsg ]

            Nothing ->
                text ""
        ]


viewRoleSelect : List RoleItem -> String -> Maybe String -> Html Msg
viewRoleSelect roles selectedRoleId error =
    div []
        [ label [ class "block text-sm font-medium text-secondary-700 mb-1" ] [ text "ロール" ]
        , select
            [ class
                ("w-full rounded-lg border px-3 py-2 text-sm "
                    ++ (case error of
                            Just _ ->
                                "border-error-300 focus:border-error-500 focus:ring-error-500"

                            Nothing ->
                                "border-secondary-300 focus:border-primary-500 focus:ring-primary-500"
                       )
                )
            , onInput UpdateRole
            , value selectedRoleId
            ]
            (option [ Html.Attributes.value "" ] [ text "-- ロールを選択 --" ]
                :: List.map
                    (\role ->
                        option [ Html.Attributes.value role.id ] [ text role.name ]
                    )
                    roles
            )
        , case error of
            Just errorMsg ->
                p [ class "mt-1 text-sm text-error-600" ] [ text errorMsg ]

            Nothing ->
                text ""
        ]
