module Page.Home exposing (view)

{-| ホームページ

アプリケーションのトップページを表示する。


## 将来の拡張

  - ダッシュボード（未処理タスク数、最近の申請など）
  - クイックアクション（新規申請ボタンなど）

-}

import Html exposing (..)
import Html.Attributes exposing (..)


{-| ホームページの描画
-}
view : Html msg
view =
    div []
        [ h2 [] [ text "ようこそ RingiFlow へ" ]
        , p [] [ text "ワークフロー管理システムです。" ]
        , viewQuickActions
        , viewStatus
        ]


{-| クイックアクションエリア
-}
viewQuickActions : Html msg
viewQuickActions =
    div
        [ style "display" "flex"
        , style "gap" "1rem"
        , style "margin-top" "1.5rem"
        ]
        [ a
            [ href "/workflows/new"
            , style "display" "inline-block"
            , style "padding" "0.75rem 1.5rem"
            , style "background-color" "#1a73e8"
            , style "color" "white"
            , style "text-decoration" "none"
            , style "border-radius" "4px"
            ]
            [ text "新規申請" ]
        ]


{-| ステータス表示
-}
viewStatus : Html msg
viewStatus =
    div
        [ style "background-color" "white"
        , style "padding" "1.5rem"
        , style "border-radius" "8px"
        , style "box-shadow" "0 2px 4px rgba(0,0,0,0.1)"
        , style "margin-top" "1.5rem"
        ]
        [ h3 [] [ text "Phase 2 実装中" ]
        , p [] [ text "申請フォーム UI を構築しています。" ]
        ]
