module Data.DesignerCanvas exposing
    ( Assignee
    , Bounds
    , Dimensions
    , DraggingState(..)
    , Position
    , ReconnectEnd(..)
    , StepColors
    , StepNode
    , StepType(..)
    , Transition
    , autoTrigger
    , clampToViewBox
    , clientToCanvas
    , createStepFromDrop
    , decodeBounds
    , defaultStepName
    , encodeDefinition
    , generateStepId
    , loadStepsFromDefinition
    , loadTransitionsFromDefinition
    , snapToGrid
    , stepColors
    , stepContainsPoint
    , stepDimensions
    , stepInputPortPosition
    , stepOutputPortPosition
    , stepTypeToString
    , triggerLabel
    , viewBoxHeight
    , viewBoxWidth
    )

{-| ワークフローデザイナー キャンバスのデータ型と純粋関数

ページモジュール（Designer.elm）からデータ型を分離することで、
テスタビリティを確保し、既存の Data/ パターンに準拠する。

-}

import Dict exposing (Dict)
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


{-| 承認者の指定方式

Phase 2-4 では `{ type_ = "user" }` のみ。
将来のグループ指定等に拡張可能な構造。

-}
type alias Assignee =
    { type_ : String }


{-| キャンバス上に配置されたステップノード
-}
type alias StepNode =
    { id : String
    , stepType : StepType
    , name : String
    , position : Position
    , assignee : Maybe Assignee
    , endStatus : Maybe String
    }


{-| ステップ間の遷移
-}
type alias Transition =
    { from : String
    , to : String
    , trigger : Maybe String
    }


{-| SVG 要素の境界情報（getBoundingClientRect の結果）
-}
type alias Bounds =
    { x : Float, y : Float, width : Float, height : Float }


{-| 接続線の付け替え対象の端点
-}
type ReconnectEnd
    = SourceEnd
    | TargetEnd


{-| ドラッグ操作の状態

  - DraggingExistingStep: 既存ステップの移動中（stepId, ステップ原点からのオフセット）
  - DraggingNewStep: パレットからの新規配置中（StepType, 現在のキャンバス座標）
  - DraggingConnection: 接続線作成中（接続元 stepId, 現在のマウス座標）
  - DraggingReconnection: 接続線端点の付け替え中（transition index, 変更する端点, 現在のマウス座標）

-}
type DraggingState
    = DraggingExistingStep String Position
    | DraggingNewStep StepType Position
    | DraggingConnection String Position
    | DraggingReconnection Int ReconnectEnd Position


{-| グリッドサイズ（px）
-}
gridSize : Float
gridSize =
    20


{-| SVG viewBox の幅
-}
viewBoxWidth : Float
viewBoxWidth =
    800


{-| SVG viewBox の高さ
-}
viewBoxHeight : Float
viewBoxHeight =
    600


{-| 座標をグリッドにスナップする

四捨五入で最も近いグリッド位置に合わせる。

-}
snapToGrid : Float -> Float
snapToGrid value =
    toFloat (round (value / gridSize)) * gridSize


{-| 座標を viewBox 内に制約する

ステップサイズを考慮し、ステップ全体が viewBox 内に収まるよう制限する。

-}
clampToViewBox : Position -> Position
clampToViewBox pos =
    { x = pos.x |> max 0 |> min (viewBoxWidth - stepDimensions.width)
    , y = pos.y |> max 0 |> min (viewBoxHeight - stepDimensions.height)
    }


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


{-| 文字列から StepType に変換
-}
stepTypeFromString : String -> Maybe StepType
stepTypeFromString str =
    case str of
        "start" ->
            Just Start

        "approval" ->
            Just Approval

        "end" ->
            Just End

        _ ->
            Nothing


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
    { width = 180, height = 90 }


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
    , assignee = Nothing
    , endStatus = Nothing
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



-- CONNECTION HELPERS


{-| ステップの出力ポート位置（右端中央）
-}
stepOutputPortPosition : StepNode -> Position
stepOutputPortPosition step =
    { x = step.position.x + stepDimensions.width
    , y = step.position.y + stepDimensions.height / 2
    }


{-| ステップの入力ポート位置（左端中央）
-}
stepInputPortPosition : StepNode -> Position
stepInputPortPosition step =
    { x = step.position.x
    , y = step.position.y + stepDimensions.height / 2
    }


{-| 座標がステップの矩形内に含まれるか判定する
-}
stepContainsPoint : Position -> StepNode -> Bool
stepContainsPoint point step =
    let
        right =
            step.position.x + stepDimensions.width

        bottom =
            step.position.y + stepDimensions.height
    in
    point.x >= step.position.x && point.x <= right && point.y >= step.position.y && point.y <= bottom


{-| 新しい接続に対する trigger を自動判定する

Approval ステップからの接続は approve/reject を自動設定する。

-}
autoTrigger : StepType -> String -> List Transition -> Maybe String
autoTrigger sourceType sourceId transitions =
    case sourceType of
        Approval ->
            let
                fromTransitions =
                    List.filter (\t -> t.from == sourceId) transitions

                hasApprove =
                    List.any (\t -> t.trigger == Just "approve") fromTransitions

                hasReject =
                    List.any (\t -> t.trigger == Just "reject") fromTransitions
            in
            if not hasApprove then
                Just "approve"

            else if not hasReject then
                Just "reject"

            else
                Nothing

        _ ->
            Nothing


