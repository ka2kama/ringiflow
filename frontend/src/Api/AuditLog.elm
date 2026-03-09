module Api.AuditLog exposing (AuditLogFilter, emptyFilter, listAuditLogs)

{-| 監査ログ API クライアント

BFF の `/api/v1/audit-logs` エンドポイントへのアクセスを提供。

-}

import Api exposing (ApiError, RequestConfig)
import Data.AuditLog as AuditLog exposing (AuditLogList)


{-| 監査ログのフィルタ条件
-}
type alias AuditLogFilter =
    { cursor : Maybe String
    , limit : Int
    , from : Maybe String
    , to : Maybe String
    , actorId : Maybe String
    , action : Maybe String
    , result : Maybe String
    }


{-| デフォルトのフィルタ条件
-}
emptyFilter : AuditLogFilter
emptyFilter =
    { cursor = Nothing
    , limit = 20
    , from = Nothing
    , to = Nothing
    , actorId = Nothing
    , action = Nothing
    , result = Nothing
    }


{-| 監査ログ一覧を取得

`GET /api/v1/audit-logs`

フィルタ条件をクエリパラメータとして付与する。

-}
listAuditLogs :
    { config : RequestConfig
    , filter : AuditLogFilter
    , toMsg : Result ApiError AuditLogList -> msg
    }
    -> Cmd msg
listAuditLogs { config, filter, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/audit-logs" ++ buildFilterQuery filter
        , decoder = AuditLog.auditLogListDecoder
        , toMsg = toMsg
        }


{-| フィルタ条件をクエリ文字列に変換

Nothing のフィールドは除外する。

-}
buildFilterQuery : AuditLogFilter -> String
buildFilterQuery filter =
    let
        params =
            List.filterMap identity
                [ filter.cursor |> Maybe.map (\v -> "cursor=" ++ v)
                , Just ("limit=" ++ String.fromInt filter.limit)
                , filter.from |> Maybe.map (\v -> "from=" ++ v)
                , filter.to |> Maybe.map (\v -> "to=" ++ v)
                , filter.actorId |> Maybe.map (\v -> "actor_id=" ++ v)
                , filter.action |> Maybe.map (\v -> "action=" ++ v)
                , filter.result |> Maybe.map (\v -> "result=" ++ v)
                ]
    in
    case params of
        [] ->
            ""

        _ ->
            "?" ++ String.join "&" params
