module Data.Role exposing
    ( RoleDetail
    , RoleItem
    , roleDetailDecoder
    , roleItemDecoder
    , roleItemListDecoder
    )

{-| ロール管理用のデータ型

ロール一覧・詳細画面で使用する型とデコーダーを提供する。

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)



-- TYPES


{-| ロール一覧の要素
-}
type alias RoleItem =
    { id : String
    , name : String
    , description : Maybe String
    , permissions : List String
    , isSystem : Bool
    , userCount : Int
    }


{-| ロール詳細
-}
type alias RoleDetail =
    { id : String
    , name : String
    , description : Maybe String
    , permissions : List String
    , isSystem : Bool
    , createdAt : String
    , updatedAt : String
    }



-- DECODERS


{-| RoleItem のデコーダー
-}
roleItemDecoder : Decoder RoleItem
roleItemDecoder =
    Decode.succeed RoleItem
        |> required "id" Decode.string
        |> required "name" Decode.string
        |> optional "description" (Decode.nullable Decode.string) Nothing
        |> required "permissions" (Decode.list Decode.string)
        |> required "is_system" Decode.bool
        |> required "user_count" Decode.int


{-| RoleItem 一覧のデコーダー

`{ "data": [...] }` 形式に対応。

-}
roleItemListDecoder : Decoder (List RoleItem)
roleItemListDecoder =
    Decode.field "data" (Decode.list roleItemDecoder)


{-| RoleDetail のデコーダー

`{ "data": {...} }` 形式に対応。

-}
roleDetailDecoder : Decoder RoleDetail
roleDetailDecoder =
    Decode.field "data" roleDetailInnerDecoder


roleDetailInnerDecoder : Decoder RoleDetail
roleDetailInnerDecoder =
    Decode.succeed RoleDetail
        |> required "id" Decode.string
        |> required "name" Decode.string
        |> optional "description" (Decode.nullable Decode.string) Nothing
        |> required "permissions" (Decode.list Decode.string)
        |> required "is_system" Decode.bool
        |> required "created_at" Decode.string
        |> required "updated_at" Decode.string
