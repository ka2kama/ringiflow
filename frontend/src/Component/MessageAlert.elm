module Component.MessageAlert exposing (view)

{-| メッセージアラートコンポーネント

成功/エラーメッセージを表示し、×ボタンで非表示にできるアラート。

型変数 `msg` により、各ページの `Msg` 型に対応。


## 使用例

    import Component.MessageAlert as MessageAlert

    view model =
        div []
            [ MessageAlert.view
                { onDismiss = DismissMessage
                , successMessage = model.successMessage
                , errorMessage = model.errorMessage
                }
            , viewContent model
            ]

-}

import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick)


{-| 成功/エラーメッセージアラートを表示

両方のメッセージが存在する場合は両方表示する。
どちらも Nothing の場合は空の HTML を返す。

-}
view :
    { onDismiss : msg
    , successMessage : Maybe String
    , errorMessage : Maybe String
    }
    -> Html msg
view config =
    div [ class "space-y-2 mb-4" ]
        [ viewSuccessMessage config.onDismiss config.successMessage
        , viewErrorMessage config.onDismiss config.errorMessage
        ]


viewSuccessMessage : msg -> Maybe String -> Html msg
viewSuccessMessage onDismiss maybeMessage =
    case maybeMessage of
        Just message ->
            div [ class "flex items-center justify-between rounded-lg bg-success-50 p-4 text-success-700 animate-alert-in", attribute "role" "alert" ]
                [ text message
                , button [ class "ml-4 cursor-pointer bg-transparent border-0 text-lg", attribute "aria-label" "閉じる", onClick onDismiss ] [ text "×" ]
                ]

        Nothing ->
            text ""


viewErrorMessage : msg -> Maybe String -> Html msg
viewErrorMessage onDismiss maybeMessage =
    case maybeMessage of
        Just message ->
            div [ class "flex items-center justify-between rounded-lg bg-error-50 p-4 text-error-700 animate-alert-in", attribute "role" "alert" ]
                [ text message
                , button [ class "ml-4 cursor-pointer bg-transparent border-0 text-lg", attribute "aria-label" "閉じる", onClick onDismiss ] [ text "×" ]
                ]

        Nothing ->
            text ""
