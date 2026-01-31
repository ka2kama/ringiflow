module Page.NotFound exposing (view)

{-| 404 ページ

存在しない URL にアクセスした場合に表示する。

-}

import Html exposing (..)
import Html.Attributes exposing (..)


{-| 404 ページの描画
-}
view : Html msg
view =
    div [ class "py-12 text-center" ]
        [ h2 [ class "mb-4 text-6xl font-bold text-secondary-500" ]
            [ text "404" ]
        , p [ class "mb-8 text-lg text-secondary-500" ]
            [ text "お探しのページは存在しません。" ]
        , a
            [ href "/"
            , class "inline-flex items-center rounded-lg bg-primary-600 px-6 py-3 font-medium text-white transition-colors hover:bg-primary-700"
            ]
            [ text "ホームに戻る" ]
        ]
