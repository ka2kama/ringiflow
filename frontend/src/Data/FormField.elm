module Data.FormField exposing
    ( FieldType(..)
    , FormField
    , Validation
    , decoder
    , listDecoder
    )

{-| 動的フォームフィールドのデータ型

ワークフロー定義の `definition.form.fields` から取得する
フォームフィールドの型とデコーダーを提供する。


## 用途

  - 動的フォームの生成
  - フィールドタイプに応じた入力コンポーネントの選択
  - バリデーションルールの適用


## フィールドタイプ

MVP では以下のタイプをサポート:

  - `Text`: 単一行テキスト入力
  - `Number`: 数値入力
  - `Select`: ドロップダウン選択
  - `Date`: 日付選択
  - `File`: ファイルアップロード

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)



-- TYPES


{-| フォームフィールドの型
-}
type FieldType
    = Text
    | Number
    | Select (List SelectOption)
    | Date
    | File


{-| Select フィールドの選択肢
-}
type alias SelectOption =
    { value : String
    , label : String
    }


{-| バリデーションルール
-}
type alias Validation =
    { required : Bool
    , minLength : Maybe Int
    , maxLength : Maybe Int
    , min : Maybe Float
    , max : Maybe Float
    }


{-| フォームフィールド
-}
type alias FormField =
    { id : String
    , label : String
    , fieldType : FieldType
    , placeholder : Maybe String
    , validation : Validation
    }



-- DECODERS


{-| 選択肢をデコード
-}
selectOptionDecoder : Decoder SelectOption
selectOptionDecoder =
    Decode.succeed SelectOption
        |> required "value" Decode.string
        |> required "label" Decode.string


{-| フィールドタイプをデコード
-}
fieldTypeDecoder : Decoder FieldType
fieldTypeDecoder =
    Decode.field "type" Decode.string
        |> Decode.andThen
            (\typeStr ->
                case typeStr of
                    "text" ->
                        Decode.succeed Text

                    "number" ->
                        Decode.succeed Number

                    "select" ->
                        Decode.field "options" (Decode.list selectOptionDecoder)
                            |> Decode.map Select

                    "date" ->
                        Decode.succeed Date

                    "file" ->
                        Decode.succeed File

                    _ ->
                        -- 未知のタイプはテキストとして扱う
                        Decode.succeed Text
            )


{-| バリデーションルールをデコード
-}
validationDecoder : Decoder Validation
validationDecoder =
    Decode.succeed Validation
        |> optional "required" Decode.bool False
        |> optional "minLength" (Decode.nullable Decode.int) Nothing
        |> optional "maxLength" (Decode.nullable Decode.int) Nothing
        |> optional "min" (Decode.nullable Decode.float) Nothing
        |> optional "max" (Decode.nullable Decode.float) Nothing


{-| 単一のフォームフィールドをデコード
-}
decoder : Decoder FormField
decoder =
    Decode.succeed FormField
        |> required "id" Decode.string
        |> required "label" Decode.string
        |> Decode.andMap fieldTypeDecoder
        |> optional "placeholder" (Decode.nullable Decode.string) Nothing
        |> optional "validation" validationDecoder defaultValidation


{-| デフォルトのバリデーション（全て無効）
-}
defaultValidation : Validation
defaultValidation =
    { required = False
    , minLength = Nothing
    , maxLength = Nothing
    , min = Nothing
    , max = Nothing
    }


{-| フォームフィールド一覧をデコード
-}
listDecoder : Decoder (List FormField)
listDecoder =
    Decode.list decoder
