module Form.DynamicForm exposing
    ( extractFormFields
    , viewFields
    )

{-| 動的フォーム生成モジュール

WorkflowDefinition の definition フィールドから動的にフォーム要素を生成する。

詳細: [UI 設計](../../../../docs/03_詳細設計書/10_ワークフロー申請フォームUI設計.md)


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
    div [ class "dynamic-form-fields" ]
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
    div
        [ style "margin-bottom" "1rem"
        , classList
            [ ( "form-field", True )
            , ( "form-field--error", maybeError /= Nothing )
            ]
        ]
        [ viewLabel field
        , viewInput field value onInputMsg
        , viewError maybeError
        ]


{-| ラベルを描画
-}
viewLabel : FormField -> Html msg
viewLabel field =
    label
        [ for field.id
        , style "display" "block"
        , style "margin-bottom" "0.5rem"
        , style "font-weight" "500"
        ]
        [ text field.label
        , if field.validation.required then
            span [ style "color" "#d93025" ] [ text " *" ]

          else
            text ""
        ]


{-| フィールドタイプに応じた入力要素を描画

Elm の case 式による網羅性チェックにより、
新しい FieldType を追加した際にコンパイラが警告を出す。

-}
viewInput : FormField -> String -> (String -> msg) -> Html msg
viewInput field value onInputMsg =
    case field.fieldType of
        Text ->
            viewTextInput field value onInputMsg

        Number ->
            viewNumberInput field value onInputMsg

        Select options ->
            viewSelectInput field value options onInputMsg

        Date ->
            viewDateInput field value onInputMsg

        File ->
            viewFileInput field


{-| テキスト入力
-}
viewTextInput : FormField -> String -> (String -> msg) -> Html msg
viewTextInput field value onInputMsg =
    input
        [ type_ "text"
        , id field.id
        , name field.id
        , Html.Attributes.value value
        , placeholder (Maybe.withDefault "" field.placeholder)
        , onInput onInputMsg
        , style "width" "100%"
        , style "padding" "0.75rem"
        , style "border" "1px solid #dadce0"
        , style "border-radius" "4px"
        , style "font-size" "1rem"
        , style "box-sizing" "border-box"
        ]
        []


{-| 数値入力
-}
viewNumberInput : FormField -> String -> (String -> msg) -> Html msg
viewNumberInput field value onInputMsg =
    input
        [ type_ "number"
        , id field.id
        , name field.id
        , Html.Attributes.value value
        , placeholder (Maybe.withDefault "" field.placeholder)
        , onInput onInputMsg
        , style "width" "100%"
        , style "padding" "0.75rem"
        , style "border" "1px solid #dadce0"
        , style "border-radius" "4px"
        , style "font-size" "1rem"
        , style "box-sizing" "border-box"

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
        []


{-| ドロップダウン選択
-}
viewSelectInput : FormField -> String -> List FormField.SelectOption -> (String -> msg) -> Html msg
viewSelectInput field value options onInputMsg =
    select
        [ id field.id
        , name field.id
        , onInput onInputMsg
        , style "width" "100%"
        , style "padding" "0.75rem"
        , style "border" "1px solid #dadce0"
        , style "border-radius" "4px"
        , style "font-size" "1rem"
        , style "background-color" "white"
        , style "cursor" "pointer"
        ]
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
viewDateInput : FormField -> String -> (String -> msg) -> Html msg
viewDateInput field value onInputMsg =
    input
        [ type_ "date"
        , id field.id
        , name field.id
        , Html.Attributes.value value
        , onInput onInputMsg
        , style "width" "100%"
        , style "padding" "0.75rem"
        , style "border" "1px solid #dadce0"
        , style "border-radius" "4px"
        , style "font-size" "1rem"
        , style "box-sizing" "border-box"
        ]
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
            , style "padding" "0.5rem 0"
            ]
            []
        , p
            [ style "font-size" "0.875rem"
            , style "color" "#5f6368"
            , style "margin-top" "0.25rem"
            ]
            [ text "※ ファイルアップロードは現在準備中です" ]
        ]


{-| エラーメッセージを描画
-}
viewError : Maybe String -> Html msg
viewError maybeError =
    case maybeError of
        Just errorMsg ->
            div
                [ style "color" "#d93025"
                , style "font-size" "0.875rem"
                , style "margin-top" "0.25rem"
                , attribute "role" "alert"
                ]
                [ text errorMsg ]

        Nothing ->
            text ""
