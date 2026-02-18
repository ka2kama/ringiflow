module Page.AuditLog.List exposing (Model, Msg, init, update, updateShared, view)

{-| 監査ログ一覧画面

フィルタ（期間・ユーザー・アクション・結果）、カーソルページネーション、
インライン展開（アコーディオン）に対応。

-}

import Api exposing (ApiError)
import Api.AdminUser as AdminUserApi
import Api.AuditLog as AuditLogApi exposing (AuditLogFilter)
import Api.ErrorMessage as ErrorMessage
import Component.Badge as Badge
import Component.Button as Button
import Component.EmptyState as EmptyState
import Component.ErrorState as ErrorState
import Component.LoadingSpinner as LoadingSpinner
import Data.AdminUser exposing (AdminUserItem)
import Data.AuditLog exposing (AuditLogItem, AuditLogList, actionToJapanese)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick, onInput)
import Json.Encode as Encode
import RemoteData exposing (RemoteData(..))
import Shared exposing (Shared)
import Util.DateFormat as DateFormat



-- MODEL


type alias Model =
    { shared : Shared
    , auditLogs : RemoteData ApiError AuditLogList
    , users : RemoteData ApiError (List AdminUserItem)
    , filterFrom : String
    , filterTo : String
    , filterActorId : String
    , filterAction : String
    , filterResult : String
    , expandedId : Maybe String
    }


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , auditLogs = Loading
      , users = Loading
      , filterFrom = ""
      , filterTo = ""
      , filterActorId = ""
      , filterAction = ""
      , filterResult = ""
      , expandedId = Nothing
      }
    , Cmd.batch
        [ AuditLogApi.listAuditLogs
            { config = Shared.toRequestConfig shared
            , filter = AuditLogApi.emptyFilter
            , toMsg = GotAuditLogs
            }
        , AdminUserApi.listAdminUsers
            { config = Shared.toRequestConfig shared
            , statusFilter = Nothing
            , toMsg = GotUsers
            }
        ]
    )


updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = GotAuditLogs (Result ApiError AuditLogList)
    | GotUsers (Result ApiError (List AdminUserItem))
    | UpdateFilterFrom String
    | UpdateFilterTo String
    | UpdateFilterActorId String
    | UpdateFilterAction String
    | UpdateFilterResult String
    | ApplyFilter
    | LoadNextPage String
    | ToggleExpand String
    | Refresh


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotAuditLogs result ->
            case result of
                Ok auditLogList ->
                    ( { model | auditLogs = Success auditLogList }, Cmd.none )

                Err err ->
                    ( { model | auditLogs = Failure err }, Cmd.none )

        GotUsers result ->
            case result of
                Ok users ->
                    ( { model | users = Success users }, Cmd.none )

                Err _ ->
                    -- ユーザー一覧の取得に失敗してもログ一覧は表示する
                    ( { model | users = Success [] }, Cmd.none )

        UpdateFilterFrom value ->
            ( { model | filterFrom = value }, Cmd.none )

        UpdateFilterTo value ->
            ( { model | filterTo = value }, Cmd.none )

        UpdateFilterActorId value ->
            ( { model | filterActorId = value }, Cmd.none )

        UpdateFilterAction value ->
            ( { model | filterAction = value }, Cmd.none )

        UpdateFilterResult value ->
            ( { model | filterResult = value }, Cmd.none )

        ApplyFilter ->
            ( { model | auditLogs = Loading, expandedId = Nothing }
            , AuditLogApi.listAuditLogs
                { config = Shared.toRequestConfig model.shared
                , filter = buildFilter model Nothing
                , toMsg = GotAuditLogs
                }
            )

        LoadNextPage cursor ->
            ( { model | auditLogs = Loading, expandedId = Nothing }
            , AuditLogApi.listAuditLogs
                { config = Shared.toRequestConfig model.shared
                , filter = buildFilter model (Just cursor)
                , toMsg = GotAuditLogs
                }
            )

        ToggleExpand logId ->
            let
                newExpanded =
                    if model.expandedId == Just logId then
                        Nothing

                    else
                        Just logId
            in
            ( { model | expandedId = newExpanded }, Cmd.none )

        Refresh ->
            ( { model | auditLogs = Loading, expandedId = Nothing }
            , AuditLogApi.listAuditLogs
                { config = Shared.toRequestConfig model.shared
                , filter = buildFilter model Nothing
                , toMsg = GotAuditLogs
                }
            )


