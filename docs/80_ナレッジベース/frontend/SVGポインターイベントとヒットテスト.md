# SVG ポインターイベントとヒットテスト

## 概要

SVG 要素のクリック判定（ヒットテスト）は HTML 要素とは異なる仕組みで動作する。特に透明な要素や座標変換が絡む場合、意図しない挙動が発生しやすい。このドキュメントでは、SVG でのポインターイベント制御と座標変換について整理する。

## pointer-events 属性

SVG の `pointer-events` 属性は、要素がポインターイベントのターゲットになる条件を制御する。

### 主な値

| 値 | 挙動 |
|----|------|
| `visiblePainted`（デフォルト） | 可視かつ fill/stroke が塗られている部分のみ |
| `visible` | 可視であれば fill/stroke の塗りに関係なく反応 |
| `painted` | 可視性に関係なく、fill/stroke が塗られている部分のみ |
| `fill` | fill 領域のみ（塗りの有無に関係なく） |
| `stroke` | stroke 領域のみ（塗りの有無に関係なく） |
| `all` | fill/stroke の塗り・可視性に関係なく、幾何学的な範囲で反応 |
| `none` | ポインターイベントを受け取らない |

### `stroke` と `all` の違い

`pointer-events: stroke` は仕様上「stroke の幾何学的な範囲」で反応するはずだが、ブラウザの実装によっては `stroke="transparent"` で期待通り動作しない場合がある。確実にクリック判定を受けたい場合は `pointer-events: all` を使用する。

## 透明要素とヒットテスト

### `opacity="0"` vs `stroke="transparent"`

| 方法 | ヒットテスト | 用途 |
|------|------------|------|
| `opacity="0"` | 除外される | 要素を完全に非表示にしたい場合 |
| `stroke="transparent"` / `fill="transparent"` | 対象になる | 不可視だがクリック可能にしたい場合 |
| `visibility="hidden"` | 除外される（`pointer-events` が `visible*` 系の場合） | 要素を非表示にしたい場合 |

`opacity="0"` はブラウザの描画パイプラインから要素を除外するため、`pointer-events: all` を設定しても `document.elementsFromPoint()` で検出されない。

### 推奨パターン: 透明クリック領域

```elm
-- 不可視だがクリック可能な太いパス（接続線のクリック判定用）
Svg.path
    [ SvgAttr.d pathData
    , SvgAttr.fill "none"
    , SvgAttr.stroke "transparent"
    , SvgAttr.strokeWidth "12"
    , SvgAttr.pointerEvents "all"
    ]
    []
```

## SVG 座標変換

### viewBox と実際のサイズ

SVG 要素は `viewBox` と実際の描画サイズ（CSS サイズ）が異なる場合がある。`preserveAspectRatio` の設定によって、viewBox の座標系が実際のピクセル座標にどうマッピングされるかが決まる。

```
viewBox="0 0 800 600"  +  実際のサイズ 528x492px
+ preserveAspectRatio="xMidYMid meet"
→ スケール: 0.66（528/800 = 492/600 ではない場合、小さい方に合わせる）
→ Y 軸オフセット: (492 - 600*0.66) / 2 = 16px（中央寄せ）
```

### getScreenCTM() による正確な座標変換

viewBox 座標からスクリーン座標への変換は `getScreenCTM()` を使う。

```javascript
const svg = document.querySelector('svg');
const ctm = svg.getScreenCTM();
// ctm = { a: scaleX, b: 0, c: 0, d: scaleY, e: translateX, f: translateY }

// viewBox 座標 (vx, vy) → スクリーン座標 (sx, sy)
const sx = ctm.a * vx + ctm.e;
const sy = ctm.d * vy + ctm.f;
```

手動計算（`bounds.x + vx/viewBoxWidth * width`）は `preserveAspectRatio` のオフセットを考慮できないため不正確。

### Elm での座標変換

Elm のワークフローデザイナーでは、`clientToCanvas` 関数でスクリーン座標を viewBox 座標に変換している。この変換は SVG 要素の `getBoundingClientRect()` と viewBox のサイズ比を使用する。

## z-order（描画順序）

SVG には CSS の `z-index` に相当する機能がない。描画順序は DOM の出現順序で決まる（後に出現する要素が前面に描画される）。

### レイヤー構成の例

```elm
Svg.svg []
    [ viewBackground      -- 最背面
    , viewGrid
    , viewTransitions      -- 接続線
    , viewSteps            -- ステップノード
    , viewHandleLayer      -- ドラッグハンドル（最前面）
    ]
```

インタラクティブな小さい要素（ドラッグハンドル等）は、大きな要素（ステップノード等）の下に隠れないよう、後のレイヤーに配置する必要がある。

## プロジェクトでの使用箇所

| ファイル | 用途 |
|---------|------|
| `frontend/src/Page/WorkflowDefinition/Designer.elm` | ワークフローデザイナーの SVG キャンバス |

## 関連リソース

- [SVG pointer-events - MDN](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/pointer-events)
- [SVGGraphicsElement.getScreenCTM() - MDN](https://developer.mozilla.org/en-US/docs/Web/API/SVGGraphicsElement/getScreenCTM)
- [preserveAspectRatio - MDN](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/preserveAspectRatio)

## 関連ドキュメント

- 調査記録: [SVG 接続線クリックのヒットテスト失敗](../../../process/investigations/2026-02/2026-02-24_2154_SVG接続線クリックのヒットテスト失敗.md)
- セッションログ: [SVG クリック判定とハンドル表示改善](../../../prompts/runs/2026-02/2026-02-24_2154_SVGクリック判定とハンドル表示改善.md)
