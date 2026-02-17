module Data.AuditLog exposing
    ( AuditLogItem
    , AuditLogList
    , actionToJapanese
    , auditLogListDecoder
    , resultToCssClass
    , resultToJapanese
    )

{-| 監査ログ用のデータ型

監査ログ一覧画面で使用する型とデコーダーを提供する。
カーソルベースのページネーションに対応。

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)



-- TYPES


{-| 監査ログの要素
-}
type alias AuditLogItem =
    { id : String
    , actorId : String
    , actorName : String
    , action : String
    , result : String
    , resourceType : String
    , resourceId : String
    , detail : Maybe Decode.Value
    , sourceIp : Maybe String
    , createdAt : String
    }


{-| 監査ログ一覧（カーソルページネーション付き）
-}
type alias AuditLogList =
    { data : List AuditLogItem
    , nextCursor : Maybe String
    }



-- DECODERS


{-| AuditLogItem のデコーダー
-}
auditLogItemDecoder : Decoder AuditLogItem
auditLogItemDecoder =
    Decode.succeed AuditLogItem
        |> required "id" Decode.string
        |> required "actor_id" Decode.string
        |> required "actor_name" Decode.string
        |> required "action" Decode.string
        |> required "result" Decode.string
        |> required "resource_type" Decode.string
        |> required "resource_id" Decode.string
        |> optional "detail" (Decode.nullable Decode.value) Nothing
        |> optional "source_ip" (Decode.nullable Decode.string) Nothing
        |> required "created_at" Decode.string


{-| AuditLogList のデコーダー

`{ "data": [...], "next_cursor": "..." | null }` 形式に対応。

-}
auditLogListDecoder : Decoder AuditLogList
auditLogListDecoder =
    Decode.succeed AuditLogList
        |> required "data" (Decode.list auditLogItemDecoder)
        |> optional "next_cursor" (Decode.nullable Decode.string) Nothing



-- HELPERS


{-| アクション名を日本語に変換
-}
actionToJapanese : String -> String
actionToJapanese action =
    case action of
        "user.create" ->
            "ユーザー作成"

        "user.update" ->
            "ユーザー更新"

        "user.deactivate" ->
            "ユーザー無効化"

        "user.activate" ->
            "ユーザー有効化"

        "role.create" ->
            "ロール作成"

        "role.update" ->
            "ロール更新"

        "role.delete" ->
            "ロール削除"

        _ ->
            action


{-| 結果を日本語に変換
-}
resultToJapanese : String -> String
resultToJapanese result =
    case result of
        "success" ->
            "成功"

        "failure" ->
            "失敗"

        _ ->
            result


{-| 結果に対応する CSS クラス
-}
resultToCssClass : String -> String
resultToCssClass result =
    case result of
        "success" ->
            "bg-success-100 text-success-800 border-success-200"

        "failure" ->
            "bg-error-100 text-error-800 border-error-200"

        _ ->
            "bg-secondary-100 text-secondary-800 border-secondary-200"