buildFilter : Model -> Maybe String -> AuditLogFilter
buildFilter model cursor =
    { cursor = cursor
    , limit = 20
    , from = nonEmpty model.filterFrom
    , to = nonEmpty model.filterTo
    , actorId = nonEmpty model.filterActorId
    , action = nonEmpty model.filterAction
    , result = nonEmpty model.filterResult
    }


nonEmpty : String -> Maybe String
nonEmpty str =
    if String.isEmpty str then
        Nothing

    else
        Just str



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewHeader
        , viewFilters model
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "mb-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ]
            [ text "監査ログ" ]
        ]


viewFilters : Model -> Html Msg
viewFilters model =
    let
        users =
            RemoteData.withDefault [] model.users
    in
    div [ class "mb-6 rounded-lg border border-secondary-200 bg-white p-4" ]
        [ div [ class "grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3" ]
            [ viewDateFilter "開始日" model.filterFrom UpdateFilterFrom
            , viewDateFilter "終了日" model.filterTo UpdateFilterTo
            , viewSelectFilter "ユーザー"
                model.filterActorId
                UpdateFilterActorId
                (List.map (\u -> ( u.id, u.name )) users)
            , viewSelectFilter "アクション"
                model.filterAction
                UpdateFilterAction
                actionOptions
            , viewSelectFilter "結果"
                model.filterResult
                UpdateFilterResult
                [ ( "success", "成功" ), ( "failure", "失敗" ) ]
            , div [ class "flex items-end" ]
                [ Button.view
                    { variant = Button.Primary
                    , disabled = False
                    , onClick = ApplyFilter
                    }
                    [ text "検索" ]
                ]
            ]
        ]


viewDateFilter : String -> String -> (String -> Msg) -> Html Msg
viewDateFilter labelText filterValue onInputMsg =
    div []
        [ label [ class "block text-sm font-medium text-secondary-700 mb-1" ] [ text labelText ]
        , input
            [ type_ "date"
            , value filterValue
            , onInput onInputMsg
            , class "w-full rounded-lg border border-secondary-300 px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
            ]
            []
        ]


viewSelectFilter : String -> String -> (String -> Msg) -> List ( String, String ) -> Html Msg
viewSelectFilter labelText filterValue onInputMsg options =
    div []
        [ label [ class "block text-sm font-medium text-secondary-700 mb-1" ] [ text labelText ]
        , select
            [ class "w-full rounded-lg border border-secondary-300 px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
            , onInput onInputMsg
            , value filterValue
            ]
            (option [ Html.Attributes.value "" ] [ text "すべて" ]
                :: List.map
                    (\( optValue, optLabel ) ->
                        option [ Html.Attributes.value optValue ] [ text optLabel ]
                    )
                    options
            )
        ]


actionOptions : List ( String, String )
actionOptions =
    [ ( "user.create", "ユーザー作成" )
    , ( "user.update", "ユーザー更新" )
    , ( "user.deactivate", "ユーザー無効化" )
    , ( "user.activate", "ユーザー有効化" )
    , ( "role.create", "ロール作成" )
    , ( "role.update", "ロール更新" )
    , ( "role.delete", "ロール削除" )
    ]


viewContent : Model -> Html Msg
viewContent model =
    case model.auditLogs of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure err ->
            ErrorState.view
                { message = ErrorMessage.toUserMessage { entityName = "監査ログ" } err
                , onRefresh = Refresh
                }

        Success auditLogList ->
            viewAuditLogList model auditLogList


