module Api.AdminUser exposing
    ( createUser
    , getUserDetail
    , listAdminUsers
    , updateUser
    , updateUserStatus
    )

{-| ユーザー管理 API クライアント

BFF の `/api/v1/users` エンドポイントへの管理操作を提供。

-}

import Api exposing (ApiError, RequestConfig)
import Data.AdminUser as AdminUser exposing (AdminUserItem, CreateUserResponse, UserDetail, UserResponse)
import Http
import Json.Encode as Encode


{-| テナント内のユーザー一覧を取得（管理用）

`GET /api/v1/users`

ステータスフィルタに対応。

-}
listAdminUsers :
    { config : RequestConfig
    , statusFilter : Maybe String
    , toMsg : Result ApiError (List AdminUserItem) -> msg
    }
    -> Cmd msg
listAdminUsers { config, statusFilter, toMsg } =
    let
        queryString =
            case statusFilter of
                Just status ->
                    "?status=" ++ status

                Nothing ->
                    ""
    in
    Api.get
        { config = config
        , url = "/api/v1/users" ++ queryString
        , decoder = AdminUser.adminUserItemListDecoder
        , toMsg = toMsg
        }


{-| ユーザー詳細を取得

`GET /api/v1/users/{display_number}`

-}
getUserDetail :
    { config : RequestConfig
    , displayNumber : Int
    , toMsg : Result ApiError UserDetail -> msg
    }
    -> Cmd msg
getUserDetail { config, displayNumber, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/users/" ++ String.fromInt displayNumber
        , decoder = AdminUser.userDetailDecoder
        , toMsg = toMsg
        }


{-| ユーザーを作成

`POST /api/v1/users`

-}
createUser :
    { config : RequestConfig
    , body : Encode.Value
    , toMsg : Result ApiError CreateUserResponse -> msg
    }
    -> Cmd msg
createUser { config, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/users"
        , body = Http.jsonBody body
        , decoder = AdminUser.createUserResponseDecoder
        , toMsg = toMsg
        }


{-| ユーザーを更新

`PATCH /api/v1/users/{display_number}`

-}
updateUser :
    { config : RequestConfig
    , displayNumber : Int
    , body : Encode.Value
    , toMsg : Result ApiError UserResponse -> msg
    }
    -> Cmd msg
updateUser { config, displayNumber, body, toMsg } =
    Api.patch
        { config = config
        , url = "/api/v1/users/" ++ String.fromInt displayNumber
        , body = Http.jsonBody body
        , decoder = AdminUser.userResponseDecoder
        , toMsg = toMsg
        }


{-| ユーザーステータスを変更

`PATCH /api/v1/users/{display_number}/status`

-}
updateUserStatus :
    { config : RequestConfig
    , displayNumber : Int
    , body : Encode.Value
    , toMsg : Result ApiError UserResponse -> msg
    }
    -> Cmd msg
updateUserStatus { config, displayNumber, body, toMsg } =
    Api.patch
        { config = config
        , url = "/api/v1/users/" ++ String.fromInt displayNumber ++ "/status"
        , body = Http.jsonBody body
        , decoder = AdminUser.userResponseDecoder
        , toMsg = toMsg
        }
