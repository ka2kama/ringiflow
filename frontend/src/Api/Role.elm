module Api.Role exposing
    ( createRole
    , deleteRole
    , getRole
    , listRoles
    , updateRole
    )

{-| ロール管理 API クライアント

BFF の `/api/v1/roles` エンドポイントへの操作を提供。

-}

import Api exposing (ApiError, RequestConfig)
import Data.Role as Role exposing (RoleDetail, RoleItem)
import Http
import Json.Encode as Encode


{-| ロール一覧を取得

`GET /api/v1/roles`

-}
listRoles :
    { config : RequestConfig
    , toMsg : Result ApiError (List RoleItem) -> msg
    }
    -> Cmd msg
listRoles { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/roles"
        , decoder = Role.roleItemListDecoder
        , toMsg = toMsg
        }


{-| ロール詳細を取得

`GET /api/v1/roles/{role_id}`

-}
getRole :
    { config : RequestConfig
    , roleId : String
    , toMsg : Result ApiError RoleDetail -> msg
    }
    -> Cmd msg
getRole { config, roleId, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/roles/" ++ roleId
        , decoder = Role.roleDetailDecoder
        , toMsg = toMsg
        }


{-| ロールを作成

`POST /api/v1/roles`

-}
createRole :
    { config : RequestConfig
    , body : Encode.Value
    , toMsg : Result ApiError RoleDetail -> msg
    }
    -> Cmd msg
createRole { config, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/roles"
        , body = Http.jsonBody body
        , decoder = Role.roleDetailDecoder
        , toMsg = toMsg
        }


{-| ロールを更新

`PATCH /api/v1/roles/{role_id}`

-}
updateRole :
    { config : RequestConfig
    , roleId : String
    , body : Encode.Value
    , toMsg : Result ApiError RoleDetail -> msg
    }
    -> Cmd msg
updateRole { config, roleId, body, toMsg } =
    Api.patch
        { config = config
        , url = "/api/v1/roles/" ++ roleId
        , body = Http.jsonBody body
        , decoder = Role.roleDetailDecoder
        , toMsg = toMsg
        }


{-| ロールを削除

`DELETE /api/v1/roles/{role_id}`

204 No Content を返す。

-}
deleteRole :
    { config : RequestConfig
    , roleId : String
    , toMsg : Result ApiError () -> msg
    }
    -> Cmd msg
deleteRole { config, roleId, toMsg } =
    Api.deleteNoContent
        { config = config
        , url = "/api/v1/roles/" ++ roleId
        , toMsg = toMsg
        }
