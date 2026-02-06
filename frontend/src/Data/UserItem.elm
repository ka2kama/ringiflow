module Data.UserItem exposing
    ( UserItem
    , decoder
    , filterUsers
    , listDecoder
    )

{-| ユーザー一覧のデータ型

バックエンドの `UserItem` に対応する型とデコーダーを提供する。
承認者選択 UI でのユーザー検索・表示に使用。

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (required)



-- TYPES


{-| ユーザー一覧の要素

同姓同名ユーザーの区別のため displayId と email を保持する。
id は内部でのみ使用（UUID）。

-}
type alias UserItem =
    { id : String
    , displayId : String
    , displayNumber : Int
    , name : String
    , email : String
    }



-- DECODERS


{-| 単一のユーザーをデコード
-}
decoder : Decoder UserItem
decoder =
    Decode.succeed UserItem
        |> required "id" Decode.string
        |> required "display_id" Decode.string
        |> required "display_number" Decode.int
        |> required "name" Decode.string
        |> required "email" Decode.string


{-| ユーザー一覧をデコード

API レスポンスの `{ data: [...] }` 形式に対応。

-}
listDecoder : Decoder (List UserItem)
listDecoder =
    Decode.field "data" (Decode.list decoder)



-- FILTERING


{-| 検索クエリでユーザーをフィルタリング

名前・表示用 ID・メールアドレスの部分一致で検索する。
大文字小文字を区別しない。空クエリは空リストを返す。

-}
filterUsers : String -> List UserItem -> List UserItem
filterUsers query users =
    let
        normalizedQuery =
            String.toLower (String.trim query)
    in
    if String.isEmpty normalizedQuery then
        []

    else
        List.filter
            (\user ->
                String.contains normalizedQuery (String.toLower user.name)
                    || String.contains normalizedQuery (String.toLower user.displayId)
                    || String.contains normalizedQuery (String.toLower user.email)
            )
            users
