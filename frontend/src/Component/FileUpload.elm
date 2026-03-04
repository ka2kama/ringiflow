module Component.FileUpload exposing
    ( FileError(..)
    , Model
    , Msg
    , UploadProgress
    , UploadingFile
    , completedCount
    , init
    , startPendingUploads
    , subscriptions
    , update
    , validateFile
    , validateFileCount
    , view
    )

{-| ファイルアップロードコンポーネント

ドラッグ&ドロップとファイル選択ダイアログによるファイルアップロードを提供する。
Presigned URL 方式で S3 に直接アップロードし、進捗バーを表示する。


## アップロードフロー

1.  ファイル選択 → バリデーション → `Pending` 状態でリストに追加
2.  `workflowInstanceId` がある場合 → `requestUploadUrl` で URL 取得 → `RequestingUrl`
3.  URL 取得成功 → `uploadToS3` で S3 に PUT → `Uploading Float`
4.  S3 アップロード完了 → `confirmUpload` で BFF に通知 → `Confirming`
5.  確認完了 → `Completed`


## 使用例

    import Component.FileUpload as FileUpload

    -- Model に含める
    type alias Model =
        { fileUpload : FileUpload.Model
        }

    -- init
    FileUpload.init fileConfig Nothing

    -- view（Html.map で Msg を変換）
    FileUpload.view model.fileUpload |> Html.map FileUploadMsg

    -- subscriptions（Sub.map で変換）
    FileUpload.subscriptions model.fileUpload |> Sub.map FileUploadMsg

-}

import Api exposing (ApiError, RequestConfig)
import Api.Document as DocumentApi
import Data.Document exposing (Document, UploadUrlResponse)
import Data.FormField exposing (FileConfig)
import File exposing (File)
import File.Select as Select
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick, preventDefaultOn)
import Http
import Json.Decode as Decode
import Util.Format



-- TYPES


{-| ファイルバリデーションエラー
-}
type FileError
    = InvalidType
    | FileTooLarge
    | TooManyFiles


{-| アップロード進捗
-}
type UploadProgress
    = Pending
    | RequestingUrl
    | Uploading Float
    | Confirming
    | Completed
    | Failed String


{-| アップロード中のファイル

`id` はコンポーネント内でのユニーク識別子（カウンタベース）。
同名ファイルを複数追加した場合でも個別に識別できる。

-}
type alias UploadingFile =
    { id : Int
    , file : File
    , documentId : Maybe String
    , name : String
    , size : Int
    , progress : UploadProgress
    }


{-| コンポーネントの状態
-}
type alias Model =
    { files : List UploadingFile
    , dragOver : Bool
    , config : FileConfig
    , workflowInstanceId : Maybe String
    , nextId : Int
    }


{-| コンポーネントのメッセージ
-}
type Msg
    = SelectFiles
    | FilesSelected File (List File)
    | DragEnter
    | DragLeave
    | FilesDropped File (List File)
    | GotUploadUrl Int (Result ApiError UploadUrlResponse)
    | GotUploadProgress String Http.Progress
    | UploadCompleted String (Result Http.Error ())
    | ConfirmCompleted String (Result ApiError Document)
    | RemoveFile Int



-- INIT


{-| 初期化
-}
init : FileConfig -> Maybe String -> Model
init config workflowInstanceId =
    { files = []
    , dragOver = False
    , config = config
    , workflowInstanceId = workflowInstanceId
    , nextId = 0
    }



-- VALIDATION


{-| 単一ファイルのバリデーション

Content-Type とファイルサイズを検証する。
`allowedTypes` が空の場合は全形式を許可する。

-}
validateFile : FileConfig -> { name : String, size : Int, mime : String } -> List FileError
validateFile config fileMeta =
    let
        typeError =
            if List.isEmpty config.allowedTypes then
                []

            else if List.member fileMeta.mime config.allowedTypes then
                []

            else
                [ InvalidType ]

        sizeError =
            if fileMeta.size > config.maxFileSize then
                [ FileTooLarge ]

            else
                []
    in
    typeError ++ sizeError


