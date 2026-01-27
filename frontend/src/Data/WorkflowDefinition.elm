module Data.WorkflowDefinition exposing
    ( WorkflowDefinition
    , WorkflowDefinitionId
    , decoder
    , listDecoder
    )

{-| ワークフロー定義のデータ型

バックエンドの `WorkflowDefinition` に対応する型とデコーダーを提供する。


## 用途

  - 申請フォームでユーザーが選択可能なワークフロー定義の表示
  - 選択されたワークフロー定義に基づく動的フォーム生成

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)



-- TYPES


{-| ワークフロー定義 ID

UUID 文字列をラップした型。型安全性のため String ではなく専用型を使用。

-}
type alias WorkflowDefinitionId =
    String


{-| ワークフロー定義

バックエンドから取得するワークフロー定義のデータ構造。
`definition` フィールドにフォーム定義（フィールド一覧など）が含まれる。

-}
type alias WorkflowDefinition =
    { id : WorkflowDefinitionId
    , name : String
    , description : Maybe String
    , version : Int
    , definition : Decode.Value -- 動的な JSON 構造
    , status : String
    , createdBy : String
    , createdAt : String
    , updatedAt : String
    }



-- DECODERS


{-| 単一のワークフロー定義をデコード
-}
decoder : Decoder WorkflowDefinition
decoder =
    Decode.succeed WorkflowDefinition
        |> required "id" Decode.string
        |> required "name" Decode.string
        |> optional "description" (Decode.nullable Decode.string) Nothing
        |> required "version" Decode.int
        |> required "definition" Decode.value
        |> required "status" Decode.string
        |> required "created_by" Decode.string
        |> required "created_at" Decode.string
        |> required "updated_at" Decode.string


{-| ワークフロー定義一覧をデコード

API レスポンスの `{ data: [...] }` 形式に対応。

-}
listDecoder : Decoder (List WorkflowDefinition)
listDecoder =
    Decode.field "data" (Decode.list decoder)
