module Data.WorkflowInstance exposing
    ( Status(..)
    , WorkflowInstance
    , WorkflowInstanceId
    , decoder
    , listDecoder
    , statusFromString
    , statusToCssClass
    , statusToJapanese
    , statusToString
    )

{-| ワークフローインスタンスのデータ型

バックエンドの `WorkflowInstance` に対応する型とデコーダーを提供する。


## 用途

  - 申請一覧の表示
  - 申請詳細の表示
  - 申請の作成・更新レスポンスの処理

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)
import Json.Encode as Encode



-- TYPES


{-| ワークフローインスタンス ID

UUID 文字列をラップした型。

-}
type alias WorkflowInstanceId =
    String


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
    , title : String
    , definitionId : String
    , status : Status
    , formData : Encode.Value
    , initiatedBy : String
    , currentStepId : Maybe String
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


{-| ステータスを CSS クラス名に変換（スタイリング用）
-}
statusToCssClass : Status -> String
statusToCssClass status =
    case status of
        Draft ->
            "status-draft"

        Pending ->
            "status-pending"

        InProgress ->
            "status-in-progress"

        Approved ->
            "status-approved"

        Rejected ->
            "status-rejected"

        Cancelled ->
            "status-cancelled"



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


{-| 単一のワークフローインスタンスをデコード
-}
decoder : Decoder WorkflowInstance
decoder =
    Decode.succeed WorkflowInstance
        |> required "id" Decode.string
        |> required "title" Decode.string
        |> required "definition_id" Decode.string
        |> required "status" statusDecoder
        |> required "form_data" Decode.value
        |> required "initiated_by" Decode.string
        |> optional "current_step_id" (Decode.nullable Decode.string) Nothing
        |> optional "submitted_at" (Decode.nullable Decode.string) Nothing
        |> required "created_at" Decode.string
        |> required "updated_at" Decode.string


{-| ワークフローインスタンス一覧をデコード

API レスポンスの `{ data: [...] }` 形式に対応。

-}
listDecoder : Decoder (List WorkflowInstance)
listDecoder =
    Decode.field "data" (Decode.list decoder)
