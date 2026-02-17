module Page.Role.Edit exposing (Model, Msg, init, isDirty, update, updateShared, view)

{-| ロール編集・詳細画面

DJ-3: 詳細と編集を統合。
システムロールは読み取り専用表示、カスタムロールは編集可能フォーム表示。
保存成功後はロール一覧に遷移する。

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.Role as RoleApi
import Browser.Navigation as Nav
import Component.Button as Button
import Component.FormField as FormField
import Component.LoadingSpinner as LoadingSpinner
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
import RemoteData exposing (RemoteData(..))
import Route
import Set exposing (Set)
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , key : Nav.Key
    , roleId : String
    , role : RemoteData ApiError RoleDetail
    , name : String
    , description : String
    , selectedPermissions : Set String
    , validationErrors : Dict String String
    , submitting : Bool
    , errorMessage : Maybe String
    , isDirty_ : Bool
    , isReadOnly : Bool
    }


init : Shared -> Nav.Key -> String -> ( Model, Cmd Msg )
init shared key roleId =
    ( { shared = shared
      , key = key
      , roleId = roleId
      , role = Loading
      , name = ""
      , description = ""
      , selectedPermissions = Set.empty
      , validationErrors = Dict.empty
      , submitting = False
      , errorMessage = Nothing
      , isDirty_ = False
      , isReadOnly = False
      }
    , RoleApi.getRole
        { config = Shared.toRequestConfig shared
        , roleId = roleId
        , toMsg = GotRole
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
    = GotRole (Result ApiError RoleDetail)
    | UpdateName String
    | UpdateDescription String
    | TogglePermission String
    | ToggleAllPermissions String
    | SubmitForm
    | GotUpdateResult (Result ApiError RoleDetail)
    | DismissMessage


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotRole result ->
            case result of
                Ok role ->
                    ( { model
                        | role = Success role
                        , name = role.name
                        , description = Maybe.withDefault "" role.description
                        , selectedPermissions = Set.fromList role.permissions
                        , isReadOnly = role.isSystem
                      }
                    , Cmd.none
                    )

                Err err ->
                    ( { model | role = Failure err }, Cmd.none )

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
                , RoleApi.updateRole
                    { config = Shared.toRequestConfig model.shared
                    , roleId = model.roleId
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
        [ viewBreadcrumb model
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = Nothing
            , errorMessage = model.errorMessage
            }
        , viewContent model
        ]


viewBreadcrumb : Model -> Html Msg
viewBreadcrumb model =
    let
        lastSegment =
            if model.isReadOnly then
                "詳細"

            else
                "編集"
    in
    nav [ class "mb-4 text-sm text-secondary-500" ]
        [ a [ href (Route.toString Route.Roles), class "hover:text-primary-600" ] [ text "ロール管理" ]
        , span [ class "mx-2" ] [ text ">" ]
        , span [] [ text lastSegment ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.role of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure err ->
            div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
                [ text (ErrorMessage.toUserMessage { entityName = "ロール" } err) ]

        Success _ ->
            if model.isReadOnly then
                viewReadOnly model

            else
                viewEditForm model


{-| システムロール: 読み取り専用表示
-}
viewReadOnly : Model -> Html Msg
viewReadOnly model =
    div [ class "mx-auto max-w-2xl space-y-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ] [ text "ロール詳細" ]
        , FormField.viewReadOnlyField "ロール名" model.name
        , FormField.viewReadOnlyField "説明"
            (if String.isEmpty model.description then
                "—"

             else
                model.description
            )
        , div []
            [ label [ class "block text-sm font-medium text-secondary-700 mb-2" ] [ text "権限" ]
            , PermissionMatrix.view
                { selectedPermissions = model.selectedPermissions
                , onToggle = TogglePermission
                , onToggleAll = ToggleAllPermissions
                , disabled = True
                }
            ]
        , div [ class "flex gap-3" ]
            [ Button.link
                { variant = Button.Outline
                , href = Route.toString Route.Roles
                }
                [ text "ロール一覧に戻る" ]
            ]
        ]


{-| カスタムロール: 編集可能フォーム
-}
viewEditForm : Model -> Html Msg
viewEditForm model =
    Html.form [ onSubmit SubmitForm, class "mx-auto max-w-2xl space-y-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ] [ text "ロールを編集" ]
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
                        "保存中..."

                     else
                        "保存"
                    )
                ]
            , Button.link
                { variant = Button.Outline
                , href = Route.toString Route.Roles
                }
                [ text "キャンセル" ]
            ]
        ]
