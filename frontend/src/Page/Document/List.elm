module Page.Document.List exposing (Model, Msg, init, subscriptions, update, updateShared, view)

{-| ドキュメント管理画面

フォルダツリー + ファイル一覧のレイアウト。
フォルダ選択でファイル一覧が切り替わる。

-}

import Api exposing (ApiError)
import Api.Document as DocumentApi
import Api.ErrorMessage as ErrorMessage
import Api.Folder as FolderApi
import Component.EmptyState as EmptyState
import Component.ErrorState as ErrorState
import Component.FolderTree as FolderTree exposing (FolderNode(..), childrenOf, folderOf)
import Component.LoadingSpinner as LoadingSpinner
import Data.Document exposing (Document)
import Data.Folder exposing (Folder)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick, stopPropagationOn)
import Json.Decode as Decode
import RemoteData exposing (RemoteData(..))
import Set exposing (Set)
import Shared exposing (Shared)



-- MODEL


type alias Model =
    { shared : Shared
    , folders : RemoteData ApiError (List Folder)
    , selectedFolderId : Maybe String
    , expandedFolderIds : Set String
    , documents : RemoteData ApiError (List Document)
    }


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , folders = Loading
      , selectedFolderId = Nothing
      , expandedFolderIds = Set.empty
      , documents = NotAsked
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
    | SelectFolder String
    | ToggleFolder String
    | GotDocuments (Result ApiError (List Document))
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

        SelectFolder folderId ->
            ( { model
                | selectedFolderId = Just folderId
                , documents = Loading
              }
            , DocumentApi.listDocuments
                { config = Shared.toRequestConfig model.shared
                , folderId = folderId
                , toMsg = GotDocuments
                }
            )

        ToggleFolder folderId ->
            let
                newExpanded =
                    if Set.member folderId model.expandedFolderIds then
                        Set.remove folderId model.expandedFolderIds

                    else
                        Set.insert folderId model.expandedFolderIds
            in
            ( { model | expandedFolderIds = newExpanded }, Cmd.none )

        GotDocuments result ->
            case result of
                Ok docs ->
                    ( { model | documents = Success docs }, Cmd.none )

                Err err ->
                    ( { model | documents = Failure err }, Cmd.none )

        Refresh ->
            ( { model | folders = Loading, selectedFolderId = Nothing, documents = NotAsked }
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

        Success folders ->
            let
                tree =
                    FolderTree.buildTree folders
            in
            div [ class "flex gap-6" ]
                [ viewFolderTreePanel model tree
                , viewDocumentPanel model
                ]


{-| フォルダツリーパネル（左側）
-}
viewFolderTreePanel : Model -> List FolderNode -> Html Msg
viewFolderTreePanel model tree =
    div [ class "w-72 shrink-0 rounded-lg border border-secondary-200 bg-white" ]
        [ div [ class "border-b border-secondary-200 px-4 py-3" ]
            [ h2 [ class "text-sm font-semibold text-secondary-700" ] [ text "フォルダ" ] ]
        , div [ class "p-2" ]
            [ if List.isEmpty tree then
                p [ class "px-2 py-4 text-center text-sm text-secondary-400" ]
                    [ text "フォルダがありません" ]

              else
                ul [ class "space-y-0.5" ]
                    (List.map (viewFolderNode model 0) tree)
            ]
        ]


{-| フォルダツリーノード（再帰的に描画）
-}
viewFolderNode : Model -> Int -> FolderNode -> Html Msg
viewFolderNode model depth node =
    let
        folder =
            folderOf node

        children =
            childrenOf node

        hasChildren =
            not (List.isEmpty children)

        isExpanded =
            Set.member folder.id model.expandedFolderIds

        isSelected =
            model.selectedFolderId == Just folder.id

        paddingLeft =
            String.fromInt (depth * 16 + 8) ++ "px"

        selectedClass =
            if isSelected then
                " bg-primary-50 text-primary-700"

            else
                " text-secondary-700 hover:bg-secondary-50"
    in
    li []
        [ div
            [ class ("flex items-center rounded px-2 py-1.5 text-sm cursor-pointer select-none" ++ selectedClass)
            , style "padding-left" paddingLeft
            , onClick (SelectFolder folder.id)
            ]
            [ if hasChildren then
                button
                    [ class "mr-1 h-4 w-4 shrink-0 text-secondary-400"
                    , stopPropagationOn "click"
                        (Decode.succeed ( ToggleFolder folder.id, True ))
                    ]
                    [ text
                        (if isExpanded then
                            "▼"

                         else
                            "▶"
                        )
                    ]

              else
                span [ class "mr-1 h-4 w-4 shrink-0" ] []
            , span [ class "truncate" ] [ text folder.name ]
            ]
        , if hasChildren && isExpanded then
            ul [ class "space-y-0.5" ]
                (List.map (viewFolderNode model (depth + 1)) children)

          else
            text ""
        ]


{-| ドキュメント一覧パネル（右側）
-}
viewDocumentPanel : Model -> Html Msg
viewDocumentPanel model =
    div [ class "min-w-0 flex-1 rounded-lg border border-secondary-200 bg-white" ]
        [ div [ class "border-b border-secondary-200 px-4 py-3" ]
            [ h2 [ class "text-sm font-semibold text-secondary-700" ] [ text "ファイル一覧" ] ]
        , div [ class "p-4" ]
            [ viewDocumentContent model ]
        ]


{-| ドキュメント一覧の内容
-}
viewDocumentContent : Model -> Html Msg
viewDocumentContent model =
    case model.selectedFolderId of
        Nothing ->
            p [ class "py-8 text-center text-sm text-secondary-400" ]
                [ text "フォルダを選択してください" ]

        Just _ ->
            case model.documents of
                NotAsked ->
                    text ""

                Loading ->
                    LoadingSpinner.view

                Failure err ->
                    ErrorState.view
                        { message = ErrorMessage.toUserMessage { entityName = "ドキュメント" } err
                        , onRefresh = Refresh
                        }

                Success docs ->
                    if List.isEmpty docs then
                        EmptyState.view
                            { message = "ファイルがありません"
                            , description = Just "このフォルダにはファイルがまだアップロードされていません"
                            }

                    else
                        viewDocumentTable docs


{-| ドキュメント一覧テーブル
-}
viewDocumentTable : List Document -> Html Msg
viewDocumentTable docs =
    div [ class "overflow-x-auto" ]
        [ table [ class "w-full" ]
            [ thead [ class "bg-secondary-50" ]
                [ tr []
                    [ th [ class "px-4 py-2 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "ファイル名" ]
                    , th [ class "px-4 py-2 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "サイズ" ]
                    , th [ class "px-4 py-2 text-left text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "ステータス" ]
                    ]
                ]
            , tbody [ class "divide-y divide-secondary-200" ]
                (List.map viewDocumentRow docs)
            ]
        ]


{-| ドキュメント行
-}
viewDocumentRow : Document -> Html Msg
viewDocumentRow doc =
    tr [ class "hover:bg-secondary-50 transition-colors" ]
        [ td [ class "px-4 py-3 text-sm text-secondary-900" ] [ text doc.filename ]
        , td [ class "px-4 py-3 text-sm text-secondary-500" ] [ text (formatFileSize doc.size) ]
        , td [ class "px-4 py-3 text-sm text-secondary-500" ] [ text doc.status ]
        ]


{-| ファイルサイズを人間が読める形式に変換
-}
formatFileSize : Int -> String
formatFileSize bytes =
    if bytes < 1024 then
        String.fromInt bytes ++ " B"

    else if bytes < 1024 * 1024 then
        String.fromFloat (toFloat bytes / 1024 |> roundTo 1) ++ " KB"

    else
        String.fromFloat (toFloat bytes / (1024 * 1024) |> roundTo 1) ++ " MB"


roundTo : Int -> Float -> Float
roundTo decimals value =
    let
        factor =
            toFloat (10 ^ decimals)
    in
    toFloat (round (value * factor)) / factor
