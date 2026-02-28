module Form.DynamicForm exposing
    ( extractFormFields
    , viewFields
    )

{-| 動的フォーム生成モジュール

WorkflowDefinition の definition フィールドから動的にフォーム要素を生成する。

詳細: [UI 設計](../../../../docs/40_詳細設計書/10_ワークフロー申請フォームUI設計.md)


## 設計方針

1.  **型安全**: FieldType のパターンマッチにより、各タイプに適切な入力要素を生成
2.  **疎結合**: ページモジュールは DynamicForm に依存するが、逆は依存しない
3.  **拡張性**: 新しいフィールドタイプ追加時、FieldType と viewInput の拡張のみ


## 使用例

    case extractFormFields definition of
        Ok fields ->
            viewFields fields model.formValues model.validationErrors UpdateField

        Err _ ->
            text "フォーム定義の読み込みに失敗しました"

-}

import Data.FormField as FormField exposing (FieldType(..), FormField)
import Dict exposing (Dict)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)
import Json.Decode as Decode



-- EXTRACT


{-| WorkflowDefinition.definition から FormField リストを抽出

definition JSON の構造:

    {
        "form": {
            "fields": [
                { "id": "amount", "label": "金額", "type": "number", ... },
                ...
            ]
        }
    }

-}
extractFormFields : Decode.Value -> Result Decode.Error (List FormField)
extractFormFields definition =
    Decode.decodeValue
        (Decode.at [ "form", "fields" ] FormField.listDecoder)
        definition



-- VIEW


{-| 全フィールドを描画

各フィールドの入力値は formValues Dict から取得。
バリデーションエラーは validationErrors Dict から取得。

-}
viewFields :
    List FormField
    -> Dict String String
    -> Dict String String
    -> (String -> String -> msg)
    -> Html msg
viewFields fields formValues validationErrors onInputMsg =
    div []
        (List.map
            (\field ->
                let
                    value =
                        Dict.get field.id formValues
                            |> Maybe.withDefault ""

                    maybeError =
                        Dict.get field.id validationErrors
                in
                viewField field value maybeError (onInputMsg field.id)
            )
            fields
        )


{-| 単一フィールドを描画
-}
viewField :
    FormField
    -> String
    -> Maybe String
    -> (String -> msg)
    -> Html msg
viewField field value maybeError onInputMsg =
    let
        errorId =
            field.id ++ "-error"
    in
    div
        [ class "mb-4" ]
        [ viewLabel field
        , viewInput field value maybeError onInputMsg
        , viewError errorId maybeError
        ]


{-| ラベルを描画
-}
viewLabel : FormField -> Html msg
viewLabel field =
    label
        [ for field.id
        , class "mb-2 block font-medium"
        ]
        [ text field.label
        , if field.validation.required then
            span [ class "text-error-600" ] [ text " *" ]

          else
            text ""
        ]


{-| フィールドタイプに応じた入力要素を描画

Elm の case 式による網羅性チェックにより、
新しい FieldType を追加した際にコンパイラが警告を出す。

-}
viewInput : FormField -> String -> Maybe String -> (String -> msg) -> Html msg
viewInput field value maybeError onInputMsg =
    case field.fieldType of
        Text ->
            viewTextInput field value maybeError onInputMsg

        Number ->
            viewNumberInput field value maybeError onInputMsg

        Select options ->
            viewSelectInput field value maybeError options onInputMsg

        Date ->
            viewDateInput field value maybeError onInputMsg

        File ->
            viewFileInput field


{-| テキスト入力
-}
viewTextInput : FormField -> String -> Maybe String -> (String -> msg) -> Html msg
viewTextInput field value maybeError onInputMsg =
    input
        ([ type_ "text"
         , id field.id
         , name field.id
         , Html.Attributes.value value
         , placeholder (Maybe.withDefault "" field.placeholder)
         , onInput onInputMsg
         , class "w-full rounded border border-secondary-300 bg-white px-3 py-3 text-base outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
         ]
            ++ ariaErrorAttrs field.id maybeError
        )
        []


{-| 数値入力
-}
viewNumberInput : FormField -> String -> Maybe String -> (String -> msg) -> Html msg
viewNumberInput field value maybeError onInputMsg =
    input
        ([ type_ "number"
         , id field.id
         , name field.id
         , Html.Attributes.value value
         , placeholder (Maybe.withDefault "" field.placeholder)
         , onInput onInputMsg
         , class "w-full rounded border border-secondary-300 bg-white px-3 py-3 text-base outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"

         -- 数値バリデーション属性を追加
         , case field.validation.min of
            Just minVal ->
                Html.Attributes.min (String.fromFloat minVal)

            Nothing ->
                class ""
         , case field.validation.max of
            Just maxVal ->
                Html.Attributes.max (String.fromFloat maxVal)

            Nothing ->
                class ""
         ]
            ++ ariaErrorAttrs field.id maybeError
        )
        []


{-| ドロップダウン選択
-}
viewSelectInput : FormField -> String -> Maybe String -> List FormField.SelectOption -> (String -> msg) -> Html msg
viewSelectInput field value maybeError options onInputMsg =
    select
        ([ id field.id
         , name field.id
         , onInput onInputMsg
         , class "w-full cursor-pointer rounded border border-secondary-100 bg-white px-3 py-3 text-base outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
         ]
            ++ ariaErrorAttrs field.id maybeError
        )
        (option
            [ Html.Attributes.value ""
            , selected (value == "")
            ]
            [ text "選択してください" ]
            :: List.map
                (\opt ->
                    option
                        [ Html.Attributes.value opt.value
                        , selected (value == opt.value)
                        ]
                        [ text opt.label ]
                )
                options
        )


{-| 日付入力
-}
viewDateInput : FormField -> String -> Maybe String -> (String -> msg) -> Html msg
viewDateInput field value maybeError onInputMsg =
    input
        ([ type_ "date"
         , id field.id
         , name field.id
         , Html.Attributes.value value
         , onInput onInputMsg
         , class "w-full rounded border border-secondary-300 bg-white px-3 py-3 text-base outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
         ]
            ++ ariaErrorAttrs field.id maybeError
        )
        []


{-| ファイル入力

MVP では簡易実装。ファイル選択のみでアップロードはなし。
Elm でファイル操作は Ports 経由が必要なため、将来的に実装予定。

-}
viewFileInput : FormField -> Html msg
viewFileInput field =
    div []
        [ input
            [ type_ "file"
            , id field.id
            , name field.id
            , class "py-2"
            ]
            []
        , p
            [ class "mt-1 text-sm text-secondary-500" ]
            [ text "※ ファイルアップロードは現在準備中です" ]
        ]



-- ERROR & ACCESSIBILITY


{-| バリデーションエラー時のアクセシビリティ属性

エラーがある場合、`aria-invalid` と `aria-describedby` を設定する。
スクリーンリーダーがエラー状態とエラーメッセージの関連を認識できるようになる。

-}
ariaErrorAttrs : String -> Maybe String -> List (Html.Attribute msg)
ariaErrorAttrs fieldId maybeError =
    case maybeError of
        Just _ ->
            [ attribute "aria-invalid" "true"
            , attribute "aria-describedby" (fieldId ++ "-error")
            ]

        Nothing ->
            []


{-| エラーメッセージを描画
-}
viewError : String -> Maybe String -> Html msg
viewError errorId maybeError =
    case maybeError of
        Just errorMsg ->
            div
                [ id errorId
                , class "mt-1 text-sm text-error-600"
                , attribute "role" "alert"
                ]
                [ text errorMsg ]

        Nothing ->
            text ""
