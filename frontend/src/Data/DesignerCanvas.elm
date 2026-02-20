module Data.DesignerCanvas exposing
    ( Bounds
    , Dimensions
    , DraggingState(..)
    , Position
    , StepColors
    , StepNode
    , StepType(..)
    , clientToCanvas
    , createStepFromDrop
    , decodeBounds
    , defaultStepName
    , generateStepId
    , snapToGrid
    , stepColors
    , stepDimensions
    , stepTypeToString
    )

{-| ワークフローデザイナー キャンバスのデータ型と純粋関数

ページモジュール（Designer.elm）からデータ型を分離することで、
テスタビリティを確保し、既存の Data/ パターンに準拠する。

-}

import Json.Decode as Decode
import Json.Encode as Encode


{-| ステップの種別
-}
type StepType
    = Start
    | Approval
    | End


{-| キャンバス上の座標
-}
type alias Position =
    { x : Float, y : Float }


{-| キャンバス上に配置されたステップノード
-}
type alias StepNode =
    { id : String
    , stepType : StepType
    , name : String
    , position : Position
    }


{-| SVG 要素の境界情報（getBoundingClientRect の結果）
-}
type alias Bounds =
    { x : Float, y : Float, width : Float, height : Float }


{-| ドラッグ操作の状態

  - DraggingExistingStep: 既存ステップの移動中（stepId, ステップ原点からのオフセット）
  - DraggingNewStep: パレットからの新規配置中（StepType, 現在のキャンバス座標）

-}
type DraggingState
    = DraggingExistingStep String Position
    | DraggingNewStep StepType Position


{-| グリッドサイズ（px）
-}
gridSize : Float
gridSize =
    20


{-| SVG viewBox の幅
-}
viewBoxWidth : Float
viewBoxWidth =
    1200


{-| SVG viewBox の高さ
-}
viewBoxHeight : Float
viewBoxHeight =
    800


{-| 座標をグリッドにスナップする

四捨五入で最も近いグリッド位置に合わせる。

-}
snapToGrid : Float -> Float
snapToGrid value =
    toFloat (round (value / gridSize)) * gridSize


{-| StepType を文字列に変換（ID 生成用）
-}
stepTypeToString : StepType -> String
stepTypeToString stepType =
    case stepType of
        Start ->
            "start"

        Approval ->
            "approval"

        End ->
            "end"


{-| StepType に応じたデフォルト名
-}
defaultStepName : StepType -> String
defaultStepName stepType =
    case stepType of
        Start ->
            "開始"

        Approval ->
            "承認"

        End ->
            "終了"


{-| ステップの表示色（fill と stroke の hex 値）

デザインガイドラインの色トークンに対応:

  - Start: success-100 (#d1fae5) / success-600 (#059669)
  - Approval: primary-100 (#e0e7ff) / primary-600 (#4f46e5)
  - End: secondary-100 (#f1f5f9) / secondary-600 (#475569)

-}
type alias StepColors =
    { fill : String, stroke : String }


stepColors : StepType -> StepColors
stepColors stepType =
    case stepType of
        Start ->
            { fill = "#d1fae5", stroke = "#059669" }

        Approval ->
            { fill = "#e0e7ff", stroke = "#4f46e5" }

        End ->
            { fill = "#f1f5f9", stroke = "#475569" }


{-| ステップノードの固定サイズ
-}
type alias Dimensions =
    { width : Float, height : Float }


stepDimensions : Dimensions
stepDimensions =
    { width = 120, height = 60 }


{-| マウスの clientX/clientY を SVG viewBox 座標に変換する

    canvasX =
        (clientX - bounds.x) / bounds.width * viewBoxWidth

    canvasY =
        (clientY - bounds.y) / bounds.height * viewBoxHeight

-}
clientToCanvas : Maybe Bounds -> Float -> Float -> Maybe Position
clientToCanvas maybeBounds clientX clientY =
    case maybeBounds of
        Just bounds ->
            Just
                { x = (clientX - bounds.x) / bounds.width * viewBoxWidth
                , y = (clientY - bounds.y) / bounds.height * viewBoxHeight
                }

        Nothing ->
            Nothing


{-| ステップ ID を生成する

"stepType\_番号" 形式。API 連携時に UUID に置き換え可能な設計。

-}
generateStepId : StepType -> Int -> String
generateStepId stepType number =
    stepTypeToString stepType ++ "_" ++ String.fromInt number


{-| パレットからのドロップでステップを生成する

ドロップ位置をグリッドにスナップして StepNode を作成する。

-}
createStepFromDrop : StepType -> Int -> Position -> StepNode
createStepFromDrop stepType stepNumber dropPosition =
    { id = generateStepId stepType stepNumber
    , stepType = stepType
    , name = defaultStepName stepType
    , position =
        { x = snapToGrid dropPosition.x
        , y = snapToGrid dropPosition.y
        }
    }


{-| JSON Value を Bounds にデコードする

Port 経由で受信した getBoundingClientRect の結果をデコードする。

-}
decodeBounds : Encode.Value -> Result Decode.Error Bounds
decodeBounds value =
    Decode.decodeValue boundsDecoder value


boundsDecoder : Decode.Decoder Bounds
boundsDecoder =
    Decode.map4 Bounds
        (Decode.field "x" Decode.float)
        (Decode.field "y" Decode.float)
        (Decode.field "width" Decode.float)
        (Decode.field "height" Decode.float)
