module Data.WorkflowInstance exposing
    ( Decision(..)
    , Status(..)
    , StepStatus(..)
    , WorkflowInstance
    , WorkflowInstanceId
    , WorkflowStep
    , decisionToJapanese
    , decoder
    , listDecoder
    , statusFromString
    , statusToCssClass
    , statusToJapanese
    , statusToString
    , stepDecoder
    , stepStatusDecoder
    , stepStatusToCssClass
    , stepStatusToJapanese
    )

{-| ワークフローインスタンスのデータ型

バックエンドの `WorkflowInstance` に対応する型とデコーダーを提供する。


## 用途

  - 申請一覧の表示
  - 申請詳細の表示
  - 申請の作成・更新レスポンスの処理

-}

import Data.UserRef exposing (UserRef)
import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)
import Json.Encode as Encode



-- TYPES


{-| ワークフローインスタンス ID

UUID 文字列をラップした型。

-}
type alias WorkflowInstanceId =
    String


{-| ワークフローステップ

承認フローの各ステップを表す。

-}
type alias WorkflowStep =
    { id : String
    , displayId : String
    , stepName : String
    , status : StepStatus
    , decision : Maybe Decision
    , assignedTo : Maybe UserRef
    , comment : Maybe String
    , version : Int
    }


{-| ステップのステータス
-}
type StepStatus
    = StepPending
    | StepActive
    | StepCompleted
    | StepSkipped


{-| 承認/却下の判定結果
-}
type Decision
    = DecisionApproved
    | DecisionRejected


{-| ワークフローのステータス

ワークフローのライフサイクルを表すカスタム型。
バックエンドの enum に対応。

-}
type Status
    = Draft
    | Pending
    | InProgress
    | Approved
    | Rejected
    | Cancelled


{-| ワークフローインスタンス

申請されたワークフローの状態を表す。

-}
type alias WorkflowInstance =
    { id : WorkflowInstanceId
    , displayId : String
    , title : String
    , definitionId : String
    , status : Status
    , version : Int
    , formData : Encode.Value
    , initiatedBy : UserRef
    , currentStepId : Maybe String
    , steps : List WorkflowStep
    , submittedAt : Maybe String
    , createdAt : String
    , updatedAt : String
    }



-- STATUS HELPERS


{-| ステータスを文字列に変換
-}
statusToString : Status -> String
statusToString status =
    case status of
        Draft ->
            "Draft"

        Pending ->
            "Pending"

        InProgress ->
            "InProgress"

        Approved ->
            "Approved"

        Rejected ->
            "Rejected"

        Cancelled ->
            "Cancelled"


{-| 文字列からステータスに変換
-}
statusFromString : String -> Maybe Status
statusFromString str =
    case str of
        "Draft" ->
            Just Draft

        "Pending" ->
            Just Pending

        "InProgress" ->
            Just InProgress

        "Approved" ->
            Just Approved

        "Rejected" ->
            Just Rejected

        "Cancelled" ->
            Just Cancelled

        _ ->
            Nothing


{-| ステータスを日本語に変換（UI 表示用）
-}
statusToJapanese : Status -> String
statusToJapanese status =
    case status of
        Draft ->
            "下書き"

        Pending ->
            "申請待ち"

        InProgress ->
            "承認中"

        Approved ->
            "承認済み"

        Rejected ->
            "却下"

        Cancelled ->
            "キャンセル"


{-| ステータスを Tailwind CSS クラスに変換（バッジスタイリング用）
-}
statusToCssClass : Status -> String
statusToCssClass status =
    case status of
        Draft ->
            "bg-gray-100 text-gray-600"

        Pending ->
            "bg-warning-50 text-warning-600"

        InProgress ->
            "bg-info-50 text-info-600"

        Approved ->
            "bg-success-50 text-success-600"

        Rejected ->
            "bg-error-50 text-error-600"

        Cancelled ->
            "bg-secondary-100 text-secondary-500"


