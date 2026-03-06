module Data.Folder exposing
    ( Folder
    , decoder
    , detailDecoder
    , listDecoder
    )

{-| フォルダ管理のデータ型

フォルダ API のレスポンスデータ型とデコーダーを提供する。

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)



-- TYPES


{-| フォルダ
-}
type alias Folder =
    { id : String
    , name : String
    , parentId : Maybe String
    , path : String
    , depth : Int
    , createdAt : String
    , updatedAt : String
    }



-- DECODERS


{-| 単一フォルダをデコード
-}
decoder : Decoder Folder
decoder =
    Decode.succeed Folder
        |> required "id" Decode.string
        |> required "name" Decode.string
        |> optional "parent_id" (Decode.nullable Decode.string) Nothing
        |> required "path" Decode.string
        |> required "depth" Decode.int
        |> required "created_at" Decode.string
        |> required "updated_at" Decode.string


{-| data フィールドから単一フォルダをデコード
-}
detailDecoder : Decoder Folder
detailDecoder =
    Decode.field "data" decoder


{-| data フィールドからフォルダ一覧をデコード
-}
listDecoder : Decoder (List Folder)
listDecoder =
    Decode.field "data" (Decode.list decoder)
