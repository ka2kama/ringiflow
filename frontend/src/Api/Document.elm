module Api.Document exposing
    ( UploadRequest
    , confirmUpload
    , deleteDocument
    , encodeUploadRequest
    , listDocuments
    , listWorkflowAttachments
    , requestDownloadUrl
    , requestUploadUrl
    , requestUploadUrlForFolder
    , uploadToS3
    )

{-| ドキュメント管理 API クライアント

BFF の `/api/v1/documents` エンドポイントと S3 Presigned URL への直接アップロードを提供。


## アップロードフロー

1.  `requestUploadUrl` で Presigned URL を取得
2.  `uploadToS3` で S3 に直接 PUT（`Http.track` で進捗追跡可能）
3.  `confirmUpload` で BFF にアップロード完了を通知


## 使用例

    import Api.Document as DocumentApi

    -- アップロード URL を取得
    DocumentApi.requestUploadUrl
        { config = requestConfig
        , body = { filename = "領収書.pdf", contentType = "application/pdf", size = 1258291, workflowInstanceId = "wf-001" }
        , toMsg = GotUploadUrl
        }

-}

import Api exposing (ApiError, RequestConfig)
import Data.Document as Document exposing (Document, DownloadUrlResponse, UploadUrlResponse)
import File exposing (File)
import Http
import Json.Encode as Encode


{-| アップロード URL リクエスト
-}
type alias UploadRequest =
    { filename : String
    , contentType : String
    , size : Int
    , workflowInstanceId : String
    }


{-| アップロード URL を取得

`POST /api/v1/documents/upload-url`

Presigned URL を発行する。レスポンスの `uploadUrl` に S3 PUT でファイルをアップロードする。

-}
requestUploadUrl :
    { config : RequestConfig
    , body : UploadRequest
    , toMsg : Result ApiError UploadUrlResponse -> msg
    }
    -> Cmd msg
requestUploadUrl { config, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/documents/upload-url"
        , body = Http.jsonBody (encodeUploadRequest body)
        , decoder = Document.uploadUrlResponseDecoder
        , toMsg = toMsg
        }


{-| フォルダ用のアップロード URL を取得

`POST /api/v1/documents/upload-url`

`folder_id` を指定してアップロード URL を発行する。

-}
requestUploadUrlForFolder :
    { config : RequestConfig
    , filename : String
    , contentType : String
    , size : Int
    , folderId : String
    , toMsg : Result ApiError UploadUrlResponse -> msg
    }
    -> Cmd msg
requestUploadUrlForFolder { config, filename, contentType, size, folderId, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/documents/upload-url"
        , body =
            Http.jsonBody
                (Encode.object
                    [ ( "filename", Encode.string filename )
                    , ( "content_type", Encode.string contentType )
                    , ( "content_length", Encode.int size )
                    , ( "folder_id", Encode.string folderId )
                    ]
                )
        , decoder = Document.uploadUrlResponseDecoder
        , toMsg = toMsg
        }


{-| アップロード完了を通知

`POST /api/v1/documents/{documentId}/confirm`

S3 へのアップロード完了後、BFF にステータス更新を通知する。

-}
confirmUpload :
    { config : RequestConfig
    , documentId : String
    , toMsg : Result ApiError Document -> msg
    }
    -> Cmd msg
confirmUpload { config, documentId, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/documents/" ++ documentId ++ "/confirm"
        , body = Http.emptyBody
        , decoder = Document.decoder
        , toMsg = toMsg
        }


{-| ダウンロード URL を取得

`POST /api/v1/documents/{documentId}/download-url`

Presigned URL を発行する。レスポンスの `downloadUrl` からファイルをダウンロードできる。

-}
requestDownloadUrl :
    { config : RequestConfig
    , documentId : String
    , toMsg : Result ApiError DownloadUrlResponse -> msg
    }
    -> Cmd msg
requestDownloadUrl { config, documentId, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/documents/" ++ documentId ++ "/download-url"
        , decoder = Document.downloadUrlResponseDecoder
        , body = Http.emptyBody
        , toMsg = toMsg
        }


{-| フォルダ内のドキュメント一覧を取得

`GET /api/v1/documents?folder_id={folderId}`

-}
listDocuments :
    { config : RequestConfig
    , folderId : String
    , toMsg : Result ApiError (List Document) -> msg
    }
    -> Cmd msg
listDocuments { config, folderId, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/documents?folder_id=" ++ folderId
        , decoder = Document.listDecoder
        , toMsg = toMsg
        }


{-| ドキュメントを削除（ソフトデリート）

`DELETE /api/v1/documents/{documentId}`

204 No Content を返す。

-}
deleteDocument :
    { config : RequestConfig
    , documentId : String
    , toMsg : Result ApiError () -> msg
    }
    -> Cmd msg
deleteDocument { config, documentId, toMsg } =
    Api.deleteNoContent
        { config = config
        , url = "/api/v1/documents/" ++ documentId
        , toMsg = toMsg
        }


{-| ワークフローの添付ファイル一覧を取得

`GET /api/v1/workflows/{workflowInstanceId}/attachments`

-}
listWorkflowAttachments :
    { config : RequestConfig
    , workflowInstanceId : String
    , toMsg : Result ApiError (List Document) -> msg
    }
    -> Cmd msg
listWorkflowAttachments { config, workflowInstanceId, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflows/" ++ workflowInstanceId ++ "/attachments"
        , decoder = Document.listDecoder
        , toMsg = toMsg
        }


{-| S3 Presigned URL にファイルを直接アップロード

BFF を経由せず S3 に直接 PUT する。
`trackerId` を指定すると `Http.track` で送信進捗を購読できる。

    -- リクエスト
    uploadToS3
        { uploadUrl = presignedUrl
        , file = selectedFile
        , trackerId = "upload-doc-001"
        , toMsg = UploadComplete
        }

    -- サブスクリプション（Main.elm で）
    Http.track "upload-doc-001" GotUploadProgress

-}
uploadToS3 :
    { uploadUrl : String
    , file : File
    , trackerId : String
    , toMsg : Result Http.Error () -> msg
    }
    -> Cmd msg
uploadToS3 { uploadUrl, file, trackerId, toMsg } =
    Http.request
        { method = "PUT"
        , headers = []
        , url = uploadUrl
        , body = Http.fileBody file
        , expect = Http.expectWhatever toMsg
        , timeout = Just 300000
        , tracker = Just trackerId
        }



-- ENCODERS


encodeUploadRequest : UploadRequest -> Encode.Value
encodeUploadRequest req =
    Encode.object
        [ ( "filename", Encode.string req.filename )
        , ( "content_type", Encode.string req.contentType )
        , ( "content_length", Encode.int req.size )
        , ( "workflow_instance_id", Encode.string req.workflowInstanceId )
        ]
