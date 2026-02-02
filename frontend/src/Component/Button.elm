module Component.Button exposing (Variant(..), link, variantClass, view)

{-| ボタンコンポーネント

Tailwind CSS ベースの共通ボタン。5 つの Variant で色を制御する。

レイアウト（margin 等）は親要素で制御する。


## 使用例

    import Component.Button as Button

    -- <button> 要素
    Button.view
        { variant = Button.Primary
        , disabled = False
        , onClick = ClickSubmit
        }
        [ text "送信" ]

    -- <a> 要素（リンクボタン）
    Button.link
        { variant = Button.Success
        , href = "/workflows"
        }
        [ text "申請一覧" ]

-}

import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick)


{-| ボタンの色バリエーション

  - Primary: メイン CTA（新規申請など）
  - Success: 承認など前向きな操作
  - Error: 却下など否定的な操作
  - Warning: 注意喚起
  - Outline: セカンダリーアクション（キャンセル、再読み込みなど）

-}
type Variant
    = Primary
    | Success
    | Error
    | Warning
    | Outline


{-| Variant に応じた CSS クラス（テスト用に公開）
-}
variantClass : Variant -> String
variantClass variant =
    case variant of
        Primary ->
            "bg-primary-600 hover:bg-primary-700 text-white"

        Success ->
            "bg-success-600 hover:bg-success-700 text-white"

        Error ->
            "bg-error-600 hover:bg-error-700 text-white"

        Warning ->
            "bg-warning-600 hover:bg-warning-700 text-white"

        Outline ->
            "border border-secondary-300 bg-white text-secondary-700 hover:bg-secondary-50"


{-| アクションボタン（`<button>` 要素）

`type="button"` をデフォルトで設定し、form 内での意図しない submit を防止する。

-}
view :
    { variant : Variant
    , disabled : Bool
    , onClick : msg
    }
    -> List (Html msg)
    -> Html msg
view config children =
    button
        [ type_ "button"
        , class (baseClass ++ " " ++ variantClass config.variant ++ " disabled:opacity-50 disabled:cursor-not-allowed")
        , disabled config.disabled
        , onClick config.onClick
        ]
        children


{-| リンクボタン（`<a>` 要素）

ナビゲーション用。ボタンと同じ外観でリンクを描画する。

-}
link :
    { variant : Variant
    , href : String
    }
    -> List (Html msg)
    -> Html msg
link config children =
    a
        [ href config.href
        , class (baseClass ++ " no-underline " ++ variantClass config.variant)
        ]
        children


{-| 全 variant 共通の基本クラス

サイズ（px-4 py-2）、角丸（rounded-lg）、フォント、カーソルを統一する。
レイアウト（margin 等）は親要素で制御する。

-}
baseClass : String
baseClass =
    "inline-flex items-center justify-center px-4 py-2 rounded-lg text-sm font-semibold cursor-pointer transition-colors"
