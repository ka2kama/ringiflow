module Data.UserRef exposing
    ( UserRef
    , decoder
    )

{-| ユーザー参照型

UUID の代わりに、ID とユーザー名をペアで扱う。
バックエンドの `UserRefDto` に対応。


## 用途

  - ワークフローの申請者（`initiatedBy`）
  - ステップの担当者（`assignedTo`）

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (required)



-- TYPES


{-| ユーザー参照

ID とユーザー名のペア。

-}
type alias UserRef =
    { id : String
    , name : String
    }



-- DECODERS


{-| ユーザー参照をデコード
-}
decoder : Decoder UserRef
decoder =
    Decode.succeed UserRef
        |> required "id" Decode.string
        |> required "name" Decode.string
