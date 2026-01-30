module Api.Dashboard exposing (getStats)

{-| ダッシュボード API クライアント

BFF の `/api/v1/dashboard` エンドポイントへのアクセスを提供。


## 使用例

    import Api.Dashboard as DashboardApi

    -- ダッシュボード統計情報を取得
    DashboardApi.getStats
        { config = requestConfig
        , toMsg = GotDashboardStats
        }

-}

import Api exposing (ApiError, RequestConfig)
import Data.Dashboard as Dashboard exposing (DashboardStats)


{-| ダッシュボード統計情報を取得

`GET /api/v1/dashboard/stats`

ログインユーザーの承認待ちタスク数、申請中ワークフロー数、本日完了数を返す。
ホーム画面で使用。

-}
getStats :
    { config : RequestConfig
    , toMsg : Result ApiError DashboardStats -> msg
    }
    -> Cmd msg
getStats { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/dashboard/stats"
        , decoder = Dashboard.decoder
        , toMsg = toMsg
        }
