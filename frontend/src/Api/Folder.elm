module Api.Folder exposing
    ( createFolder
    , deleteFolder
    , listFolders
    , updateFolder
    )

{-| フォルダ管理 API クライアント

BFF の `/api/v1/folders` エンドポイントへの操作を提供。

-}

import Api exposing (ApiError, RequestConfig)
import Data.Folder as Folder exposing (Folder)
import Http
import Json.Encode as Encode


{-| フォルダ一覧を取得

`GET /api/v1/folders`

テナント内の全フォルダを path 順で返す。

-}
listFolders :
    { config : RequestConfig
    , toMsg : Result ApiError (List Folder) -> msg
    }
    -> Cmd msg
listFolders { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/folders"
        , decoder = Folder.listDecoder
        , toMsg = toMsg
        }


{-| フォルダを作成

`POST /api/v1/folders`

-}
createFolder :
    { config : RequestConfig
    , name : String
    , parentId : Maybe String
    , toMsg : Result ApiError Folder -> msg
    }
    -> Cmd msg
createFolder { config, name, parentId, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/folders"
        , body = Http.jsonBody (encodeCreateFolder name parentId)
        , decoder = Folder.singleDecoder
        , toMsg = toMsg
        }


{-| フォルダを更新（名前変更）

`PUT /api/v1/folders/{folder_id}`

-}
updateFolder :
    { config : RequestConfig
    , folderId : String
    , name : String
    , toMsg : Result ApiError Folder -> msg
    }
    -> Cmd msg
updateFolder { config, folderId, name, toMsg } =
    Api.put
        { config = config
        , url = "/api/v1/folders/" ++ folderId
        , body = Http.jsonBody (encodeUpdateFolder name)
        , decoder = Folder.singleDecoder
        , toMsg = toMsg
        }


{-| フォルダを削除

`DELETE /api/v1/folders/{folder_id}`

204 No Content を返す。空のフォルダのみ削除可能。

-}
deleteFolder :
    { config : RequestConfig
    , folderId : String
    , toMsg : Result ApiError () -> msg
    }
    -> Cmd msg
deleteFolder { config, folderId, toMsg } =
    Api.deleteNoContent
        { config = config
        , url = "/api/v1/folders/" ++ folderId
        , toMsg = toMsg
        }



-- ENCODERS


encodeCreateFolder : String -> Maybe String -> Encode.Value
encodeCreateFolder name parentId =
    Encode.object
        (( "name", Encode.string name )
            :: (case parentId of
                    Just pid ->
                        [ ( "parent_id", Encode.string pid ) ]

                    Nothing ->
                        []
               )
        )


encodeUpdateFolder : String -> Encode.Value
encodeUpdateFolder name =
    Encode.object
        [ ( "name", Encode.string name ) ]
