module Api.Workflow exposing
    ( ApproveRejectRequest
    , CreateWorkflowRequest
    , PostCommentRequest
    , ResubmitRequest
    , StepApproverRequest
    , SubmitWorkflowRequest
    , approveStep
    , createWorkflow
    , encodeApproveRejectRequest
    , encodeCreateRequest
    , encodePostCommentRequest
    , encodeResubmitRequest
    , encodeSubmitRequest
    , getWorkflow
    , listComments
    , listMyWorkflows
    , postComment
    , rejectStep
    , requestChangesStep
    , resubmitWorkflow
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
import Data.WorkflowComment as WorkflowComment exposing (WorkflowComment)
import Data.WorkflowInstance as WorkflowInstance exposing (WorkflowInstance)
import Http
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
        , decoder = WorkflowInstance.detailDecoder
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
        , decoder = WorkflowInstance.detailDecoder
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
        , decoder = WorkflowInstance.detailDecoder
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
        , decoder = WorkflowInstance.detailDecoder
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
        , decoder = WorkflowInstance.detailDecoder
        , toMsg = toMsg
        }


{-| ステップを差し戻し

`POST /api/v1/workflows/{display_number}/steps/{step_display_number}/request-changes`

指定されたステップを差し戻す。
楽観的ロックにより、バージョン不一致の場合は 409 Conflict が返る。

-}
requestChangesStep :
    { config : RequestConfig
    , workflowDisplayNumber : Int
    , stepDisplayNumber : Int
    , body : ApproveRejectRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
requestChangesStep { config, workflowDisplayNumber, stepDisplayNumber, body, toMsg } =
    Api.post
        { config = config
        , url =
            "/api/v1/workflows/"
                ++ String.fromInt workflowDisplayNumber
                ++ "/steps/"
                ++ String.fromInt stepDisplayNumber
                ++ "/request-changes"
        , body = Http.jsonBody (encodeApproveRejectRequest body)
        , decoder = WorkflowInstance.detailDecoder
        , toMsg = toMsg
        }


{-| ワークフローを再申請

`POST /api/v1/workflows/{display_number}/resubmit`

差し戻しされたワークフローを修正して再申請する。
楽観的ロックにより、バージョン不一致の場合は 409 Conflict が返る。

-}
resubmitWorkflow :
    { config : RequestConfig
    , displayNumber : Int
    , body : ResubmitRequest
    , toMsg : Result ApiError WorkflowInstance -> msg
    }
    -> Cmd msg
resubmitWorkflow { config, displayNumber, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/workflows/" ++ String.fromInt displayNumber ++ "/resubmit"
        , body = Http.jsonBody (encodeResubmitRequest body)
        , decoder = WorkflowInstance.detailDecoder
        , toMsg = toMsg
        }


{-| コメント一覧を取得

`GET /api/v1/workflows/{display_number}/comments`

ワークフローに紐づくコメント一覧を返す。

-}
listComments :
    { config : RequestConfig
    , displayNumber : Int
    , toMsg : Result ApiError (List WorkflowComment) -> msg
    }
    -> Cmd msg
listComments { config, displayNumber, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflows/" ++ String.fromInt displayNumber ++ "/comments"
        , decoder = WorkflowComment.listDecoder
        , toMsg = toMsg
        }


{-| コメントを投稿

`POST /api/v1/workflows/{display_number}/comments`

ワークフローにコメントを投稿する。

-}
postComment :
    { config : RequestConfig
    , displayNumber : Int
    , body : PostCommentRequest
    , toMsg : Result ApiError WorkflowComment -> msg
    }
    -> Cmd msg
postComment { config, displayNumber, body, toMsg } =
    Api.post
        { config = config
        , url = "/api/v1/workflows/" ++ String.fromInt displayNumber ++ "/comments"
        , body = Http.jsonBody (encodePostCommentRequest body)
        , decoder = WorkflowComment.detailDecoder
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


{-| 再申請リクエスト
-}
type alias ResubmitRequest =
    { version : Int
    , formData : Encode.Value
    , approvers : List StepApproverRequest
    }


{-| コメント投稿リクエスト
-}
type alias PostCommentRequest =
    { body : String
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


encodeResubmitRequest : ResubmitRequest -> Encode.Value
encodeResubmitRequest req =
    Encode.object
        [ ( "version", Encode.int req.version )
        , ( "form_data", req.formData )
        , ( "approvers", Encode.list encodeStepApproverRequest req.approvers )
        ]


encodePostCommentRequest : PostCommentRequest -> Encode.Value
encodePostCommentRequest req =
    Encode.object
        [ ( "body", Encode.string req.body )
        ]
