module Data.WorkflowInstance exposing
    ( Decision(..)
    , Status(..)
    , StepStatus(..)
    , WorkflowInstance
    , WorkflowInstanceId
    , WorkflowStep
    , decisionFromString
    , decisionToJapanese
    , decisionToString
    , decoder
    , detailDecoder
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
    , displayNumber : Int
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


{-| 承認/却下/差し戻しの判定結果
-}
type Decision
    = DecisionApproved
    | DecisionRejected
    | DecisionRequestChanges


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
    | ChangesRequested


{-| ワークフローインスタンス

申請されたワークフローの状態を表す。

-}
type alias WorkflowInstance =
    { id : WorkflowInstanceId
    , displayId : String
    , displayNumber : Int
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

        ChangesRequested ->
            "ChangesRequested"


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

        "ChangesRequested" ->
            Just ChangesRequested

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

        ChangesRequested ->
            "差し戻し"


{-| ステータスを Tailwind CSS クラスに変換（バッジスタイリング用）
-}
statusToCssClass : Status -> String
statusToCssClass status =
    case status of
        Draft ->
            "bg-secondary-100 text-secondary-600 border-secondary-200"

        Pending ->
            "bg-warning-50 text-warning-600 border-warning-200"

        InProgress ->
            "bg-info-50 text-info-600 border-info-300"

        Approved ->
            "bg-success-50 text-success-600 border-success-200"

        Rejected ->
            "bg-error-50 text-error-600 border-error-200"

        Cancelled ->
            "bg-secondary-100 text-secondary-500 border-secondary-200"

        ChangesRequested ->
            "bg-warning-50 text-warning-600 border-warning-200"


{-| ステップステータスを Tailwind CSS クラスに変換（バッジスタイリング用）
-}
stepStatusToCssClass : StepStatus -> String
stepStatusToCssClass status =
    case status of
        StepPending ->
            "bg-secondary-100 text-secondary-600 border-secondary-200"

        StepActive ->
            "bg-warning-50 text-warning-600 border-warning-200"

        StepCompleted ->
            "bg-success-50 text-success-600 border-success-200"

        StepSkipped ->
            "bg-secondary-100 text-secondary-500 border-secondary-200"


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


{-| 判定結果を文字列に変換
-}
decisionToString : Decision -> String
decisionToString decision =
    case decision of
        DecisionApproved ->
            "Approved"

        DecisionRejected ->
            "Rejected"

        DecisionRequestChanges ->
            "RequestChanges"


{-| 文字列から判定結果に変換
-}
decisionFromString : String -> Maybe Decision
decisionFromString str =
    case str of
        "Approved" ->
            Just DecisionApproved

        "Rejected" ->
            Just DecisionRejected

        "RequestChanges" ->
            Just DecisionRequestChanges

        _ ->
            Nothing


{-| 判定結果を日本語に変換
-}
decisionToJapanese : Decision -> String
decisionToJapanese decision =
    case decision of
        DecisionApproved ->
            "承認"

        DecisionRejected ->
            "却下"

        DecisionRequestChanges ->
            "差し戻し"



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
                case decisionFromString str of
                    Just decision ->
                        Decode.succeed decision

                    Nothing ->
                        Decode.fail ("Unknown decision: " ++ str)
            )


{-| ワークフローステップをデコード
-}
stepDecoder : Decoder WorkflowStep
stepDecoder =
    Decode.succeed WorkflowStep
        |> required "id" Decode.string
        |> required "display_id" Decode.string
        |> required "display_number" Decode.int
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
        |> required "display_number" Decode.int
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


{-| 単一のワークフローインスタンスレスポンスをデコード

API レスポンスの `{ data: {...} }` 形式に対応。

-}
detailDecoder : Decoder WorkflowInstance
detailDecoder =
    Decode.field "data" decoder


{-| ワークフローインスタンス一覧をデコード

API レスポンスの `{ data: [...] }` 形式に対応。

-}
listDecoder : Decoder (List WorkflowInstance)
listDecoder =
    Decode.field "data" (Decode.list decoder)
