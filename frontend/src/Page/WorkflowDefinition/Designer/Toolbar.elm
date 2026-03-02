module Page.WorkflowDefinition.Designer.Toolbar exposing
    ( viewMessages
    , viewPublishDialog
    , viewStatusBar
    , viewToolbar
    , viewValidationPanel
    )

{-| ツールバー・メッセージ・バリデーション・ステータスバー・公開ダイアログ

Designer ページ上部・下部の UI コンポーネントを集約する。

-}

import Component.Button as Button
import Component.ConfirmDialog as ConfirmDialog
import Component.MessageAlert as MessageAlert
import Data.WorkflowDefinition exposing (ValidationError)
import Dict
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events
import Page.WorkflowDefinition.Designer.Types exposing (CanvasState, Msg(..))


{-| ツールバー（定義名 + バリデーション・保存・公開ボタン）
-}
viewToolbar : CanvasState -> Html Msg
viewToolbar canvas =
    div [ class "flex items-center gap-4 border-b border-secondary-200 bg-white px-4 py-2" ]
        [ h1 [ class "shrink-0 text-base font-semibold text-secondary-800" ]
            [ text "ワークフローデザイナー" ]
        , input
            [ type_ "text"
            , class "min-w-0 flex-1 rounded border border-secondary-300 px-3 py-1 text-sm focus:border-primary-500 focus:ring-1 focus:ring-primary-500"
            , value canvas.name
            , Html.Events.onInput UpdateDefinitionName
            , placeholder "定義名を入力"
            ]
            []
        , div [ class "flex shrink-0 gap-2" ]
            [ Button.view
                { variant = Button.Outline
                , disabled = canvas.isValidating
                , onClick = ValidateClicked
                }
                [ text
                    (if canvas.isValidating then
                        "検証中..."

                     else
                        "検証"
                    )
                ]
            , Button.view
                { variant = Button.Primary
                , disabled = canvas.isSaving || not canvas.isDirty_
                , onClick = SaveClicked
                }
                [ text
                    (if canvas.isSaving then
                        "保存中..."

                     else
                        "保存"
                    )
                ]
            , Button.view
                { variant = Button.Success
                , disabled = canvas.isPublishing || canvas.isSaving || canvas.isValidating
                , onClick = PublishClicked
                }
                [ text
                    (if canvas.isPublishing then
                        "公開中..."

                     else
                        "公開"
                    )
                ]
            ]
        ]


{-| 成功・エラーメッセージ表示
-}
viewMessages : CanvasState -> Html Msg
viewMessages canvas =
    MessageAlert.view
        { onDismiss = DismissMessage
        , successMessage = canvas.successMessage
        , errorMessage = canvas.errorMessage
        }


{-| バリデーション結果パネル

キャンバス下部に表示。valid なら緑、invalid ならエラー一覧。

-}
viewValidationPanel : CanvasState -> Html Msg
viewValidationPanel canvas =
    case canvas.validationResult of
        Just result ->
            if result.valid then
                div [ class "border-t border-success-200 bg-success-50 px-4 py-2 text-sm text-success-700" ]
                    [ text "フロー定義は有効です" ]

            else
                div [ class "border-t border-error-200 bg-error-50 px-4 py-2" ]
                    [ p [ class "text-sm font-medium text-error-700" ]
                        [ text ("バリデーションエラー（" ++ String.fromInt (List.length result.errors) ++ " 件）") ]
                    , ul [ class "mt-1 space-y-1" ]
                        (List.map viewValidationError result.errors)
                    ]

        Nothing ->
            text ""


{-| 個別のバリデーションエラー行

stepId がある場合、クリックで該当ステップを選択する。

-}
viewValidationError : ValidationError -> Html Msg
viewValidationError error =
    li
        (class "text-sm text-error-600"
            :: (case error.stepId of
                    Just stepId ->
                        [ class "cursor-pointer hover:underline"
                        , Html.Events.onClick (StepClicked stepId)
                        ]

                    Nothing ->
                        []
               )
        )
        [ text error.message ]


{-| ステータスバー
-}
viewStatusBar : CanvasState -> Html Msg
viewStatusBar canvas =
    div [ class "border-t border-secondary-200 bg-white px-4 py-1.5 text-xs text-secondary-500" ]
        [ text
            (String.fromInt (Dict.size canvas.steps)
                ++ " ステップ / "
                ++ String.fromInt (List.length canvas.transitions)
                ++ " 接続"
            )
        ]


{-| 公開確認ダイアログ
-}
viewPublishDialog : CanvasState -> Html Msg
viewPublishDialog canvas =
    if canvas.pendingPublish then
        ConfirmDialog.view
            { title = "ワークフロー定義を公開"
            , message = "「" ++ canvas.name ++ "」を公開しますか？公開後はユーザーが申請に使用できるようになります。"
            , confirmLabel = "公開する"
            , cancelLabel = "キャンセル"
            , onConfirm = ConfirmPublish
            , onCancel = CancelPublish
            , actionStyle = ConfirmDialog.Positive
            }

    else
        text ""
