module Data.Task exposing
    ( TaskDetail
    , TaskItem
    , WorkflowSummary
    , detailDecoder
    , listDecoder
    )

{-| タスクデータ型

「タスク」はワークフローステップのユーザー向けビュー。
自分にアサインされたアクティブな承認ステップを表す。


## 型の構成

  - `TaskItem`: タスク一覧用（軽量なワークフロー概要を含む）
  - `TaskDetail`: タスク詳細用（完全な WorkflowStep + WorkflowInstance）
  - `WorkflowSummary`: タスク一覧に表示するワークフロー概要

-}

import Data.UserRef exposing (UserRef)
import Data.WorkflowInstance as WorkflowInstance
    exposing
        ( StepStatus
        , WorkflowInstance
        , WorkflowStep
        )
import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)



-- TYPES


{-| タスク一覧に表示するワークフロー概要
-}
type alias WorkflowSummary =
    { id : String
    , title : String
    , status : String
    , initiatedBy : UserRef
    , submittedAt : Maybe String
    }


{-| タスク一覧の要素

承認ステップの基本情報と、関連するワークフローの概要を含む。

-}
type alias TaskItem =
    { id : String
    , stepName : String
    , status : StepStatus
    , version : Int
    , assignedTo : Maybe UserRef
    , dueDate : Maybe String
    , startedAt : Maybe String
    , createdAt : String
    , workflow : WorkflowSummary
    }


{-| タスク詳細

承認ステップの完全な情報と、関連するワークフローインスタンス全体を含む。
既存の `WorkflowStep` と `WorkflowInstance` を再利用する。

-}
type alias TaskDetail =
    { step : WorkflowStep
    , workflow : WorkflowInstance
    }



-- DECODERS


{-| ワークフロー概要をデコード
-}
workflowSummaryDecoder : Decoder WorkflowSummary
workflowSummaryDecoder =
    Decode.succeed WorkflowSummary
        |> required "id" Decode.string
        |> required "title" Decode.string
        |> required "status" Decode.string
        |> required "initiated_by" Data.UserRef.decoder
        |> optional "submitted_at" (Decode.nullable Decode.string) Nothing


{-| タスク一覧の要素をデコード
-}
taskItemDecoder : Decoder TaskItem
taskItemDecoder =
    Decode.succeed TaskItem
        |> required "id" Decode.string
        |> required "step_name" Decode.string
        |> required "status" WorkflowInstance.stepStatusDecoder
        |> optional "version" Decode.int 1
        |> optional "assigned_to" (Decode.nullable Data.UserRef.decoder) Nothing
        |> optional "due_date" (Decode.nullable Decode.string) Nothing
        |> optional "started_at" (Decode.nullable Decode.string) Nothing
        |> required "created_at" Decode.string
        |> required "workflow" workflowSummaryDecoder


{-| タスク一覧をデコード

API レスポンスの `{ data: [...] }` 形式に対応。

-}
listDecoder : Decoder (List TaskItem)
listDecoder =
    Decode.field "data" (Decode.list taskItemDecoder)


{-| タスク詳細をデコード

API レスポンスの `{ data: { step: ..., workflow: ... } }` 形式に対応。

-}
detailDecoder : Decoder TaskDetail
detailDecoder =
    Decode.field "data"
        (Decode.succeed TaskDetail
            |> required "step" WorkflowInstance.stepDecoder
            |> required "workflow" WorkflowInstance.decoder
        )
