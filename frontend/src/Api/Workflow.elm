module Api.Workflow exposing
    ( createWorkflow
    , getWorkflow
    , listMyWorkflows
    , submitWorkflow
    )

{-| ワークフローインスタンス API クライアント

BFF の `/api/v1/workflows` エンドポイントへのアクセスを提供。


## 使用例

    import Api.Workflow as WorkflowApi

    -- 自分の申請一覧を取得
    WorkflowApi.listMyWorkflows
        { config = requestConfig
        , toMsg = GotWorkflows
        }

    -- 新規ワークフロー作成（下書き保存）
    WorkflowApi.createWorkflow
        { config = requestConfig
        , body = createRequestBody
        , toMsg = WorkflowCreated
        }

-}

import Api.Http as Api exposing (ApiError, RequestConfig)
import Data.WorkflowInstance as WorkflowInstance exposing (WorkflowInstance)
import Http
import Json.Decode as Decode exposing (Decoder)
import Json.Encode as Encode


{-| 自分のワークフロー一覧を取得

`GET /api/v1/workflows`

ログインユーザーが作成したワークフローインスタンスの一覧を返す。
申請一覧画面で使用。

-}
listMyWorkflows :
    { config : RequestConfig
    , toMsg : Result ApiError (List WorkflowInstance) -> msg
    }
    -> Cmd msg
listMyWorkflows { config, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflows"
        , decoder = WorkflowInstance.listDecoder
        , toMsg = toMsg
        }


{-| ワークフロー詳細を取得

`GET /api/v1/workflows/{id}`

指定された ID のワークフローインスタンスを取得。
申請詳細画面で使用。

-}
getWorkflow :
    { config : RequestConfig
    , id : String
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
getWorkflow { config, id, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflows/" ++ id
        , decoder = Decode.field "data" WorkflowInstance.decoder
        , toMsg = toMsg
        }


{-| 新規ワークフローを作成（下書き保存）

`POST /api/v1/workflows`

ワークフローインスタンスを Draft 状態で作成。
フォーム入力途中でも保存可能。

-}
createWorkflow :
    { config : RequestConfig
    , body : CreateWorkflowRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
createWorkflow { config, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/workflows"
        , body = Http.jsonBody (encodeCreateRequest body)
        , decoder = createResponseDecoder
        , toMsg = toMsg
        }


{-| ワークフローを申請（承認依頼）

`POST /api/v1/workflows/{id}/submit`

Draft 状態のワークフローを Pending 状態に遷移させ、
承認フローを開始する。

-}
submitWorkflow :
    { config : RequestConfig
    , id : String
    , body : SubmitWorkflowRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
submitWorkflow { config, id, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/workflows/" ++ id ++ "/submit"
        , body = Http.jsonBody (encodeSubmitRequest body)
        , decoder = submitResponseDecoder
        , toMsg = toMsg
        }



-- REQUEST/RESPONSE TYPES


{-| ワークフロー作成リクエスト
-}
type alias CreateWorkflowRequest =
    { definitionId : String
    , title : String
    , formData : Encode.Value
    }


{-| ワークフロー申請リクエスト
-}
type alias SubmitWorkflowRequest =
    { approverIds : List String
    }



-- ENCODERS


encodeCreateRequest : CreateWorkflowRequest -> Encode.Value
encodeCreateRequest req =
    Encode.object
        [ ( "definition_id", Encode.string req.definitionId )
        , ( "title", Encode.string req.title )
        , ( "form_data", req.formData )
        ]


encodeSubmitRequest : SubmitWorkflowRequest -> Encode.Value
encodeSubmitRequest req =
    Encode.object
        [ ( "approver_ids", Encode.list Encode.string req.approverIds )
        ]



-- DECODERS


{-| 作成レスポンスのデコーダー

レスポンス形式: `{ data: WorkflowInstance }`

-}
createResponseDecoder : Decoder WorkflowInstance
createResponseDecoder =
    Decode.field "data" WorkflowInstance.decoder


{-| 申請レスポンスのデコーダー

レスポンス形式: `{ data: WorkflowInstance }`

-}
submitResponseDecoder : Decoder WorkflowInstance
submitResponseDecoder =
    Decode.field "data" WorkflowInstance.decoder