{-| ステップステータスを Tailwind CSS クラスに変換（バッジスタイリング用）
-}
stepStatusToCssClass : StepStatus -> String
stepStatusToCssClass status =
    case status of
        StepPending ->
            "bg-gray-100 text-gray-600"

        StepActive ->
            "bg-warning-50 text-warning-600"

        StepCompleted ->
            "bg-success-50 text-success-600"

        StepSkipped ->
            "bg-secondary-100 text-secondary-500"


{-| ステップステータスを日本語に変換
-}
stepStatusToJapanese : StepStatus -> String
stepStatusToJapanese status =
    case status of
        StepPending ->
            "待機中"

        StepActive ->
            "承認待ち"

        StepCompleted ->
            "完了"

        StepSkipped ->
            "スキップ"


{-| 判定結果を日本語に変換
-}
decisionToJapanese : Decision -> String
decisionToJapanese decision =
    case decision of
        DecisionApproved ->
            "承認"

        DecisionRejected ->
            "却下"



-- DECODERS


{-| ステータスをデコード
-}
statusDecoder : Decoder Status
statusDecoder =
    Decode.string
        |> Decode.andThen
            (\str ->
                case statusFromString str of
                    Just status ->
                        Decode.succeed status

                    Nothing ->
                        Decode.fail ("Unknown status: " ++ str)
            )


{-| ステップステータスをデコード
-}
stepStatusDecoder : Decoder StepStatus
stepStatusDecoder =
    Decode.string
        |> Decode.andThen
            (\str ->
                case str of
                    "Pending" ->
                        Decode.succeed StepPending

                    "Active" ->
                        Decode.succeed StepActive

                    "Completed" ->
                        Decode.succeed StepCompleted

                    "Skipped" ->
                        Decode.succeed StepSkipped

                    _ ->
                        Decode.fail ("Unknown step status: " ++ str)
            )


{-| 判定結果をデコード
-}
decisionDecoder : Decoder Decision
decisionDecoder =
    Decode.string
        |> Decode.andThen
            (\str ->
                case str of
                    "Approved" ->
                        Decode.succeed DecisionApproved

                    "Rejected" ->
                        Decode.succeed DecisionRejected

                    _ ->
                        Decode.fail ("Unknown decision: " ++ str)
            )


{-| ワークフローステップをデコード
-}
stepDecoder : Decoder WorkflowStep
stepDecoder =
    Decode.succeed WorkflowStep
        |> required "id" Decode.string
        |> required "display_id" Decode.string
        |> required "step_name" Decode.string
        |> required "status" stepStatusDecoder
        |> optional "decision" (Decode.nullable decisionDecoder) Nothing
        |> optional "assigned_to" (Decode.nullable Data.UserRef.decoder) Nothing
        |> optional "comment" (Decode.nullable Decode.string) Nothing
        |> optional "version" Decode.int 1


{-| 単一のワークフローインスタンスをデコード
-}
decoder : Decoder WorkflowInstance
decoder =
    Decode.succeed WorkflowInstance
        |> required "id" Decode.string
        |> required "display_id" Decode.string
        |> required "title" Decode.string
        |> required "definition_id" Decode.string
        |> required "status" statusDecoder
        |> optional "version" Decode.int 1
        |> required "form_data" Decode.value
        |> required "initiated_by" Data.UserRef.decoder
        |> optional "current_step_id" (Decode.nullable Decode.string) Nothing
        |> optional "steps" (Decode.list stepDecoder) []
        |> optional "submitted_at" (Decode.nullable Decode.string) Nothing
        |> required "created_at" Decode.string
        |> required "updated_at" Decode.string


{-| ワークフローインスタンス一覧をデコード

API レスポンスの `{ data: [...] }` 形式に対応。

-}
listDecoder : Decoder (List WorkflowInstance)
listDecoder =
    Decode.field "data" (Decode.list decoder)
