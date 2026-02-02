module Component.Badge exposing (view)

{-| バッジコンポーネント

ステータス表示に使用する共通バッジ。
外観構造（角丸、パディング、フォントサイズ）を統一し、色は呼び出し側が指定する。


## 使用例

    import Component.Badge as Badge
    import Data.WorkflowInstance as WorkflowInstance

    Badge.view
        { colorClass = WorkflowInstance.statusToCssClass workflow.status
        , label = WorkflowInstance.statusToJapanese workflow.status
        }

-}

import Html exposing (..)
import Html.Attributes exposing (..)


{-| バッジを表示

`colorClass` で背景色とテキスト色を指定し、`label` でテキストを設定する。
外観構造（rounded-full、パディング、フォントサイズ等）はコンポーネントが管理する。

-}
view : { colorClass : String, label : String } -> Html msg
view config =
    span [ class ("inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium " ++ config.colorClass) ]
        [ text config.label ]