{-| ファイル数のバリデーション

既存ファイル数と新規ファイル数の合計が上限を超えていないか検証する。

-}
validateFileCount : FileConfig -> { existingCount : Int, newCount : Int } -> Maybe FileError
validateFileCount config { existingCount, newCount } =
    if existingCount + newCount > config.maxFiles then
        Just TooManyFiles

    else
        Nothing



-- UPDATE


{-| コンポーネントの更新

`RequestConfig` は API 呼び出しに必要。親ページの Model から毎回渡す。

-}
update : RequestConfig -> Msg -> Model -> ( Model, Cmd Msg )
update requestConfig msg model =
    case msg of
        SelectFiles ->
            ( model
            , Select.files (acceptTypes model.config) FilesSelected
            )

        FilesSelected first rest ->
            addFiles requestConfig (first :: rest) model

        DragEnter ->
            ( { model | dragOver = True }, Cmd.none )

        DragLeave ->
            ( { model | dragOver = False }, Cmd.none )

        FilesDropped first rest ->
            addFiles requestConfig (first :: rest) { model | dragOver = False }

        GotUploadUrl fileId result ->
            case result of
                Ok response ->
                    let
                        updatedFiles =
                            List.map
                                (\f ->
                                    if f.id == fileId then
                                        { f
                                            | documentId = Just response.documentId
                                            , progress = Uploading 0.0
                                        }

                                    else
                                        f
                                )
                                model.files

                        uploadCmd =
                            updatedFiles
                                |> List.filter (\f -> f.id == fileId)
                                |> List.head
                                |> Maybe.map
                                    (\f ->
                                        DocumentApi.uploadToS3
                                            { uploadUrl = response.uploadUrl
                                            , file = f.file
                                            , trackerId = "upload-" ++ response.documentId
                                            , toMsg = UploadCompleted response.documentId
                                            }
                                    )
                                |> Maybe.withDefault Cmd.none
                    in
                    ( { model | files = updatedFiles }, uploadCmd )

                Err _ ->
                    ( { model
                        | files =
                            updateFileProgressById fileId (Failed "アップロード URL の取得に失敗しました") model.files
                      }
                    , Cmd.none
                    )

        GotUploadProgress documentId progress ->
            case progress of
                Http.Sending { sent, size } ->
                    let
                        fraction =
                            if size == 0 then
                                1.0

                            else
                                toFloat sent / toFloat size
                    in
                    ( { model
                        | files =
                            updateFileProgressByDocumentId documentId (Uploading fraction) model.files
                      }
                    , Cmd.none
                    )

                Http.Receiving _ ->
                    ( model, Cmd.none )

        UploadCompleted documentId result ->
            case result of
                Ok () ->
                    ( { model
                        | files =
                            updateFileProgressByDocumentId documentId Confirming model.files
                      }
                    , DocumentApi.confirmUpload
                        { config = requestConfig
                        , documentId = documentId
                        , toMsg = ConfirmCompleted documentId
                        }
                    )

                Err _ ->
                    ( { model
                        | files =
                            updateFileProgressByDocumentId documentId (Failed "ファイルのアップロードに失敗しました") model.files
                      }
                    , Cmd.none
                    )

        ConfirmCompleted documentId result ->
            case result of
                Ok _ ->
                    ( { model
                        | files =
                            updateFileProgressByDocumentId documentId Completed model.files
                      }
                    , Cmd.none
                    )

                Err _ ->
                    ( { model
                        | files =
                            updateFileProgressByDocumentId documentId (Failed "アップロードの確認に失敗しました") model.files
                      }
                    , Cmd.none
                    )

        RemoveFile fileId ->
            ( { model | files = List.filter (\f -> f.id /= fileId) model.files }
            , Cmd.none
            )



-- UPDATE HELPERS


