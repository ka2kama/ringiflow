module Component.ConfirmDialog exposing (ActionStyle(..), view)

{-| 確認ダイアログコンポーネント

破壊的操作（承認・却下）の前にユーザーに確認を求めるモーダルダイアログ。

型変数 `msg` により、各ページの `Msg` 型に対応。


## 使用例

    import Component.ConfirmDialog as ConfirmDialog

    viewConfirmDialog : Html Msg
    viewConfirmDialog =
        ConfirmDialog.view
            { title = "承認の確認"
            , message = "この申請を承認しますか？"
            , confirmLabel = "承認する"
            , cancelLabel = "キャンセル"
            , onConfirm = ConfirmAction
            , onCancel = CancelAction
            , actionStyle = ConfirmDialog.Positive
            }

-}

import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick)


{-| 確認ボタンのスタイル

  - Positive: 承認など前向きな操作（success-600 系）
  - Destructive: 却下など破壊的な操作（error-600 系）

-}
type ActionStyle
    = Positive
    | Destructive


{-| 確認ダイアログを表示

オーバーレイクリックで `onCancel` を発行する。

-}
view :
    { title : String
    , message : String
    , confirmLabel : String
    , cancelLabel : String
    , onConfirm : msg
    , onCancel : msg
    , actionStyle : ActionStyle
    }
    -> Html msg
view config =
    div []
        [ -- オーバーレイ
          div
            [ class "fixed inset-0 z-40 bg-black/50"
            , onClick config.onCancel
            ]
            []

        -- ダイアログ本体
        , div
            [ class "fixed inset-0 z-50 flex items-center justify-center"
            , attribute "role" "dialog"
            , attribute "aria-modal" "true"
            ]
            [ div [ class "w-full max-w-md rounded-lg bg-white p-6 shadow-xl" ]
                [ h2 [ class "text-lg font-semibold text-secondary-900" ] [ text config.title ]
                , p [ class "mt-2 text-sm text-secondary-600" ] [ text config.message ]
                , div [ class "mt-6 flex justify-end gap-3" ]
                    [ button
                        [ class "inline-flex items-center rounded-lg border border-secondary-300 bg-white px-4 py-2 text-sm font-medium text-secondary-700 transition-colors hover:bg-secondary-50"
                        , onClick config.onCancel
                        ]
                        [ text config.cancelLabel ]
                    , button
                        [ class (confirmButtonClass config.actionStyle)
                        , onClick config.onConfirm
                        ]
                        [ text config.confirmLabel ]
                    ]
                ]
            ]
        ]


{-| ActionStyle に応じた確認ボタンの CSS クラス
-}
confirmButtonClass : ActionStyle -> String
confirmButtonClass actionStyle =
    let
        colorClasses =
            case actionStyle of
                Positive ->
                    "bg-success-600 hover:bg-success-700"

                Destructive ->
                    "bg-error-600 hover:bg-error-700"
    in
    "inline-flex items-center rounded-lg px-4 py-2 text-sm font-medium text-white transition-colors " ++ colorClasses
