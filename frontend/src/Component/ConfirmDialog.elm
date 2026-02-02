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

import Component.Button as Button
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
                    [ Button.view
                        { variant = Button.Outline
                        , disabled = False
                        , onClick = config.onCancel
                        }
                        [ text config.cancelLabel ]
                    , Button.view
                        { variant = actionStyleToVariant config.actionStyle
                        , disabled = False
                        , onClick = config.onConfirm
                        }
                        [ text config.confirmLabel ]
                    ]
                ]
            ]
        ]


{-| ActionStyle を Button.Variant にマッピング
-}
actionStyleToVariant : ActionStyle -> Button.Variant
actionStyleToVariant actionStyle =
    case actionStyle of
        Positive ->
            Button.Success

        Destructive ->
            Button.Error
