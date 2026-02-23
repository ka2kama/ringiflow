module Component.FormField exposing
    ( inputClass
    , viewReadOnlyField
    , viewSelectField
    , viewTextArea
    , viewTextField
    )

{-| フォームフィールドコンポーネント

ラベル + 入力 + エラー表示を統一したフォームフィールド群。
User/Role の Edit/New ページで共通利用する。

各関数は `fieldId` を受け取り、`<label for>` と `<input id>` で
明示的に関連付ける（WCAG 2.1 AA 準拠）。


## 使用例

    import Component.FormField as FormField

    FormField.viewTextField
        { label = "名前"
        , value = model.name
        , onInput = UpdateName
        , error = Dict.get "name" model.validationErrors
        , inputType = "text"
        , placeholder = "山田 太郎"
        , fieldId = "user-name"
        }

-}

import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)


{-| 入力フィールドの CSS クラス（テスト用に公開）

エラーの有無に応じてボーダー色・フォーカス色を切り替える。

-}
inputClass : Maybe String -> String
inputClass error =
    "w-full rounded-lg border px-3 py-2 text-sm outline-none "
        ++ (case error of
                Just _ ->
                    "border-error-300 focus-visible:ring-2 focus-visible:ring-error-500 focus-visible:border-error-500"

                Nothing ->
                    "border-secondary-300 focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
           )


{-| テキスト入力フィールド
-}
viewTextField :
    { label : String
    , value : String
    , onInput : String -> msg
    , error : Maybe String
    , inputType : String
    , placeholder : String
    , fieldId : String
    }
    -> Html msg
viewTextField config =
    div []
        [ label [ for config.fieldId, class "block text-sm font-medium text-secondary-700 mb-1" ] [ text config.label ]
        , input
            [ id config.fieldId
            , type_ config.inputType
            , value config.value
            , onInput config.onInput
            , placeholder config.placeholder
            , class (inputClass config.error)
            ]
            []
        , viewError config.error
        ]


{-| テキストエリアフィールド
-}
viewTextArea :
    { label : String
    , value : String
    , onInput : String -> msg
    , placeholder : String
    , fieldId : String
    }
    -> Html msg
viewTextArea config =
    div []
        [ label [ for config.fieldId, class "block text-sm font-medium text-secondary-700 mb-1" ] [ text config.label ]
        , textarea
            [ id config.fieldId
            , value config.value
            , onInput config.onInput
            , placeholder config.placeholder
            , rows 3
            , class "w-full rounded-lg border border-secondary-300 px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
            ]
            []
        ]


{-| セレクトフィールド
-}
viewSelectField :
    { label : String
    , value : String
    , onInput : String -> msg
    , error : Maybe String
    , options : List { value : String, label : String }
    , placeholder : String
    , fieldId : String
    }
    -> Html msg
viewSelectField config =
    div []
        [ label [ for config.fieldId, class "block text-sm font-medium text-secondary-700 mb-1" ] [ text config.label ]
        , select
            [ id config.fieldId
            , class (inputClass config.error)
            , onInput config.onInput
            , value config.value
            ]
            (option [ Html.Attributes.value "" ] [ text config.placeholder ]
                :: List.map
                    (\opt ->
                        option [ Html.Attributes.value opt.value ] [ text opt.label ]
                    )
                    config.options
            )
        , viewError config.error
        ]


{-| 読み取り専用フィールド
-}
viewReadOnlyField : String -> String -> String -> Html msg
viewReadOnlyField fieldId labelText fieldValue =
    div []
        [ label [ for fieldId, class "block text-sm font-medium text-secondary-700 mb-1" ] [ text labelText ]
        , div [ id fieldId, class "w-full rounded-lg border border-secondary-200 bg-secondary-50 px-3 py-2 text-sm text-secondary-500" ]
            [ text fieldValue ]
        ]


{-| エラーメッセージ表示（内部ヘルパー）
-}
viewError : Maybe String -> Html msg
viewError error =
    case error of
        Just errorMsg ->
            p [ class "mt-1 text-sm text-error-600" ] [ text errorMsg ]

        Nothing ->
            text ""
