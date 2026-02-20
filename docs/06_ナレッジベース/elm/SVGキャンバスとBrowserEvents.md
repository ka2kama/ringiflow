# SVG キャンバスと Browser.Events

## 概要

Elm で SVG ベースのインタラクティブなキャンバス（ドラッグ&ドロップ、選択、キーボード操作）を構築するためのパターン。Browser.Events モジュールによるグローバルイベント購読と、SVG viewBox 座標系の扱いを中心に整理する。

## Browser.Events

### 基本

`Browser.Events` は `elm/browser` パッケージに含まれ、`document` レベルのイベントをサブスクリプションとして購読する。

```elm
import Browser.Events

subscriptions : Model -> Sub Msg
subscriptions model =
    Browser.Events.onMouseMove
        (Decode.map2 MouseMoved
            (Decode.field "clientX" Decode.float)
            (Decode.field "clientY" Decode.float)
        )
```

### 条件付き subscription

ドラッグ操作のように「特定の状態でのみ」イベントを受け取りたい場合、Model の状態に応じて subscription を切り替える。

```elm
subscriptions : Model -> Sub Msg
subscriptions model =
    if model.dragging /= Nothing then
        Sub.batch
            [ Browser.Events.onMouseMove mouseMoveDecoder
            , Browser.Events.onMouseUp (Decode.succeed MouseUp)
            ]
    else
        Sub.none
```

subscription が `Model -> Sub Msg` 型になる点に注意。Main.elm の subscriptions ルーティングで `Model` を渡す必要がある:

```elm
-- Main.elm
subscriptions model =
    case model.page of
        DesignerPage subModel ->
            Sub.map DesignerMsg (Designer.subscriptions subModel)
```

### キーボードイベント

```elm
Browser.Events.onKeyDown
    (Decode.field "key" Decode.string
        |> Decode.map KeyDown
    )
```

`key` フィールドは `"Delete"`, `"Backspace"`, `"Escape"` 等の文字列を返す。

## SVG viewBox 座標系

### viewBox とレスポンシブスケーリング

```elm
svg
    [ SvgAttr.viewBox "0 0 1200 800"
    , SvgAttr.width "100%"
    , SvgAttr.height "100%"
    ]
```

viewBox は SVG 内部の座標系を定義する。`width="100%"` と組み合わせることで、表示サイズに関わらず内部座標が一定に保たれる。

### マウス座標 → SVG 座標への変換

ブラウザのマウスイベントは `clientX/clientY`（ビューポート座標）を返すが、SVG 要素内の座標とは異なる。変換には SVG 要素の `getBoundingClientRect()` が必要。

```
canvasX = (clientX - bounds.x) / bounds.width * viewBoxWidth
canvasY = (clientY - bounds.y) / bounds.height * viewBoxHeight
```

Elm は直接 `getBoundingClientRect()` を呼べないため、Ports 経由で取得する:

```elm
-- Ports.elm
port requestCanvasBounds : String -> Cmd msg
port receiveCanvasBounds : (Encode.Value -> msg) -> Sub msg
```

```javascript
// main.js
app.ports.requestCanvasBounds.subscribe((elementId) => {
    requestAnimationFrame(() => {
        const el = document.getElementById(elementId);
        if (el) {
            const rect = el.getBoundingClientRect();
            app.ports.receiveCanvasBounds.send({
                x: rect.x, y: rect.y,
                width: rect.width, height: rect.height
            });
        }
    });
});
```

`requestAnimationFrame` を使う理由: Elm の Virtual DOM 更新が完了してから DOM 情報を取得するため。

## SVG レイヤー順序

SVG は後に記述した要素が前面に描画される（z-index なし）。インタラクティブなキャンバスでは描画順序がイベント伝播に影響する。

```elm
svg []
    [ viewCanvasBackground  -- 最背面: 背景クリック検出用
    , viewGrid              -- グリッド線
    , viewSteps             -- ステップノード
    , viewDragPreview       -- 最前面: ドラッグ中のプレビュー
    ]
```

注意点:
- 背景の透明レイヤー（`fill "transparent"`）を最背面に配置し、空白クリックを検出する
- グリッド線には `pointerEvents "none"` を設定し、クリック判定から除外する
- ドラッグプレビューには `pointerEvents "none"` を設定し、下のステップへのクリックを妨げない

## ドラッグ&ドロップ状態機械

ドラッグ操作を代数的データ型で表現する:

```elm
type DraggingState
    = DraggingNewStep StepType Position     -- パレットから新規配置
    | DraggingExistingStep String Position  -- 既存ステップの移動（stepId, offset）
```

状態遷移:
1. `PaletteMouseDown` → `DraggingNewStep`（開始）
2. `StepMouseDown` → `DraggingExistingStep`（開始、offset を計算）
3. `CanvasMouseMove` → 位置更新（DraggingState に応じて分岐）
4. `CanvasMouseUp` → ドロップ確定 → `dragging = Nothing`

### グリッドスナップ

```elm
gridSize : Float
gridSize = 20

snapToGrid : Float -> Float
snapToGrid value =
    toFloat (round (value / gridSize)) * gridSize
```

## SVG 要素のイベント

SVG 要素にイベントを設定する場合、`Html.Events.stopPropagationOn` を使ってイベントバブリングを制御する:

```elm
Svg.g
    [ Html.Events.stopPropagationOn "mousedown"
        (Decode.map2 (\cx cy -> ( StepMouseDown stepId cx cy, True ))
            (Decode.field "clientX" Decode.float)
            (Decode.field "clientY" Decode.float)
        )
    ]
```

`stopPropagationOn` は `(msg, Bool)` タプルを返し、`True` でイベント伝播を停止する。ステップのクリックが背景のクリックハンドラに伝播するのを防ぐ。

## プロジェクトでの使用箇所

- `frontend/src/Page/WorkflowDefinition/Designer.elm` — ワークフローデザイナー画面
- `frontend/src/Data/DesignerCanvas.elm` — キャンバスの型定義と純粋関数
- `frontend/src/Ports.elm` — Canvas Bounds の Ports 定義
- `frontend/src/main.js` — getBoundingClientRect の Port ハンドラ

## 関連リソース

- [Elm SVG パッケージ](https://package.elm-lang.org/packages/elm/svg/latest/)
- [Browser.Events](https://package.elm-lang.org/packages/elm/browser/latest/Browser-Events)
- [MDN: SVG viewBox](https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/viewBox)
- [MDN: getBoundingClientRect](https://developer.mozilla.org/en-US/docs/Web/API/Element/getBoundingClientRect)
- [ADR-053: ワークフローデザイナー技術選定](../../05_ADR/053_ワークフローデザイナー技術選定.md)
