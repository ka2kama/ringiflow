module Api.Auth exposing (getCsrfToken, getMe)

{-| 認証 API クライアント

BFF の `/api/v1/auth` エンドポイントへのアクセスを提供。


## 使用例

    import Api.Auth as AuthApi

    -- CSRF トークンを取得
    AuthApi.getCsrfToken
        { config = requestConfig
        , toMsg = GotCsrfToken
        }

    -- ユーザー情報を取得
    AuthApi.getMe
        { config = requestConfig
        , toMsg = GotUser
        }

-}

import Api exposing (ApiError, RequestConfig)
import Json.Decode as Decode exposing (Decoder)
import Shared exposing (User)


{-| CSRF トークンを取得

`GET /api/v1/auth/csrf`

状態変更リクエスト（POST/PUT/DELETE）で必要な CSRF トークンを取得する。
セッションが存在しない場合は 401 Unauthorized が返される。

-}
getCsrfToken :
    { config : RequestConfig
    , toMsg : Result ApiError String -> msg
    }
    -> Cmd msg
getCsrfToken { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/auth/csrf"
        , decoder = csrfTokenDecoder
        , toMsg = toMsg
        }


{-| 現在のユーザー情報を取得

`GET /api/v1/auth/me`

セッションが有効な場合、ユーザー情報（ID、メール、名前、ロール）を返す。
未認証の場合は 401 Unauthorized が返される。

-}
getMe :
    { config : RequestConfig
    , toMsg : Result ApiError User -> msg
    }
    -> Cmd msg
getMe { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/auth/me"
        , decoder = userDecoder
        , toMsg = toMsg
        }



-- DECODERS


{-| CSRF トークンレスポンスのデコーダー

レスポンス形式: `{ "data": { "token": "..." } }`

-}
csrfTokenDecoder : Decoder String
csrfTokenDecoder =
    Decode.at [ "data", "token" ] Decode.string


{-| ユーザー情報レスポンスのデコーダー

レスポンス形式: `{ "data": { "id": "...", "email": "...", "name": "...", "roles": [...] } }`

-}
userDecoder : Decoder User
userDecoder =
    Decode.field "data"
        (Decode.map4 User
            (Decode.field "id" Decode.string)
            (Decode.field "email" Decode.string)
            (Decode.field "name" Decode.string)
            (Decode.field "roles" (Decode.list Decode.string))
        )