{-| トリガー種別の表示ラベルを返す
-}
triggerLabel : Maybe String -> String
triggerLabel trigger =
    case trigger of
        Just "approve" ->
            "承認"

        Just "reject" ->
            "却下"

        Just other ->
            other

        Nothing ->
            "なし"



-- ENCODERS


{-| ステップと遷移からバックエンド API 用の定義 JSON を生成する

生成される JSON 構造:

    { "steps": [
        { "id": "...", "type": "start", "name": "...", "position": {...} },
        ...
      ],
      "transitions": [
        { "from": "...", "to": "...", "trigger": "approve" },
        ...
      ]
    }

-}
encodeDefinition : Dict String StepNode -> List Transition -> Encode.Value
encodeDefinition steps transitions =
    Encode.object
        [ ( "steps"
          , steps
                |> Dict.values
                |> Encode.list encodeStep
          )
        , ( "transitions", Encode.list encodeTransition transitions )
        ]


encodeStep : StepNode -> Encode.Value
encodeStep step =
    let
        baseFields =
            [ ( "id", Encode.string step.id )
            , ( "type", Encode.string (stepTypeToString step.stepType) )
            , ( "name", Encode.string step.name )
            , ( "position"
              , Encode.object
                    [ ( "x", Encode.float step.position.x )
                    , ( "y", Encode.float step.position.y )
                    ]
              )
            ]

        assigneeField =
            case step.assignee of
                Just assignee ->
                    [ ( "assignee"
                      , Encode.object [ ( "type", Encode.string assignee.type_ ) ]
                      )
                    ]

                Nothing ->
                    []

        endStatusField =
            case step.endStatus of
                Just status ->
                    [ ( "status", Encode.string status ) ]

                Nothing ->
                    []
    in
    Encode.object (baseFields ++ assigneeField ++ endStatusField)


encodeTransition : Transition -> Encode.Value
encodeTransition transition =
    let
        baseFields =
            [ ( "from", Encode.string transition.from )
            , ( "to", Encode.string transition.to )
            ]

        triggerField =
            case transition.trigger of
                Just trigger ->
                    [ ( "trigger", Encode.string trigger ) ]

                Nothing ->
                    []
    in
    Encode.object (baseFields ++ triggerField)



-- DECODERS


{-| 定義 JSON からステップを Dict として読み込む

position フィールドが省略されている場合は、縦一列の自動配置を適用する。

-}
loadStepsFromDefinition : Decode.Value -> Result Decode.Error (Dict String StepNode)
loadStepsFromDefinition value =
    Decode.decodeValue stepsDecoder value


stepsDecoder : Decode.Decoder (Dict String StepNode)
stepsDecoder =
    Decode.field "steps" (Decode.list stepDecoder)
        |> Decode.map autoLayoutIfNeeded
        |> Decode.map (\steps -> List.map (\s -> ( s.id, s )) steps |> Dict.fromList)


{-| position がないステップに自動配置を適用する

すべてのステップに position がある場合はそのまま返す。
1 つでも position がないステップがある場合は、全ステップに縦一列の自動配置を適用する。

-}
autoLayoutIfNeeded : List StepNode -> List StepNode
autoLayoutIfNeeded steps =
    let
        hasAnyPosition =
            List.any (\s -> s.position.x /= 0 || s.position.y /= 0) steps
    in
    if hasAnyPosition then
        steps

    else
        -- 縦一列、等間隔で自動配置
        let
            autoLayoutX =
                viewBoxWidth / 2 - stepDimensions.width / 2
        in
        List.indexedMap
            (\i step ->
                { step | position = { x = autoLayoutX, y = 60 + toFloat i * 150 } }
            )
            steps


stepDecoder : Decode.Decoder StepNode
stepDecoder =
    Decode.map6 StepNode
        (Decode.field "id" Decode.string)
        (Decode.field "type" Decode.string |> Decode.andThen stepTypeDecoder)
        (Decode.field "name" Decode.string)
        (Decode.oneOf
            [ Decode.field "position" positionDecoder
            , Decode.succeed { x = 0, y = 0 }
            ]
        )
        (Decode.maybe (Decode.field "assignee" assigneeDecoder))
        (Decode.maybe (Decode.field "status" Decode.string))


stepTypeDecoder : String -> Decode.Decoder StepType
stepTypeDecoder str =
    case stepTypeFromString str of
        Just st ->
            Decode.succeed st

        Nothing ->
            Decode.fail ("Unknown step type: " ++ str)


positionDecoder : Decode.Decoder Position
positionDecoder =
    Decode.map2 Position
        (Decode.field "x" Decode.float)
        (Decode.field "y" Decode.float)


assigneeDecoder : Decode.Decoder Assignee
assigneeDecoder =
    Decode.map Assignee
        (Decode.field "type" Decode.string)


{-| 定義 JSON から遷移のリストを読み込む
-}
loadTransitionsFromDefinition : Decode.Value -> Result Decode.Error (List Transition)
loadTransitionsFromDefinition value =
    Decode.decodeValue transitionsDecoder value


transitionsDecoder : Decode.Decoder (List Transition)
transitionsDecoder =
    Decode.field "transitions" (Decode.list transitionDecoder)


transitionDecoder : Decode.Decoder Transition
transitionDecoder =
    Decode.map3 Transition
        (Decode.field "from" Decode.string)
        (Decode.field "to" Decode.string)
        (Decode.maybe (Decode.field "trigger" Decode.string))
