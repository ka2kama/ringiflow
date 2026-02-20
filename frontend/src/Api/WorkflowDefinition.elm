module Api.WorkflowDefinition exposing
    ( archiveDefinition
    , createDefinition
    , deleteDefinition
    , getDefinition
    , listDefinitions
    , publishDefinition
    )

{-| ワークフロー定義 API クライアント

BFF の `/api/v1/workflow-definitions` エンドポイントへのアクセスを提供。


## 使用例

    import Api.WorkflowDefinition as WorkflowDefinitionApi

    -- 定義一覧を取得
    WorkflowDefinitionApi.listDefinitions
        { config = requestConfig
        , toMsg = GotDefinitions
        }

    -- 特定の定義を取得
    WorkflowDefinitionApi.getDefinition
        { config = requestConfig
        , id = "uuid-string"
        , toMsg = GotDefinition
        }

-}

import Api exposing (ApiError, RequestConfig)
import Data.WorkflowDefinition as WorkflowDefinition exposing (WorkflowDefinition)
import Http
import Json.Encode as Encode


{-| ワークフロー定義一覧を取得

`GET /api/v1/workflow-definitions`

管理画面では全ステータス（Draft/Published/Archived）の定義を返す。
申請画面では BFF 側で Published のみにフィルタされる。

-}
listDefinitions :
    { config : RequestConfig
    , toMsg : Result ApiError (List WorkflowDefinition) -> msg
    }
    -> Cmd msg
listDefinitions { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflow-definitions"
        , decoder = WorkflowDefinition.listDecoder
        , toMsg = toMsg
        }


{-| ワークフロー定義を取得

`GET /api/v1/workflow-definitions/{id}`

指定された ID のワークフロー定義を取得。
定義に含まれるフォームフィールド情報を使用して動的フォームを生成。

-}
getDefinition :
    { config : RequestConfig
    , id : String
    , toMsg : Result ApiError WorkflowDefinition -> msg
    }
    -> Cmd msg
getDefinition { config, id, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflow-definitions/" ++ id
        , decoder = WorkflowDefinition.detailDecoder
        , toMsg = toMsg
        }


{-| ワークフロー定義を新規作成

`POST /api/v1/workflow-definitions`

Draft 状態で作成される。

-}
createDefinition :
    { config : RequestConfig
    , body : Encode.Value
    , toMsg : Result ApiError WorkflowDefinition -> msg
    }
    -> Cmd msg
createDefinition { config, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/workflow-definitions"
        , body = Http.jsonBody body
        , decoder = WorkflowDefinition.detailDecoder
        , toMsg = toMsg
        }


{-| ワークフロー定義を公開

`POST /api/v1/workflow-definitions/{id}/publish`

Draft → Published に遷移。version による楽観的ロックあり。

-}
publishDefinition :
    { config : RequestConfig
    , id : String
    , body : Encode.Value
    , toMsg : Result ApiError WorkflowDefinition -> msg
    }
    -> Cmd msg
publishDefinition { config, id, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/workflow-definitions/" ++ id ++ "/publish"
        , body = Http.jsonBody body
        , decoder = WorkflowDefinition.detailDecoder
        , toMsg = toMsg
        }


{-| ワークフロー定義をアーカイブ

`POST /api/v1/workflow-definitions/{id}/archive`

Published → Archived に遷移。version による楽観的ロックあり。

-}
archiveDefinition :
    { config : RequestConfig
    , id : String
    , body : Encode.Value
    , toMsg : Result ApiError WorkflowDefinition -> msg
    }
    -> Cmd msg
archiveDefinition { config, id, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/workflow-definitions/" ++ id ++ "/archive"
        , body = Http.jsonBody body
        , decoder = WorkflowDefinition.detailDecoder
        , toMsg = toMsg
        }


{-| ワークフロー定義を削除

`DELETE /api/v1/workflow-definitions/{id}`

Draft 状態の定義のみ削除可能。204 No Content を返す。

-}
deleteDefinition :
    { config : RequestConfig
    , id : String
    , toMsg : Result ApiError () -> msg
    }
    -> Cmd msg
deleteDefinition { config, id, toMsg } =
    Api.deleteNoContent
        { config = config
        , url = "/api/v1/workflow-definitions/" ++ id
        , toMsg = toMsg
        }
