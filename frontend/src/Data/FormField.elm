module Data.FormField exposing
    ( FieldType(..)
    , FileConfig
    , FormField
    , SelectOption
    , Validation
    , decoder
    , defaultFileConfig
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
  - `File`: ファイルアップロード（FileConfig で制約をカスタマイズ可能）

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (custom, optional, required)



-- TYPES


{-| フォームフィールドの型
-}
type FieldType
    = Text
    | Number
    | Select (List SelectOption)
    | Date
    | File FileConfig


{-| ファイルフィールドの設定
-}
type alias FileConfig =
    { maxFiles : Int
    , maxFileSize : Int
    , allowedTypes : List String
    }


{-| デフォルトのファイル設定

  - maxFiles: 10
  - maxFileSize: 20MB（20971520 バイト）
  - allowedTypes: []（空 = 全形式許可）

-}
defaultFileConfig : FileConfig
defaultFileConfig =
    { maxFiles = 10
    , maxFileSize = 20971520
    , allowedTypes = []
    }


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
                        fileConfigDecoder
                            |> Decode.map File

                    _ ->
                        -- 未知のタイプはテキストとして扱う
                        Decode.succeed Text
            )


{-| ファイル設定をデコード（省略時はデフォルト値）
-}
fileConfigDecoder : Decoder FileConfig
fileConfigDecoder =
    Decode.succeed FileConfig
        |> optional "maxFiles" Decode.int defaultFileConfig.maxFiles
        |> optional "maxFileSize" Decode.int defaultFileConfig.maxFileSize
        |> optional "allowedTypes" (Decode.list Decode.string) defaultFileConfig.allowedTypes


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
        |> custom fieldTypeDecoder
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
