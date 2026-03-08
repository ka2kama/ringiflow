module Data.Document exposing
    ( Document
    , DownloadUrlResponse
    , UploadUrlResponse
    , decoder
    , detailDecoder
    , downloadUrlResponseDecoder
    , listDecoder
    , uploadUrlResponseDecoder
    )

{-| ドキュメント管理のデータ型

ファイルアップロード・ダウンロード API のレスポンスデータ型とデコーダーを提供する。

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (required)



-- TYPES


{-| ドキュメント
-}
type alias Document =
    { id : String
    , filename : String
    , contentType : String
    , size : Int
    , status : String
    , createdAt : String
    }


{-| アップロード URL 発行レスポンス
-}
type alias UploadUrlResponse =
    { documentId : String
    , uploadUrl : String
    , expiresIn : Int
    }


{-| ダウンロード URL 発行レスポンス
-}
type alias DownloadUrlResponse =
    { downloadUrl : String
    , expiresIn : Int
    }



-- DECODERS


{-| 単一ドキュメントをデコード
-}
decoder : Decoder Document
decoder =
    Decode.succeed Document
        |> required "id" Decode.string
        |> required "filename" Decode.string
        |> required "content_type" Decode.string
        |> required "size" Decode.int
        |> required "status" Decode.string
        |> required "created_at" Decode.string


{-| 単一ドキュメントをデコード
-}
detailDecoder : Decoder Document
detailDecoder =
    decoder


{-| アップロード URL レスポンスをデコード
-}
uploadUrlResponseDecoder : Decoder UploadUrlResponse
uploadUrlResponseDecoder =
    Decode.succeed UploadUrlResponse
        |> required "document_id" Decode.string
        |> required "upload_url" Decode.string
        |> required "expires_in" Decode.int


{-| ダウンロード URL レスポンスをデコード
-}
downloadUrlResponseDecoder : Decoder DownloadUrlResponse
downloadUrlResponseDecoder =
    Decode.succeed DownloadUrlResponse
        |> required "download_url" Decode.string
        |> required "expires_in" Decode.int


{-| ドキュメント一覧をデコード
-}
listDecoder : Decoder (List Document)
listDecoder =
    Decode.list decoder
