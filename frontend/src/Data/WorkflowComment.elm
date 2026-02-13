module Data.WorkflowComment exposing
    ( WorkflowComment
    , decoder
    , listDecoder
    )

{-| ワークフローコメントのデータ型

バックエンドの `WorkflowCommentData` に対応する型とデコーダーを提供する。


## 用途

  - 申請詳細画面でのコメントスレッド表示
  - コメント投稿後のレスポンス処理

-}

import Data.UserRef exposing (UserRef)
import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (required)



-- TYPES


{-| ワークフローコメント

申請に対するコメント。投稿者情報を `UserRef` で保持する。

-}
type alias WorkflowComment =
    { id : String
    , postedBy : UserRef
    , body : String
    , createdAt : String
    }



-- DECODERS


{-| 単一のコメントをデコード
-}
decoder : Decoder WorkflowComment
decoder =
    Decode.succeed WorkflowComment
        |> required "id" Decode.string
        |> required "posted_by" Data.UserRef.decoder
        |> required "body" Decode.string
        |> required "created_at" Decode.string


{-| コメント一覧をデコード

API レスポンスの `{ data: [...] }` 形式に対応。

-}
listDecoder : Decoder (List WorkflowComment)
listDecoder =
    Decode.field "data" (Decode.list decoder)
