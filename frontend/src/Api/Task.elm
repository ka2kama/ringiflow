module Api.Task exposing (getTask, listMyTasks)

{-| タスク API クライアント

BFF の `/api/v1/tasks` エンドポイントへのアクセスを提供。


## 使用例

    import Api.Task as TaskApi

    -- 自分のタスク一覧を取得
    TaskApi.listMyTasks
        { config = requestConfig
        , toMsg = GotTasks
        }

    -- タスク詳細を取得
    TaskApi.getTask
        { config = requestConfig
        , id = "step-uuid"
        , toMsg = GotTaskDetail
        }

-}

import Api.Http as Api exposing (ApiError, RequestConfig)
import Data.Task as Task exposing (TaskDetail, TaskItem)


{-| 自分のタスク一覧を取得

`GET /api/v1/tasks/my`

ログインユーザーにアサインされたアクティブなタスクの一覧を返す。
タスク一覧画面で使用。

-}
listMyTasks :
    { config : RequestConfig
    , toMsg : Result ApiError (List TaskItem) -> msg
    }
    -> Cmd msg
listMyTasks { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/tasks/my"
        , decoder = Task.listDecoder
        , toMsg = toMsg
        }


{-| タスク詳細を取得

`GET /api/v1/tasks/{id}`

指定された ID のタスク詳細（承認ステップ + ワークフロー全体）を取得。
タスク詳細画面で使用。

-}
getTask :
    { config : RequestConfig
    , id : String
    , toMsg : Result ApiError TaskDetail -> msg
    }
    -> Cmd msg
getTask { config, id, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/tasks/" ++ id
        , decoder = Task.detailDecoder
        , toMsg = toMsg
        }
