module Component.ErrorState exposing (containerClass, view, viewSimple)

{-| エラー状態コンポーネント

API 呼び出し失敗時のエラー表示を統一する。
`LoadingSpinner` が Loading 状態を担当するのに対し、本モジュールは Failure 状態を担当する。


## 使用例

    import Component.ErrorState as ErrorState

    -- リフレッシュボタン付き（RemoteData Failure での主要パターン）
    viewContent model =
        case model.data of
            Failure err ->
                ErrorState.view
                    { message = ErrorMessage.toUserMessage { entityName = "ユーザー" } err
                    , onRefresh = Refresh
                    }

            ...

    -- リフレッシュ不可の文脈（フォームの補助データ取得失敗など）
    viewRoleError =
        ErrorState.viewSimple "ロール情報の取得に失敗しました。"

-}

import Component.Button as Button
import Html exposing (..)
import Html.Attributes exposing (..)


{-| コンテナの CSS クラス（テスト用に公開）
-}
containerClass : String
containerClass =
    "rounded-lg bg-error-50 p-4 text-error-700"


{-| リフレッシュボタン付きエラー表示

`role="alert"` により、スクリーンリーダーがエラーの発生をアナウンスする。

-}
view :
    { message : String
    , onRefresh : msg
    }
    -> Html msg
view config =
    div
        [ class containerClass
        , attribute "role" "alert"
        ]
        [ p [] [ text config.message ]
        , Button.view
            { variant = Button.Outline
            , disabled = False
            , onClick = config.onRefresh
            }
            [ text "再読み込み" ]
        ]


{-| シンプルなエラー表示（リフレッシュボタンなし）

リフレッシュ操作が不可能な文脈（フォームの補助データ取得失敗など）で使用する。

-}
viewSimple : String -> Html msg
viewSimple message =
    div
        [ class containerClass
        , attribute "role" "alert"
        ]
        [ text message ]