{-| ファイルを追加し、workflowInstanceId がある場合はアップロードを開始
-}
addFiles : RequestConfig -> List File -> Model -> ( Model, Cmd Msg )
addFiles requestConfig newFiles model =
    let
        -- ファイル数上限チェック: 残り容量分だけ追加可能
        remainingCapacity =
            Basics.max 0 (model.config.maxFiles - List.length model.files)

        validFiles =
            newFiles
                |> List.filter
                    (\f ->
                        List.isEmpty
                            (validateFile model.config
                                { name = File.name f
                                , size = File.size f
                                , mime = File.mime f
                                }
                            )
                    )
                |> List.take remainingCapacity

        -- カウンタベースの ID を付与
        uploadingFiles =
            List.indexedMap
                (\i f ->
                    { id = model.nextId + i
                    , file = f
                    , documentId = Nothing
                    , name = File.name f
                    , size = File.size f
                    , progress = Pending
                    }
                )
                validFiles

        newNextId =
            model.nextId + List.length validFiles

        uploadCmds =
            case model.workflowInstanceId of
                Just wfId ->
                    uploadingFiles
                        |> List.map
                            (\uf ->
                                DocumentApi.requestUploadUrl
                                    { config = requestConfig
                                    , body =
                                        { filename = uf.name
                                        , contentType = File.mime uf.file
                                        , size = uf.size
                                        , workflowInstanceId = wfId
                                        }
                                    , toMsg = GotUploadUrl uf.id
                                    }
                            )
                        |> Cmd.batch

                Nothing ->
                    Cmd.none

        filesWithProgress =
            case model.workflowInstanceId of
                Just _ ->
                    List.map (\f -> { f | progress = RequestingUrl }) uploadingFiles

                Nothing ->
                    uploadingFiles
    in
    ( { model | files = model.files ++ filesWithProgress, nextId = newNextId }
    , uploadCmds
    )


{-| Pending ファイルのアップロードを開始

下書き保存成功後に呼び出す。workflowInstanceId を設定し、
Pending 状態のファイルのアップロードを開始する。

-}
startPendingUploads : RequestConfig -> String -> Model -> ( Model, Cmd Msg )
startPendingUploads requestConfig workflowInstanceId model =
    let
        updatedModel =
            { model | workflowInstanceId = Just workflowInstanceId }

        pendingFiles =
            model.files
                |> List.filter (\f -> f.progress == Pending)

        uploadCmds =
            pendingFiles
                |> List.map
                    (\f ->
                        DocumentApi.requestUploadUrl
                            { config = requestConfig
                            , body =
                                { filename = f.name
                                , contentType = File.mime f.file
                                , size = f.size
                                , workflowInstanceId = workflowInstanceId
                                }
                            , toMsg = GotUploadUrl f.id
                            }
                    )
                |> Cmd.batch

        filesWithProgress =
            List.map
                (\f ->
                    if f.progress == Pending then
                        { f | progress = RequestingUrl }

                    else
                        f
                )
                updatedModel.files
    in
    ( { updatedModel | files = filesWithProgress }
    , uploadCmds
    )


{-| 完了済みファイルの数を取得

バリデーション時に使用する。

-}
completedCount : Model -> Int
completedCount model =
    model.files
        |> List.filter (\f -> f.progress == Completed)
        |> List.length


updateFileProgressById : Int -> UploadProgress -> List UploadingFile -> List UploadingFile
updateFileProgressById fileId progress files =
    List.map
        (\f ->
            if f.id == fileId then
                { f | progress = progress }

            else
                f
        )
        files


updateFileProgressByDocumentId : String -> UploadProgress -> List UploadingFile -> List UploadingFile
updateFileProgressByDocumentId documentId progress files =
    List.map
        (\f ->
            if f.documentId == Just documentId then
                { f | progress = progress }

            else
                f
        )
        files


acceptTypes : FileConfig -> List String
acceptTypes config =
    config.allowedTypes



-- SUBSCRIPTIONS


{-| アップロード進捗の購読

アップロード中のファイルがある場合のみ `Http.track` で購読する。

-}
subscriptions : Model -> Sub Msg
subscriptions model =
    model.files
        |> List.filterMap
            (\f ->
                case ( f.progress, f.documentId ) of
                    ( Uploading _, Just docId ) ->
                        Just (Http.track ("upload-" ++ docId) (GotUploadProgress docId))

                    _ ->
                        Nothing
            )
        |> Sub.batch



-- VIEW


{-| コンポーネントの描画
-}
view : Model -> Html Msg
view model =
    div [ class "space-y-3" ]
        [ viewDropZone model.dragOver
        , if List.isEmpty model.files then
            text ""

          else
            viewFileList model.files
        ]


