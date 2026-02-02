module Component.LoadingSpinner exposing (view)

{-| ローディングスピナーコンポーネント

データ読み込み中に表示するスピナーとメッセージ。

型変数 `msg` により、どのページからでも利用可能。


## 使用例

    import Component.LoadingSpinner as LoadingSpinner

    viewContent model =
        case model.data of
            Loading ->
                LoadingSpinner.view

            Success data ->
                viewData data

-}

import Html exposing (..)
import Html.Attributes exposing (..)


{-| ローディングスピナーを表示

`role="status"` により、スクリーンリーダーがコンテンツの変化をアナウンスする。

-}
view : Html msg
view =
    div
        [ class "flex flex-col items-center justify-center py-8"
        , attribute "role" "status"
        , attribute "aria-label" "読み込み中"
        ]
        [ div [ class "h-8 w-8 animate-spin rounded-full border-4 border-secondary-100 border-t-primary-600" ] []
        , p [ class "mt-4 text-secondary-500" ] [ text "読み込み中..." ]
        ]
