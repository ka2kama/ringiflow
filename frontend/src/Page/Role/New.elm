module Page.Role.New exposing (Model, Msg, init, isDirty, update, updateShared, view)

{-| ロール作成画面

ロール名、説明、権限を設定してカスタムロールを作成する。
作成成功後はロール一覧に遷移する。

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.Role as RoleApi
import Browser.Navigation as Nav
import Component.Button as Button
import Component.FormField as FormField
import Component.MessageAlert as MessageAlert
import Component.PermissionMatrix as PermissionMatrix
import Data.Role exposing (RoleDetail)
import Dict exposing (Dict)
import Form.DirtyState as DirtyState
import Form.Validation as Validation
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onSubmit)
import Json.Encode as Encode
import Route
import Set exposing (Set)
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , key : Nav.Key
    , name : String
    , description : String
    , selectedPermissions : Set String
    , validationErrors : Dict String String
    , submitting : Bool
    , errorMessage : Maybe String
    , isDirty_ : Bool
    }


init : Shared -> Nav.Key -> ( Model, Cmd Msg )
init shared key =
    ( { shared = shared
      , key = key
      , name = ""
      , description = ""
      , selectedPermissions = Set.empty
      , validationErrors = Dict.empty
      , submitting = False
      , errorMessage = Nothing
      , isDirty_ = False
      }
    , Cmd.none
    )


isDirty : Model -> Bool
isDirty =
    DirtyState.isDirty


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = UpdateName String
    | UpdateDescription String
    | TogglePermission String
    | ToggleAllPermissions String
    | SubmitForm
    | GotCreateResult (Result ApiError RoleDetail)
    | DismissMessage


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        UpdateName value ->
            let
                ( dirtyModel, dirtyCmd ) =
                    DirtyState.markDirty model
            in
            ( { dirtyModel | name = value }, dirtyCmd )

        UpdateDescription value ->
            let
                ( dirtyModel, dirtyCmd ) =
                    DirtyState.markDirty model
            in
            ( { dirtyModel | description = value }, dirtyCmd )

        TogglePermission permission ->
            let
                ( dirtyModel, dirtyCmd ) =
                    DirtyState.markDirty model

                newPermissions =
                    if Set.member permission model.selectedPermissions then
                        Set.remove permission model.selectedPermissions

                    else
                        Set.insert permission model.selectedPermissions
            in
            ( { dirtyModel | selectedPermissions = newPermissions }, dirtyCmd )

        ToggleAllPermissions resource ->
            let
                ( dirtyModel, dirtyCmd ) =
                    DirtyState.markDirty model

                allActions =
                    [ "read", "create", "update", "delete" ]

                allPermissions =
                    List.map (\action -> resource ++ ":" ++ action) allActions

                allSelected =
                    List.all (\p -> Set.member p model.selectedPermissions) allPermissions

                newPermissions =
                    if allSelected then
                        List.foldl Set.remove model.selectedPermissions allPermissions

                    else
                        List.foldl Set.insert model.selectedPermissions allPermissions
            in
            ( { dirtyModel | selectedPermissions = newPermissions }, dirtyCmd )

        SubmitForm ->
            let
                errors =
                    validateForm model
            in
            if Dict.isEmpty errors then
                let
                    body =
                        Encode.object
                            ([ ( "name", Encode.string model.name )
                             , ( "permissions", Encode.set Encode.string model.selectedPermissions )
                             ]
                                ++ (if String.isEmpty (String.trim model.description) then
                                        []

                                    else
                                        [ ( "description", Encode.string model.description ) ]
                                   )
                            )
                in
                ( { model | submitting = True, validationErrors = Dict.empty }
                , RoleApi.createRole
                    { config = Shared.toRequestConfig model.shared
                    , body = body
                    , toMsg = GotCreateResult
                    }
                )

            else
                ( { model | validationErrors = errors }, Cmd.none )

        GotCreateResult result ->
            case result of
                Ok _ ->
                    let
                        ( cleanModel, cleanCmd ) =
                            DirtyState.clearDirty model
                    in
                    ( { cleanModel | submitting = False }
                    , Cmd.batch
                        [ cleanCmd
                        , Nav.pushUrl model.key (Route.toString Route.Roles)
                        ]
                    )

                Err err ->
                    ( { model
                        | submitting = False
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ロール" } err)
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
            { fieldKey = "name", fieldLabel = "ロール名", maxLength = 100 }
            model.name
        |> validatePermissions model.selectedPermissions


validatePermissions : Set String -> Dict String String -> Dict String String
validatePermissions permissions errors =
    if Set.isEmpty permissions then
        Dict.insert "permissions" "権限を1つ以上選択してください。" errors

    else
        errors



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewBreadcrumb
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = Nothing
            , errorMessage = model.errorMessage
            }
        , viewFormContent model
        ]


viewBreadcrumb : Html Msg
viewBreadcrumb =
    nav [ class "mb-4 text-sm text-secondary-500" ]
        [ a [ href (Route.toString Route.Roles), class "hover:text-primary-600" ] [ text "ロール管理" ]
        , span [ class "mx-2" ] [ text ">" ]
        , span [] [ text "新規作成" ]
        ]


viewFormContent : Model -> Html Msg
viewFormContent model =
    Html.form [ onSubmit SubmitForm, class "mx-auto max-w-2xl space-y-6" ]
        [ h2 [ class "text-2xl font-bold text-secondary-900" ] [ text "ロールを作成" ]
        , FormField.viewTextField
            { label = "ロール名"
            , value = model.name
            , onInput = UpdateName
            , error = Dict.get "name" model.validationErrors
            , inputType = "text"
            , placeholder = "例: 編集者"
            }
        , FormField.viewTextArea
            { label = "説明（任意）"
            , value = model.description
            , onInput = UpdateDescription
            , placeholder = "ロールの説明を入力"
            }
        , div []
            [ label [ class "block text-sm font-medium text-secondary-700 mb-2" ] [ text "権限" ]
            , PermissionMatrix.view
                { selectedPermissions = model.selectedPermissions
                , onToggle = TogglePermission
                , onToggleAll = ToggleAllPermissions
                , disabled = False
                }
            , case Dict.get "permissions" model.validationErrors of
                Just errorMsg ->
                    p [ class "mt-1 text-sm text-error-600" ] [ text errorMsg ]

                Nothing ->
                    text ""
            ]
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
                , href = Route.toString Route.Roles
                }
                [ text "キャンセル" ]
            ]
        ]
