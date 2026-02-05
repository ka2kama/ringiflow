module Api.Task exposing (getTaskByDisplayNumbers, listMyTasks)

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
    TaskApi.getTaskByDisplayNumbers
        { config = requestConfig
        , workflowDisplayNumber = 1
        , stepDisplayNumber = 1
        , toMsg = GotTaskDetail
        }

-}

import Api exposing (ApiError, RequestConfig)
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

`GET /api/v1/workflows/{workflowDisplayNumber}/tasks/{stepDisplayNumber}`

ワークフローとステップの表示用番号を使ってタスク詳細を取得。
タスク詳細画面で使用。

-}
getTaskByDisplayNumbers :
    { config : RequestConfig
    , workflowDisplayNumber : Int
    , stepDisplayNumber : Int
    , toMsg : Result ApiError TaskDetail -> msg
    }
    -> Cmd msg
getTaskByDisplayNumbers { config, workflowDisplayNumber, stepDisplayNumber, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflows/" ++ String.fromInt workflowDisplayNumber ++ "/tasks/" ++ String.fromInt stepDisplayNumber
        , decoder = Task.detailDecoder
        , toMsg = toMsg
        }
