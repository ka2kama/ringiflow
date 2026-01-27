module Form.Validation exposing
    ( ValidationResult
    , validateAllFields
    , validateTitle
    )

{-| フォームバリデーションモジュール

FormField の validation ルールに基づいてフォーム入力値を検証する。

詳細: [UI 設計](../../../../docs/03_詳細設計書/10_ワークフロー申請フォームUI設計.md)


## 設計方針

1.  **早期リターン**: 最初のエラーで検証を停止（複数エラーは混乱を招く）
2.  **型安全**: Result 型でエラーを表現、パターンマッチで処理を強制
3.  **拡張性**: バリデーションルール追加が容易な構造


## サポートするバリデーション

| ルール | 対象 FieldType | 説明 |
|--------|---------------|------|
| required | 全タイプ | 必須入力チェック |
| minLength | Text | 最小文字数 |
| maxLength | Text | 最大文字数 |
| min | Number | 最小値 |
| max | Number | 最大値 |

-}

import Data.FormField exposing (FieldType(..), FormField)
import Dict exposing (Dict)



-- TYPES


{-| バリデーション結果

  - `Ok ()`: 検証成功
  - `Err String`: 検証失敗（エラーメッセージ）

-}
type alias ValidationResult =
    Result String ()



-- SINGLE FIELD VALIDATION


{-| タイトルのバリデーション

タイトルは必須で、1文字以上200文字以内。

-}
validateTitle : String -> ValidationResult
validateTitle title =
    let
        trimmed =
            String.trim title
    in
    if String.isEmpty trimmed then
        Err "タイトルは必須です"

    else if String.length trimmed > 200 then
        Err "タイトルは200文字以内で入力してください"

    else
        Ok ()


{-| 単一フィールドのバリデーション

FormField の validation ルールに基づいて値を検証する。
複数のルールがある場合、最初に失敗したルールでエラーを返す。

-}
validateField : FormField -> String -> ValidationResult
validateField field value =
    let
        validation =
            field.validation

        -- 各チェックを順番に実行
        checks =
            [ checkRequired validation.required value
            , checkMinLength field.fieldType validation.minLength value
            , checkMaxLength field.fieldType validation.maxLength value
            , checkMin field.fieldType validation.min value
            , checkMax field.fieldType validation.max value
            ]
    in
    -- 最初のエラーを返す
    List.foldl combineResults (Ok ()) checks


{-| 必須チェック
-}
checkRequired : Bool -> String -> ValidationResult
checkRequired isRequired value =
    if isRequired && String.isEmpty (String.trim value) then
        Err "必須項目です"

    else
        Ok ()


{-| 最小文字数チェック（Text タイプのみ）
-}
checkMinLength : FieldType -> Maybe Int -> String -> ValidationResult
checkMinLength fieldType maybeMinLength value =
    case ( fieldType, maybeMinLength ) of
        ( Text, Just minLen ) ->
            if not (String.isEmpty value) && String.length value < minLen then
                Err ("最低 " ++ String.fromInt minLen ++ " 文字必要です")

            else
                Ok ()

        _ ->
            Ok ()


{-| 最大文字数チェック（Text タイプのみ）
-}
checkMaxLength : FieldType -> Maybe Int -> String -> ValidationResult
checkMaxLength fieldType maybeMaxLength value =
    case ( fieldType, maybeMaxLength ) of
        ( Text, Just maxLen ) ->
            if String.length value > maxLen then
                Err ("最大 " ++ String.fromInt maxLen ++ " 文字までです")

            else
                Ok ()

        _ ->
            Ok ()


{-| 最小値チェック（Number タイプのみ）
-}
checkMin : FieldType -> Maybe Float -> String -> ValidationResult
checkMin fieldType maybeMin value =
    case ( fieldType, maybeMin ) of
        ( Number, Just minVal ) ->
            case String.toFloat value of
                Just num ->
                    if num < minVal then
                        Err ("最小値は " ++ String.fromFloat minVal ++ " です")

                    else
                        Ok ()

                Nothing ->
                    -- 数値でない場合は required で検出
                    Ok ()

        _ ->
            Ok ()


{-| 最大値チェック（Number タイプのみ）
-}
checkMax : FieldType -> Maybe Float -> String -> ValidationResult
checkMax fieldType maybeMax value =
    case ( fieldType, maybeMax ) of
        ( Number, Just maxVal ) ->
            case String.toFloat value of
                Just num ->
                    if num > maxVal then
                        Err ("最大値は " ++ String.fromFloat maxVal ++ " です")

                    else
                        Ok ()

                Nothing ->
                    Ok ()

        _ ->
            Ok ()


{-| バリデーション結果を結合

先に失敗した結果を優先。

-}
combineResults : ValidationResult -> ValidationResult -> ValidationResult
combineResults new acc =
    case acc of
        Err _ ->
            acc

        Ok _ ->
            new



-- BULK VALIDATION


{-| 全フィールドのバリデーション

全フィールドを検証し、エラーがあるフィールドの Dict を返す。
キー: フィールド ID、値: エラーメッセージ

-}
validateAllFields :
    List FormField
    -> Dict String String
    -> Dict String String
validateAllFields fields values =
    fields
        |> List.filterMap
            (\field ->
                let
                    value =
                        Dict.get field.id values
                            |> Maybe.withDefault ""
                in
                case validateField field value of
                    Err msg ->
                        Just ( field.id, msg )

                    Ok _ ->
                        Nothing
            )
        |> Dict.fromList
