module Api.Workflow exposing
    ( ApproveRejectRequest
    , CreateWorkflowRequest
    , StepApproverRequest
    , SubmitWorkflowRequest
    , approveStep
    , createWorkflow
    , encodeApproveRejectRequest
    , encodeCreateRequest
    , encodeSubmitRequest
    , getWorkflow
    , listMyWorkflows
    , rejectStep
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

import Api exposing (ApiError, RequestConfig)
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

`GET /api/v1/workflows/{display_number}`

指定された display\_number のワークフローインスタンスを取得。
申請詳細画面で使用。

-}
getWorkflow :
    { config : RequestConfig
    , displayNumber : Int
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
getWorkflow { config, displayNumber, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflows/" ++ String.fromInt displayNumber
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

`POST /api/v1/workflows/{display_number}/submit`

Draft 状態のワークフローを Pending 状態に遷移させ、
承認フローを開始する。

-}
submitWorkflow :
    { config : RequestConfig
    , displayNumber : Int
    , body : SubmitWorkflowRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
submitWorkflow { config, displayNumber, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/workflows/" ++ String.fromInt displayNumber ++ "/submit"
        , body = Http.jsonBody (encodeSubmitRequest body)
        , decoder = submitResponseDecoder
        , toMsg = toMsg
        }


{-| ステップを承認

`POST /api/v1/workflows/{display_number}/steps/{step_display_number}/approve`

指定されたステップを承認する。
楽観的ロックにより、バージョン不一致の場合は 409 Conflict が返る。

-}
approveStep :
    { config : RequestConfig
    , workflowDisplayNumber : Int
    , stepDisplayNumber : Int
    , body : ApproveRejectRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
approveStep { config, workflowDisplayNumber, stepDisplayNumber, body, toMsg } =
    Api.post
        { config = config
        , url =
            "/api/v1/workflows/"
                ++ String.fromInt workflowDisplayNumber
                ++ "/steps/"
                ++ String.fromInt stepDisplayNumber
                ++ "/approve"
        , body = Http.jsonBody (encodeApproveRejectRequest body)
        , decoder = Decode.field "data" WorkflowInstance.decoder
        , toMsg = toMsg
        }


{-| ステップを却下

`POST /api/v1/workflows/{display_number}/steps/{step_display_number}/reject`

指定されたステップを却下する。
楽観的ロックにより、バージョン不一致の場合は 409 Conflict が返る。

-}
rejectStep :
    { config : RequestConfig
    , workflowDisplayNumber : Int
    , stepDisplayNumber : Int
    , body : ApproveRejectRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
rejectStep { config, workflowDisplayNumber, stepDisplayNumber, body, toMsg } =
    Api.post
        { config = config
        , url =
            "/api/v1/workflows/"
                ++ String.fromInt workflowDisplayNumber
                ++ "/steps/"
                ++ String.fromInt stepDisplayNumber
                ++ "/reject"
        , body = Http.jsonBody (encodeApproveRejectRequest body)
        , decoder = Decode.field "data" WorkflowInstance.decoder
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
    { approvers : List StepApproverRequest
    }


{-| 各承認ステップの承認者指定
-}
type alias StepApproverRequest =
    { stepId : String
    , assignedTo : String
    }


{-| 承認/却下リクエスト
-}
type alias ApproveRejectRequest =
    { version : Int
    , comment : Maybe String
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
        [ ( "approvers", Encode.list encodeStepApproverRequest req.approvers )
        ]


encodeStepApproverRequest : StepApproverRequest -> Encode.Value
encodeStepApproverRequest req =
    Encode.object
        [ ( "step_id", Encode.string req.stepId )
        , ( "assigned_to", Encode.string req.assignedTo )
        ]


encodeApproveRejectRequest : ApproveRejectRequest -> Encode.Value
encodeApproveRejectRequest req =
    let
        baseFields =
            [ ( "version", Encode.int req.version ) ]

        commentField =
            case req.comment of
                Just comment ->
                    [ ( "comment", Encode.string comment ) ]

                Nothing ->
                    []
    in
    Encode.object (baseFields ++ commentField)



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
