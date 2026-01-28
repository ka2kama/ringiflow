module Api.Auth exposing (getCsrfToken)

{-| 認証 API クライアント

BFF の `/auth` エンドポイントへのアクセスを提供。


## 使用例

    import Api.Auth as AuthApi

    -- CSRF トークンを取得
    AuthApi.getCsrfToken
        { config = requestConfig
        , toMsg = GotCsrfToken
        }

-}

import Api.Http as Api exposing (ApiError, RequestConfig)
import Json.Decode as Decode exposing (Decoder)


{-| CSRF トークンを取得

`GET /auth/csrf`

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
        , url = "/auth/csrf"
        , decoder = csrfTokenDecoder
        , toMsg = toMsg
        }



-- DECODERS


{-| CSRF トークンレスポンスのデコーダー

レスポンス形式: `{ "data": { "token": "..." } }`

-}
csrfTokenDecoder : Decoder String
csrfTokenDecoder =
    Decode.at [ "data", "token" ] Decode.string