viewAuditLogList : Model -> AuditLogList -> Html Msg
viewAuditLogList model auditLogList =
    if List.isEmpty auditLogList.data then
        EmptyState.view
            { message = "監査ログが見つかりません。"
            , description = Nothing
            }

    else
        div []
            [ viewAuditLogTable model auditLogList.data
            , viewPagination auditLogList.nextCursor
            ]


viewAuditLogTable : Model -> List AuditLogItem -> Html Msg
viewAuditLogTable model logs =
    div [ class "overflow-x-auto rounded-lg border border-secondary-200" ]
        [ table [ class "w-full" ]
            [ thead [ class "bg-secondary-50" ]
                [ tr []
                    [ th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "日時" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "ユーザー" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "アクション" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "対象" ]
                    , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "結果" ]
                    ]
                ]
            , tbody [ class "divide-y divide-secondary-200 bg-white" ]
                (List.concatMap (viewAuditLogRow model) logs)
            ]
        ]


viewAuditLogRow : Model -> AuditLogItem -> List (Html Msg)
viewAuditLogRow model logItem =
    let
        isExpanded =
            model.expandedId == Just logItem.id

        zone =
            Shared.zone model.shared
    in
    tr
        [ class "hover:bg-secondary-50 transition-colors cursor-pointer"
        , onClick (ToggleExpand logItem.id)
        ]
        [ td [ class "px-4 py-3 text-sm text-secondary-500" ]
            [ text (DateFormat.formatDateTime zone logItem.createdAt) ]
        , td [ class "px-4 py-3 text-sm text-secondary-900" ]
            [ text logItem.actorName ]
        , td [ class "px-4 py-3 text-sm text-secondary-500" ]
            [ text (actionToJapanese logItem.action) ]
        , td [ class "px-4 py-3 text-sm text-secondary-500" ]
            [ text (logItem.resourceType ++ " / " ++ logItem.resourceId) ]
        , td [ class "px-4 py-3 text-sm" ]
            [ Badge.view (resultToBadge logItem.result) ]
        ]
        :: (if isExpanded then
                [ viewExpandedDetail logItem ]

            else
                []
           )


viewExpandedDetail : AuditLogItem -> Html Msg
viewExpandedDetail logItem =
    tr [ class "bg-secondary-50" ]
        [ td [ colspan 5, class "px-4 py-4" ]
            [ div [ class "space-y-2 text-sm" ]
                [ viewDetailField "リソース ID" logItem.resourceId
                , viewDetailField "リクエスト元 IP" (Maybe.withDefault "未取得" logItem.sourceIp)
                , case logItem.detail of
                    Just detailValue ->
                        div []
                            [ dt [ class "font-medium text-secondary-500" ] [ text "操作詳細" ]
                            , dd [ class "mt-1 rounded bg-white p-2 font-mono text-xs text-secondary-700 border border-secondary-200 whitespace-pre-wrap" ]
                                [ text (Encode.encode 2 detailValue) ]
                            ]

                    Nothing ->
                        text ""
                ]
            ]
        ]


viewDetailField : String -> String -> Html msg
viewDetailField labelText fieldValue =
    div []
        [ dt [ class "font-medium text-secondary-500" ] [ text labelText ]
        , dd [ class "mt-1 text-secondary-700" ] [ text fieldValue ]
        ]


viewPagination : Maybe String -> Html Msg
viewPagination nextCursor =
    case nextCursor of
        Just cursor ->
            div [ class "mt-4 flex justify-center" ]
                [ Button.view
                    { variant = Button.Outline
                    , disabled = False
                    , onClick = LoadNextPage cursor
                    }
                    [ text "次のページ" ]
                ]

        Nothing ->
            text ""


resultToBadge : String -> { colorClass : String, label : String }
resultToBadge result =
    case result of
        "success" ->
            { colorClass = "bg-success-100 text-success-800 border-success-200", label = "成功" }

        "failure" ->
            { colorClass = "bg-error-100 text-error-800 border-error-200", label = "失敗" }

        _ ->
            { colorClass = "bg-secondary-100 text-secondary-800 border-secondary-200", label = result }
