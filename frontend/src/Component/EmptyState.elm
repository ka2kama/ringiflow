module Component.EmptyState exposing (containerClass, view)

{-| 空状態コンポーネント

データが存在しない場合の表示を統一する。
`LoadingSpinner` が Loading 状態、`ErrorState` が Failure 状態を担当するのに対し、
本モジュールは Success だがデータが空の状態を担当する。


## 使用例

    import Component.EmptyState as EmptyState


    -- シンプルな空状態
    viewUserList users =
        if List.isEmpty users then
            EmptyState.view
                { message = "ユーザーが見つかりません。"
                , description = Nothing
                }

        else
            viewTable users

    -- 説明付き空状態
    viewTaskList tasks =
        if List.isEmpty tasks then
            EmptyState.view
                { message = "承認待ちのタスクはありません"
                , description = Just "新しいタスクが割り当てられるとここに表示されます"
                }

        else
            viewTable tasks

-}

import Html exposing (..)
import Html.Attributes exposing (..)


{-| コンテナの CSS クラス（テスト用に公開）
-}
containerClass : String
containerClass =
    "py-12 text-center"


{-| 空状態を表示

`description` が `Just` の場合、メインメッセージの下に補助説明を表示する。

-}
view :
    { message : String
    , description : Maybe String
    }
    -> Html msg
view config =
    div [ class containerClass ]
        [ p [ class "text-secondary-500" ] [ text config.message ]
        , viewDescription config.description
        ]


viewDescription : Maybe String -> Html msg
viewDescription maybeDescription =
    case maybeDescription of
        Just description ->
            p [ class "mt-2 text-sm text-secondary-400" ] [ text description ]

        Nothing ->
            text ""
