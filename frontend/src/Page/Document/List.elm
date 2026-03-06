module Page.Document.List exposing (FolderDialog(..), Model, Msg(..), PendingDelete(..), init, subscriptions, update, updateShared, view)

{-| ドキュメント管理画面

フォルダツリー + ファイル一覧のレイアウト。
フォルダ選択でファイル一覧が切り替わる。

-}

import Api exposing (ApiError)
import Api.Document as DocumentApi
import Api.ErrorMessage as ErrorMessage
import Api.Folder as FolderApi
import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Component.EmptyState as EmptyState
import Component.ErrorState as ErrorState
import Component.FolderTree as FolderTree exposing (FolderNode, childrenOf, folderOf)
import Component.LoadingSpinner as LoadingSpinner
import Component.MessageAlert as MessageAlert
import Data.Document exposing (Document, DownloadUrlResponse, UploadUrlResponse)
import Data.Folder exposing (Folder)
import File exposing (File)
import File.Select as Select
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick, onInput, onSubmit, stopPropagationOn)
import Http
import Json.Decode as Decode
import Ports
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
    , folderDialog : Maybe FolderDialog
    , pendingDelete : Maybe PendingDelete
    , selectedFile : Maybe File
    , isUploading : Bool
    , successMessage : Maybe String
    , errorMessage : Maybe String
    }


{-| フォルダ作成/名前変更ダイアログの状態
-}
type FolderDialog
    = CreateFolderDialog { name : String, parentId : Maybe String, isSubmitting : Bool }
    | RenameFolderDialog { folderId : String, name : String, isSubmitting : Bool }


{-| 削除対象（フォルダまたはドキュメント）
-}
type PendingDelete
    = DeleteFolder Folder
    | DeleteDocument Document


