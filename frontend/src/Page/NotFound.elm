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
    div
        [ style "text-align" "center"
        , style "padding" "3rem"
        ]
        [ h2
            [ style "font-size" "3rem"
            , style "color" "#5f6368"
            , style "margin-bottom" "1rem"
            ]
            [ text "404" ]
        , p
            [ style "font-size" "1.25rem"
            , style "color" "#5f6368"
            , style "margin-bottom" "2rem"
            ]
            [ text "お探しのページは存在しません。" ]
        , a
            [ href "/"
            , style "display" "inline-block"
            , style "padding" "0.75rem 1.5rem"
            , style "background-color" "#1a73e8"
            , style "color" "white"
            , style "text-decoration" "none"
            , style "border-radius" "4px"
            ]
            [ text "ホームに戻る" ]
        ]
