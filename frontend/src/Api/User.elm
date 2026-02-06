module Api.User exposing (listUsers)

{-| ユーザー API クライアント

BFF の `/api/v1/users` エンドポイントへのアクセスを提供。


## 使用例

    import Api.User as UserApi

    UserApi.listUsers
        { config = requestConfig
        , toMsg = GotUsers
        }

-}

import Api exposing (ApiError, RequestConfig)
import Data.UserItem as UserItem exposing (UserItem)


{-| テナント内のアクティブユーザー一覧を取得

`GET /api/v1/users`

承認者選択 UI でのユーザー検索に使用。

-}
listUsers :
    { config : RequestConfig
    , toMsg : Result ApiError (List UserItem) -> msg
    }
    -> Cmd msg
listUsers { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/users"
        , decoder = UserItem.listDecoder
        , toMsg = toMsg
        }
