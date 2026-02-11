module Data.AdminUser exposing
    ( AdminUserItem
    , CreateUserResponse
    , UserDetail
    , UserResponse
    , adminUserItemDecoder
    , adminUserItemListDecoder
    , createUserResponseDecoder
    , userDetailDecoder
    , userResponseDecoder
    )

{-| ユーザー管理用のデータ型

管理画面でのユーザー CRUD に使用する型とデコーダーを提供する。
承認者選択用の `Data.UserItem` とは別に、ステータスやロール情報を含む。

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (required)



-- TYPES


{-| ユーザー管理一覧の要素

ステータスとロール情報を含む管理者向けの型。

-}
type alias AdminUserItem =
    { id : String
    , displayId : String
    , displayNumber : Int
    , name : String
    , email : String
    , status : String
    , roles : List String
    }


{-| ユーザー詳細

権限やテナント名を含む完全な情報。

-}
type alias UserDetail =
    { id : String
    , displayId : String
    , displayNumber : Int
    , name : String
    , email : String
    , status : String
    , roles : List String
    , permissions : List String
    , tenantName : String
    }


{-| ユーザー作成レスポンス

初期パスワードを含む。

-}
type alias CreateUserResponse =
    { id : String
    , displayId : String
    , displayNumber : Int
    , name : String
    , email : String
    , role : String
    , initialPassword : String
    }


{-| ユーザー簡易レスポンス（更新・ステータス変更用）
-}
type alias UserResponse =
    { id : String
    , name : String
    , email : String
    , status : String
    }



-- DECODERS


{-| AdminUserItem のデコーダー
-}
adminUserItemDecoder : Decoder AdminUserItem
adminUserItemDecoder =
    Decode.succeed AdminUserItem
        |> required "id" Decode.string
        |> required "display_id" Decode.string
        |> required "display_number" Decode.int
        |> required "name" Decode.string
        |> required "email" Decode.string
        |> required "status" Decode.string
        |> required "roles" (Decode.list Decode.string)


{-| AdminUserItem 一覧のデコーダー

`{ "data": [...] }` 形式に対応。

-}
adminUserItemListDecoder : Decoder (List AdminUserItem)
adminUserItemListDecoder =
    Decode.field "data" (Decode.list adminUserItemDecoder)


{-| UserDetail のデコーダー

`{ "data": {...} }` 形式に対応。

-}
userDetailDecoder : Decoder UserDetail
userDetailDecoder =
    Decode.field "data" userDetailInnerDecoder


userDetailInnerDecoder : Decoder UserDetail
userDetailInnerDecoder =
    Decode.succeed UserDetail
        |> required "id" Decode.string
        |> required "display_id" Decode.string
        |> required "display_number" Decode.int
        |> required "name" Decode.string
        |> required "email" Decode.string
        |> required "status" Decode.string
        |> required "roles" (Decode.list Decode.string)
        |> required "permissions" (Decode.list Decode.string)
        |> required "tenant_name" Decode.string


{-| CreateUserResponse のデコーダー

`{ "data": {...} }` 形式に対応。

-}
createUserResponseDecoder : Decoder CreateUserResponse
createUserResponseDecoder =
    Decode.field "data" createUserResponseInnerDecoder


createUserResponseInnerDecoder : Decoder CreateUserResponse
createUserResponseInnerDecoder =
    Decode.succeed CreateUserResponse
        |> required "id" Decode.string
        |> required "display_id" Decode.string
        |> required "display_number" Decode.int
        |> required "name" Decode.string
        |> required "email" Decode.string
        |> required "role" Decode.string
        |> required "initial_password" Decode.string


{-| UserResponse のデコーダー

`{ "data": {...} }` 形式に対応。

-}
userResponseDecoder : Decoder UserResponse
userResponseDecoder =
    Decode.field "data" userResponseInnerDecoder


userResponseInnerDecoder : Decoder UserResponse
userResponseInnerDecoder =
    Decode.succeed UserResponse
        |> required "id" Decode.string
        |> required "name" Decode.string
        |> required "email" Decode.string
        |> required "status" Decode.string
