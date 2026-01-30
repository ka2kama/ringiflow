module Api.WorkflowDefinition exposing
    ( getDefinition
    , listDefinitions
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


{-| ワークフロー定義一覧を取得

`GET /api/v1/workflow-definitions`

公開済み（published）のワークフロー定義のみを返す。
新規申請時のワークフロー選択に使用。

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
        , decoder = WorkflowDefinition.decoder
        , toMsg = toMsg
        }
