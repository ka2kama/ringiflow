module Page.NotFound exposing (view)

{-| 404 ページ

存在しない URL にアクセスした場合に表示する。

-}

import Component.Button as Button
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
        , Button.link
            { variant = Button.Primary
            , href = "/"
            }
            [ text "ホームに戻る" ]
        ]