init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , folders = Loading
      , selectedFolderId = Nothing
      , expandedFolderIds = Set.empty
      , documents = NotAsked
      , folderDialog = Nothing
      , pendingDelete = Nothing
      , selectedFile = Nothing
      , isUploading = False
      , successMessage = Nothing
      , errorMessage = Nothing
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
      -- フォルダ CRUD
    | OpenCreateFolderDialog
    | OpenRenameFolderDialog Folder
    | UpdateFolderDialogName String
    | SubmitFolderDialog
    | CloseFolderDialog
    | GotCreateFolderResult (Result ApiError Folder)
    | GotRenameFolderResult (Result ApiError Folder)
    | ClickDeleteFolder Folder
    | ClickDeleteDocument Document
    | ConfirmDelete
    | CancelDelete
    | GotDeleteFolderResult (Result ApiError ())
    | GotDeleteDocumentResult (Result ApiError ())
    | DismissMessage
      -- ファイル操作
    | SelectFile
    | FileSelected File
    | GotUploadUrl (Result ApiError UploadUrlResponse)
    | GotS3UploadResult String (Result Http.Error ())
    | GotConfirmUpload (Result ApiError Document)
    | ClickDownload Document
    | GotDownloadUrl (Result ApiError DownloadUrlResponse)


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

        -- フォルダ作成ダイアログ
        OpenCreateFolderDialog ->
            ( { model
                | folderDialog =
                    Just (CreateFolderDialog { name = "", parentId = model.selectedFolderId, isSubmitting = False })
              }
            , Cmd.none
            )

        OpenRenameFolderDialog folder ->
            ( { model
                | folderDialog =
                    Just (RenameFolderDialog { folderId = folder.id, name = folder.name, isSubmitting = False })
              }
            , Cmd.none
            )

        UpdateFolderDialogName name ->
            ( { model | folderDialog = Maybe.map (updateDialogName name) model.folderDialog }
            , Cmd.none
            )

        SubmitFolderDialog ->
            case model.folderDialog of
                Just (CreateFolderDialog dialog) ->
                    if String.isEmpty (String.trim dialog.name) then
                        ( model, Cmd.none )

                    else
                        ( { model | folderDialog = Just (CreateFolderDialog { dialog | isSubmitting = True }) }
                        , FolderApi.createFolder
                            { config = Shared.toRequestConfig model.shared
                            , name = String.trim dialog.name
                            , parentId = dialog.parentId
                            , toMsg = GotCreateFolderResult
                            }
                        )

                Just (RenameFolderDialog dialog) ->
                    if String.isEmpty (String.trim dialog.name) then
                        ( model, Cmd.none )

                    else
                        ( { model | folderDialog = Just (RenameFolderDialog { dialog | isSubmitting = True }) }
                        , FolderApi.updateFolder
                            { config = Shared.toRequestConfig model.shared
                            , folderId = dialog.folderId
                            , name = String.trim dialog.name
                            , toMsg = GotRenameFolderResult
                            }
                        )

                Nothing ->
                    ( model, Cmd.none )

        CloseFolderDialog ->
            ( { model | folderDialog = Nothing }, Cmd.none )

        GotCreateFolderResult result ->
            case result of
                Ok _ ->
                    ( { model
                        | folderDialog = Nothing
                        , folders = Loading
                        , successMessage = Just "フォルダを作成しました"
                      }
                    , FolderApi.listFolders
                        { config = Shared.toRequestConfig model.shared
                        , toMsg = GotFolders
                        }
                    )

                Err err ->
                    ( { model
                        | folderDialog = Maybe.map setDialogNotSubmitting model.folderDialog
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "フォルダ" } err)
                      }
                    , Cmd.none
                    )

        GotRenameFolderResult result ->
            case result of
                Ok _ ->
                    ( { model
                        | folderDialog = Nothing
                        , folders = Loading
                        , successMessage = Just "フォルダ名を変更しました"
                      }
                    , FolderApi.listFolders
                        { config = Shared.toRequestConfig model.shared
                        , toMsg = GotFolders
                        }
                    )

                Err err ->
                    ( { model
                        | folderDialog = Maybe.map setDialogNotSubmitting model.folderDialog
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "フォルダ" } err)
                      }
                    , Cmd.none
                    )

        ClickDeleteFolder folder ->
            ( { model | pendingDelete = Just (DeleteFolder folder) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ClickDeleteDocument doc ->
            ( { model | pendingDelete = Just (DeleteDocument doc) }
            , Ports.showModalDialog ConfirmDialog.dialogId
            )

        ConfirmDelete ->
            case model.pendingDelete of
                Just (DeleteFolder folder) ->
                    ( { model | pendingDelete = Nothing }
                    , FolderApi.deleteFolder
                        { config = Shared.toRequestConfig model.shared
                        , folderId = folder.id
                        , toMsg = GotDeleteFolderResult
                        }
                    )

                Just (DeleteDocument doc) ->
                    ( { model | pendingDelete = Nothing }
                    , DocumentApi.deleteDocument
                        { config = Shared.toRequestConfig model.shared
                        , documentId = doc.id
                        , toMsg = GotDeleteDocumentResult
                        }
                    )

                Nothing ->
                    ( model, Cmd.none )

        CancelDelete ->
            ( { model | pendingDelete = Nothing }, Cmd.none )

        GotDeleteFolderResult result ->
            case result of
                Ok () ->
                    ( { model
                        | folders = Loading
                        , selectedFolderId = Nothing
                        , documents = NotAsked
                        , successMessage = Just "フォルダを削除しました"
                      }
                    , FolderApi.listFolders
                        { config = Shared.toRequestConfig model.shared
                        , toMsg = GotFolders
                        }
                    )

                Err err ->
                    ( { model | errorMessage = Just (ErrorMessage.toUserMessage { entityName = "フォルダ" } err) }
                    , Cmd.none
                    )

        DismissMessage ->
            ( { model | successMessage = Nothing, errorMessage = Nothing }, Cmd.none )

        -- ファイル操作
        SelectFile ->
            ( model, Select.file [] FileSelected )

        FileSelected file ->
            case model.selectedFolderId of
                Just folderId ->
                    ( { model | selectedFile = Just file, isUploading = True }
                    , DocumentApi.requestUploadUrlForFolder
                        { config = Shared.toRequestConfig model.shared
                        , filename = File.name file
                        , contentType = File.mime file
                        , size = File.size file
                        , folderId = folderId
                        , toMsg = GotUploadUrl
                        }
                    )

                Nothing ->
                    ( model, Cmd.none )

        GotUploadUrl result ->
            case result of
                Ok response ->
                    case model.selectedFile of
                        Just file ->
                            ( model
                            , DocumentApi.uploadToS3
                                { uploadUrl = response.uploadUrl
                                , file = file
                                , trackerId = "upload-doc-" ++ response.documentId
                                , toMsg = GotS3UploadResult response.documentId
                                }
                            )

                        Nothing ->
                            ( { model
                                | isUploading = False
                                , errorMessage = Just "アップロードするファイルが見つかりません"
                              }
                            , Cmd.none
                            )

                Err err ->
                    ( { model
                        | isUploading = False
                        , selectedFile = Nothing
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ファイル" } err)
                      }
                    , Cmd.none
                    )

        GotS3UploadResult documentId result ->
            case result of
                Ok () ->
                    ( model
                    , DocumentApi.confirmUpload
                        { config = Shared.toRequestConfig model.shared
                        , documentId = documentId
                        , toMsg = GotConfirmUpload
                        }
                    )

                Err _ ->
                    ( { model
                        | isUploading = False
                        , selectedFile = Nothing
                        , errorMessage = Just "ファイルのアップロードに失敗しました"
                      }
                    , Cmd.none
                    )

        GotConfirmUpload result ->
            case result of
                Ok _ ->
                    case model.selectedFolderId of
                        Just folderId ->
                            ( { model
                                | isUploading = False
                                , selectedFile = Nothing
                                , successMessage = Just "ファイルをアップロードしました"
                                , documents = Loading
                              }
                            , DocumentApi.listDocuments
                                { config = Shared.toRequestConfig model.shared
                                , folderId = folderId
                                , toMsg = GotDocuments
                                }
                            )

                        Nothing ->
                            ( { model | isUploading = False, selectedFile = Nothing }, Cmd.none )

                Err err ->
                    ( { model
                        | isUploading = False
                        , selectedFile = Nothing
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ファイル" } err)
                      }
                    , Cmd.none
                    )

        ClickDownload doc ->
            ( model
            , DocumentApi.requestDownloadUrl
                { config = Shared.toRequestConfig model.shared
                , documentId = doc.id
                , toMsg = GotDownloadUrl
                }
            )

        GotDownloadUrl result ->
            case result of
                Ok response ->
                    ( model, Ports.openUrl response.downloadUrl )

                Err err ->
                    ( { model | errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ファイル" } err) }
                    , Cmd.none
                    )

        GotDeleteDocumentResult result ->
            case result of
                Ok () ->
                    case model.selectedFolderId of
                        Just folderId ->
                            ( { model
                                | documents = Loading
                                , successMessage = Just "ファイルを削除しました"
                              }
                            , DocumentApi.listDocuments
                                { config = Shared.toRequestConfig model.shared
                                , folderId = folderId
                                , toMsg = GotDocuments
                                }
                            )

                        Nothing ->
                            ( model, Cmd.none )

                Err err ->
                    ( { model | errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ファイル" } err) }
                    , Cmd.none
                    )


updateDialogName : String -> FolderDialog -> FolderDialog
updateDialogName name dialog =
    case dialog of
        CreateFolderDialog d ->
            CreateFolderDialog { d | name = name }

        RenameFolderDialog d ->
            RenameFolderDialog { d | name = name }


setDialogNotSubmitting : FolderDialog -> FolderDialog
setDialogNotSubmitting dialog =
    case dialog of
        CreateFolderDialog d ->
            CreateFolderDialog { d | isSubmitting = False }

        RenameFolderDialog d ->
            RenameFolderDialog { d | isSubmitting = False }



-- SUBSCRIPTIONS


subscriptions : Sub Msg
subscriptions =
    Sub.none



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ viewHeader model
        , MessageAlert.view
            { onDismiss = DismissMessage
            , successMessage = model.successMessage
            , errorMessage = model.errorMessage
            }
        , viewContent model
        , viewFolderDialog model.folderDialog
        , viewDeleteConfirmDialog model.pendingDelete
        ]


viewHeader : Model -> Html Msg
viewHeader model =
    div [ class "mb-6 flex items-center justify-between" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ]
            [ text "ドキュメント管理" ]
        , if Shared.isAdmin model.shared then
            Button.view
                { variant = Button.Primary
                , disabled = False
                , onClick = OpenCreateFolderDialog
                }
                [ text "フォルダ作成" ]

          else
            text ""
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
            , span [ class "flex-1 truncate" ] [ text folder.name ]
            , if isSelected && Shared.isAdmin model.shared then
                span [ class "ml-1 flex shrink-0 gap-0.5" ]
                    [ button
                        [ class "rounded p-0.5 text-xs text-secondary-400 hover:text-secondary-600"
                        , stopPropagationOn "click"
                            (Decode.succeed ( OpenRenameFolderDialog folder, True ))
                        , title "名前変更"
                        ]
                        [ text "✏" ]
                    , button
                        [ class "rounded p-0.5 text-xs text-secondary-400 hover:text-error-600"
                        , stopPropagationOn "click"
                            (Decode.succeed ( ClickDeleteFolder folder, True ))
                        , title "削除"
                        ]
                        [ text "🗑" ]
                    ]

              else
                text ""
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
        [ div [ class "flex items-center justify-between border-b border-secondary-200 px-4 py-3" ]
            [ h2 [ class "text-sm font-semibold text-secondary-700" ] [ text "ファイル一覧" ]
            , case model.selectedFolderId of
                Just _ ->
                    Button.view
                        { variant = Button.Primary
                        , disabled = model.isUploading
                        , onClick = SelectFile
                        }
                        [ text
                            (if model.isUploading then
                                "アップロード中..."

                             else
                                "アップロード"
                            )
                        ]

                Nothing ->
                    text ""
            ]
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
                    , th [ class "px-4 py-2 text-right text-xs font-medium uppercase tracking-wider text-secondary-600" ] [ text "操作" ]
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
        , td [ class "px-4 py-3 text-right" ]
            [ button
                [ class "mr-2 text-sm text-primary-600 hover:text-primary-800"
                , onClick (ClickDownload doc)
                ]
                [ text "ダウンロード" ]
            , button
                [ class "text-sm text-error-600 hover:text-error-800"
                , onClick (ClickDeleteDocument doc)
                ]
                [ text "削除" ]
            ]
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



-- FOLDER DIALOGS


{-| フォルダ作成/名前変更ダイアログ
-}
viewFolderDialog : Maybe FolderDialog -> Html Msg
viewFolderDialog maybeDialog =
    case maybeDialog of
        Nothing ->
            text ""

        Just dialog ->
            let
                dialogTitle =
                    case dialog of
                        CreateFolderDialog _ ->
                            "フォルダ作成"

                        RenameFolderDialog _ ->
                            "フォルダ名変更"

                ( dialogName, isSubmitting ) =
                    case dialog of
                        CreateFolderDialog d ->
                            ( d.name, d.isSubmitting )

                        RenameFolderDialog d ->
                            ( d.name, d.isSubmitting )

                submitLabel =
                    case dialog of
                        CreateFolderDialog _ ->
                            "作成"

                        RenameFolderDialog _ ->
                            "変更"
            in
            div [ class "fixed inset-0 z-50 flex items-center justify-center bg-black/50" ]
                [ Html.form
                    [ class "w-full max-w-md rounded-lg bg-white p-6 shadow-xl"
                    , onSubmit SubmitFolderDialog
                    ]
                    [ h2 [ class "text-lg font-semibold text-secondary-900" ] [ text dialogTitle ]
                    , div [ class "mt-4" ]
                        [ label [ class "block text-sm font-medium text-secondary-700 mb-1", Html.Attributes.for "folder-name-input" ] [ text "フォルダ名" ]
                        , input
                            [ id "folder-name-input"
                            , type_ "text"
                            , value dialogName
                            , onInput UpdateFolderDialogName
                            , class "w-full rounded-lg border border-secondary-300 px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
                            , Html.Attributes.autofocus True
                            , placeholder "フォルダ名を入力"
                            ]
                            []
                        ]
                    , div [ class "mt-6 flex justify-end gap-3" ]
                        [ Button.view
                            { variant = Button.Outline
                            , disabled = isSubmitting
                            , onClick = CloseFolderDialog
                            }
                            [ text "キャンセル" ]
                        , Button.view
                            { variant = Button.Primary
                            , disabled = isSubmitting || String.isEmpty (String.trim dialogName)
                            , onClick = SubmitFolderDialog
                            }
                            [ text
                                (if isSubmitting then
                                    "処理中..."

                                 else
                                    submitLabel
                                )
                            ]
                        ]
                    ]
                ]


{-| 削除確認ダイアログ（フォルダ/ドキュメント共通）
-}
viewDeleteConfirmDialog : Maybe PendingDelete -> Html Msg
viewDeleteConfirmDialog maybePending =
    case maybePending of
        Just (DeleteFolder folder) ->
            ConfirmDialog.view
                { title = "フォルダの削除"
                , message = "「" ++ folder.name ++ "」を削除しますか？フォルダ内のファイルも削除されます。"
                , confirmLabel = "削除する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmDelete
                , onCancel = CancelDelete
                , actionStyle = ConfirmDialog.Destructive
                }

        Just (DeleteDocument doc) ->
            ConfirmDialog.view
                { title = "ファイルの削除"
                , message = "「" ++ doc.filename ++ "」を削除しますか？この操作は取り消せません。"
                , confirmLabel = "削除する"
                , cancelLabel = "キャンセル"
                , onConfirm = ConfirmDelete
                , onCancel = CancelDelete
                , actionStyle = ConfirmDialog.Destructive
                }

        Nothing ->
            text ""