{-| ドロップゾーン
-}
viewDropZone : Bool -> Html Msg
viewDropZone isDragOver =
    div
        [ class
            ("flex flex-col items-center justify-center rounded-lg border-2 border-dashed p-6 transition-colors cursor-pointer"
                ++ (if isDragOver then
                        " border-primary-500 bg-primary-50"

                    else
                        " border-secondary-300 hover:border-primary-400 hover:bg-secondary-50"
                   )
            )
        , onClick SelectFiles
        , hijackOn "dragenter" (Decode.succeed DragEnter)
        , hijackOn "dragover" (Decode.succeed DragEnter)
        , hijackOn "dragleave" (Decode.succeed DragLeave)
        , hijackOn "drop" dropDecoder
        ]
        [ p [ class "text-sm text-secondary-600" ]
            [ text "ファイルをドラッグ&ドロップ、またはクリックして選択" ]
        ]


{-| ファイルリスト
-}
viewFileList : List UploadingFile -> Html Msg
viewFileList files =
    ul [ class "space-y-2" ]
        (List.map viewFileItem files)


{-| 個別ファイル表示
-}
viewFileItem : UploadingFile -> Html Msg
viewFileItem file =
    li [ class "flex items-center gap-3 rounded-lg border border-secondary-200 bg-white p-3" ]
        [ div [ class "min-w-0 flex-1" ]
            [ div [ class "flex items-center justify-between" ]
                [ span [ class "truncate text-sm font-medium text-secondary-900" ]
                    [ text file.name ]
                , span [ class "ml-2 shrink-0 text-xs text-secondary-500" ]
                    [ text (Util.Format.formatFileSize file.size) ]
                ]
            , viewProgress file.progress
            ]
        , button
            [ onClick (RemoveFile file.id)
            , class "shrink-0 border-0 bg-transparent cursor-pointer text-secondary-400 hover:text-error-600 transition-colors text-lg rounded outline-none focus-visible:ring-2 focus-visible:ring-primary-500"
            , type_ "button"
            , attribute "aria-label" ("ファイル「" ++ file.name ++ "」を削除")
            ]
            [ text "×" ]
        ]


{-| 進捗表示
-}
viewProgress : UploadProgress -> Html msg
viewProgress progress =
    case progress of
        Pending ->
            p [ class "mt-1 text-xs text-secondary-500" ]
                [ text "保存後にアップロードされます" ]

        RequestingUrl ->
            p [ class "mt-1 text-xs text-secondary-500" ]
                [ text "準備中..." ]

        Uploading fraction ->
            div [ class "mt-1" ]
                [ div [ class "h-1.5 w-full rounded-full bg-secondary-200" ]
                    [ div
                        [ class "h-1.5 rounded-full bg-primary-500 transition-all"
                        , style "width" (String.fromFloat (fraction * 100) ++ "%")
                        ]
                        []
                    ]
                , p [ class "mt-0.5 text-xs text-secondary-500" ]
                    [ text (String.fromInt (round (fraction * 100)) ++ "%") ]
                ]

        Confirming ->
            p [ class "mt-1 text-xs text-secondary-500" ]
                [ text "確認中..." ]

        Completed ->
            p [ class "mt-1 text-xs text-success-600" ]
                [ text "アップロード完了" ]

        Failed errorMsg ->
            p [ class "mt-1 text-xs text-error-600" ]
                [ text errorMsg ]



-- VIEW HELPERS


{-| ドラッグ&ドロップのイベントデコーダー
-}
dropDecoder : Decode.Decoder Msg
dropDecoder =
    Decode.at [ "dataTransfer", "files" ] (Decode.oneOrMore FilesDropped File.decoder)


{-| イベントのデフォルト動作を抑制して Msg を発行する

D&D では `dragenter`, `dragover`, `dragleave`, `drop` のデフォルト動作を
すべて抑制する必要がある（ブラウザがファイルを直接開くのを防ぐ）。

-}
hijackOn : String -> Decode.Decoder msg -> Attribute msg
hijackOn event decoder =
    preventDefaultOn event (Decode.map (\msg -> ( msg, True )) decoder)
