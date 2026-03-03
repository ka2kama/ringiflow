module Page.Workflow.New.Api exposing (saveAndSubmit, saveDraft, submitWorkflow)

{-| New ページの API 呼び出し

ワークフローの下書き保存・申請に関する Cmd 生成を担当する。

-}

import Api.Workflow as WorkflowApi
import Dict exposing (Dict)
import Json.Encode as Encode
import Page.Workflow.New.Types exposing (Msg(..))
import Shared exposing (Shared)


{-| 下書き保存 API を呼び出す
-}
saveDraft : Shared -> String -> String -> Dict String String -> Cmd Msg
saveDraft shared definitionId title formValues =
    WorkflowApi.createWorkflow
        { config = Shared.toRequestConfig shared
        , body =
            { definitionId = definitionId
            , title = title
            , formData = encodeFormValues formValues
            }
        , toMsg = GotSaveResult
        }


{-| ワークフローを申請
-}
submitWorkflow : Shared -> Int -> List WorkflowApi.StepApproverRequest -> Cmd Msg
submitWorkflow shared workflowDisplayNumber approvers =
    WorkflowApi.submitWorkflow
        { config = Shared.toRequestConfig shared
        , displayNumber = workflowDisplayNumber
        , body = { approvers = approvers }
        , toMsg = GotSubmitResult
        }


{-| 保存と申請を連続実行

未保存のワークフローを下書き保存する。
保存成功時は GotSaveAndSubmitResult ハンドラで submitWorkflow にチェーンし、
保存→申請の連続処理を実現する。

-}
saveAndSubmit : Shared -> String -> String -> Dict String String -> List WorkflowApi.StepApproverRequest -> Cmd Msg
saveAndSubmit shared definitionId title formValues approvers =
    WorkflowApi.createWorkflow
        { config = Shared.toRequestConfig shared
        , body =
            { definitionId = definitionId
            , title = title
            , formData = encodeFormValues formValues
            }
        , toMsg = GotSaveAndSubmitResult approvers
        }


{-| フォーム値を JSON にエンコード
-}
encodeFormValues : Dict String String -> Encode.Value
encodeFormValues values =
    Dict.toList values
        |> List.map (\( k, v ) -> ( k, Encode.string v ))
        |> Encode.object
