module Component.ConfirmDialog exposing
    ( ActionStyle(..)
    , backdropClickDecoder
    , dialogId
    , view
    )

{-| 確認ダイアログコンポーネント

破壊的操作（承認・却下）の前にユーザーに確認を求めるモーダルダイアログ。
HTML `<dialog>` 要素と `showModal()` を使用し、フォーカストラップと
ARIA ラベリングをブラウザネイティブで提供する。

詳細: [WAI-ARIA Dialog パターン](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)

型変数 `msg` により、各ページの `Msg` 型に対応。


## 使用例

    import Component.ConfirmDialog as ConfirmDialog
    import Ports

    -- update で showModal() を呼ぶ
    ShowDialog ->
        ( { model | pendingAction = Just action }
        , Ports.showModalDialog ConfirmDialog.dialogId
        )

    -- view でダイアログを描画
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
import Html.Events exposing (preventDefaultOn)
import Json.Decode as Decode


{-| 確認ボタンのスタイル

  - Positive: 承認など前向きな操作（success-600 系）
  - Caution: 差し戻しなど注意を要する操作（warning-600 系）
  - Destructive: 却下など破壊的な操作（error-600 系）

-}
type ActionStyle
    = Positive
    | Caution
    | Destructive


{-| 確認ダイアログを表示

`<dialog>` 要素を使用し、以下をブラウザネイティブで提供する:

  - フォーカストラップ: `showModal()` が Tab/Shift+Tab をダイアログ内に閉じ込める
  - ESC キー: `cancel` イベントで `onCancel` を発火（`preventDefaultOn` でネイティブ閉じを防止）
  - ARIA: `aria-labelledby` / `aria-describedby` でスクリーンリーダー対応
  - 初期フォーカス: キャンセルボタンの `autofocus` で `showModal()` が自動設定

バックドロップクリックで `onCancel` を発行する（`pointer-events` + `target.nodeName` 検出）。

注意: 表示には `Ports.showModalDialog dialogId` の呼び出しが必要。

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
    Html.node "dialog"
        [ id dialogId
        , class "fixed inset-0 m-0 h-full w-full max-h-none max-w-none bg-transparent p-0 border-none outline-none"
        , attribute "aria-labelledby" titleId
        , attribute "aria-describedby" messageId
        , preventDefaultOn "cancel"
            (Decode.succeed ( config.onCancel, True ))
        , Html.Events.on "click"
            (backdropClickDecoder config.onCancel)
        ]
        [ div [ class "flex h-full w-full items-center justify-center pointer-events-none" ]
            [ div [ class "dialog-content pointer-events-auto w-full max-w-md rounded-lg bg-white p-6 shadow-xl" ]
                [ h2 [ id titleId, class "text-lg font-semibold text-secondary-900" ] [ text config.title ]
                , p [ id messageId, class "mt-2 text-sm text-secondary-600" ] [ text config.message ]
                , div [ class "mt-6 flex justify-end gap-3" ]
                    [ Button.viewWithAttrs
                        { variant = Button.Outline
                        , disabled = False
                        , onClick = config.onCancel
                        }
                        [ Html.Attributes.id cancelButtonId
                        , Html.Attributes.autofocus True
                        ]
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


{-| ダイアログ要素の HTML id

`Ports.showModalDialog` のターゲットとして使用する。

-}
dialogId : String
dialogId =
    "confirm-dialog"


{-| バックドロップクリックを検出するデコーダ

`event.target.nodeName` が `"DIALOG"` の場合にのみ成功する。
`<dialog>` 要素を全画面透明に設定し、内部の flex コンテナに `pointer-events-none`、
ダイアログボックスに `pointer-events-auto` を設定することで、
ボックス外のクリックが `<dialog>` 要素に到達する。

-}
backdropClickDecoder : msg -> Decode.Decoder msg
backdropClickDecoder msg =
    Decode.at [ "target", "nodeName" ] Decode.string
        |> Decode.andThen
            (\nodeName ->
                if nodeName == "DIALOG" then
                    Decode.succeed msg

                else
                    Decode.fail ("Click was not on the backdrop: " ++ nodeName)
            )


{-| キャンセルボタンの HTML id

Phase 2 のページ側更新後に非公開化予定。

-}
cancelButtonId : String
cancelButtonId =
    "confirm-dialog-cancel"


{-| タイトル要素の HTML id（内部使用）
-}
titleId : String
titleId =
    "confirm-dialog-title"


{-| メッセージ要素の HTML id（内部使用）
-}
messageId : String
messageId =
    "confirm-dialog-message"


{-| ActionStyle を Button.Variant にマッピング
-}
actionStyleToVariant : ActionStyle -> Button.Variant
actionStyleToVariant actionStyle =
    case actionStyle of
        Positive ->
            Button.Success

        Caution ->
            Button.Warning

        Destructive ->
            Button.Error
