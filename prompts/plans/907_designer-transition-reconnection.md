# #907 接続線の始点・終点をドラッグで付け替え可能にする

## Context

現在のワークフローデザイナーでは、接続線（Transition）の作成後に始点や終点を変更する手段がない。接続先を変えたい場合は「削除して作り直す」必要があり、draw.io 等のツールと比べて操作性が劣る。

この変更により、選択中の接続線の端点をドラッグして別のステップに付け替える機能を追加する。

## 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Data/DesignerCanvas.elm` | `ReconnectEnd` 型、`DraggingReconnection` バリアント追加 |
| `frontend/src/Page/WorkflowDefinition/Designer.elm` | Msg 追加、update ロジック、view（ドラッグハンドル + プレビュー） |
| `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm` | 再接続ロジックのテスト |
| `docs/03_詳細設計書/15_ワークフローデザイナー設計.md` | WFD-003 セクション更新 |

## 対象外

- バックエンド変更（フロントエンドのみの操作変更、データ構造は変わらない）
- Transition のプロパティパネル（#906 の範囲）
- 新しい trigger の自動判定ロジック変更（既存 trigger を維持する方針）

## 設計判断

### 1. DraggingState の拡張方法

選択肢:
- A: `DraggingReconnection Int ReconnectEnd Position`（1 バリアント + ReconnectEnd 型）
- B: `DraggingReconnectFrom Int Position | DraggingReconnectTo Int Position`（2 バリアント）

採用: A

理由: 両方の端点ドラッグは同じ構造（Transition index + Position）を持ち、処理の差は from/to の更新先のみ。`ReconnectEnd` を使ってパラメータ化することで、CanvasMouseMove の処理を共通化でき、コードの重複を避けられる。

### 2. ドラッグハンドルの表示条件

表示: 接続線が選択中（`selectedTransitionIndex == Just index`）かつ dragging 中でないときのみ。

理由: 常時表示するとキャンバスが煩雑になる。draw.io 等のツールと同じ UX パターン。

### 3. trigger の維持

端点付け替え時、既存の trigger（approve/reject/Nothing）をそのまま維持する。`autoTrigger` は呼ばない。

理由: Issue の完了基準「付け替え後、トリガー（承認/却下）は維持される」に合致。ユーザーが意図的に端点を変えたのに trigger が自動変更されるのは予期しない動作。

### 4. リスト更新ヘルパー

既存の `removeAt` パターンに合わせて `updateAt` ヘルパーを Designer.elm に追加する（`List.Extra.setAt` を使わない）。

```elm
updateAt : Int -> (a -> a) -> List a -> List a
updateAt index fn list =
    List.indexedMap
        (\i item ->
            if i == index then fn item else item
        )
        list
```

理由: 既存の `removeAt` が `List.Extra` を使わず自前実装しているため、同じスタイルを維持。

## 型定義

### Data/DesignerCanvas.elm に追加

```elm
{-| 接続線の付け替え対象の端点 -}
type ReconnectEnd
    = SourceEnd  -- 始点（from）を変更
    | TargetEnd  -- 終点（to）を変更

type DraggingState
    = DraggingExistingStep String Position
    | DraggingNewStep StepType Position
    | DraggingConnection String Position
    | DraggingReconnection Int ReconnectEnd Position  -- ★追加
```

### Designer.elm に追加

```elm
type Msg
    = ...
    | TransitionEndpointMouseDown Int ReconnectEnd Float Float  -- ★追加
```

## Phase 1: Types + update ロジック

### 確認事項
- 型: `DraggingState`, `Transition`, `StepNode` → `Data/DesignerCanvas.elm`
- パターン: `CanvasMouseUp` での DraggingConnection 処理 → `Designer.elm:328-368`
- パターン: `removeAt` ヘルパー → `Designer.elm:757-759`
- パターン: テストヘルパー `canvasWithOneStep`, `expectLoaded` → `DesignerTest.elm:100-177`

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 選択中の接続線の始点ハンドルをドラッグし、別のステップにドロップ → from が更新される | 正常系 | ユニット |
| 2 | 選択中の接続線の終点ハンドルをドラッグし、別のステップにドロップ → to が更新される | 正常系 | ユニット |
| 3 | ドラッグ中にマウスを動かす → DraggingReconnection の Position が更新される | 正常系 | ユニット |
| 4 | 端点ハンドルの mousedown → DraggingReconnection に遷移する | 正常系 | ユニット |
| 5 | 無効なドロップ先（空白領域）にドロップ → 元の接続が維持される | 準正常系 | ユニット |
| 6 | 自分自身（反対端のステップ）にドロップ → 元の接続が維持される | 準正常系 | ユニット |
| 7 | 付け替え後、trigger（承認/却下）が維持される | 正常系 | ユニット |

### テストリスト

ユニットテスト:
- [ ] TransitionEndpointMouseDown で DraggingReconnection に遷移する（SourceEnd）
- [ ] TransitionEndpointMouseDown で DraggingReconnection に遷移する（TargetEnd）
- [ ] CanvasMouseMove で DraggingReconnection の Position が更新される
- [ ] CanvasMouseUp（SourceEnd）で有効なステップにドロップ → from が更新される
- [ ] CanvasMouseUp（TargetEnd）で有効なステップにドロップ → to が更新される
- [ ] CanvasMouseUp で空白領域にドロップ → transitions が変更されない
- [ ] CanvasMouseUp で反対端のステップにドロップ → transitions が変更されない（自己ループ防止）
- [ ] 付け替え後、trigger が元の値を維持する
- [ ] 付け替え成功時に isDirty がマークされる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — ドラッグ操作の E2E テストは Playwright での座標制御が複雑。手動確認で代替）

### 実装内容

#### 1a. DesignerCanvas.elm

- `ReconnectEnd` 型を追加（exposing に追加）
- `DraggingReconnection Int ReconnectEnd Position` バリアントを追加
- doc comment を更新

#### 1b. Designer.elm

- `TransitionEndpointMouseDown Int ReconnectEnd Float Float` Msg を追加
- `updateAt` ヘルパー関数を追加
- `updateLoaded` に以下を追加:
  - `TransitionEndpointMouseDown` ハンドラ: `clientToCanvas` で座標変換 → `DraggingReconnection` 設定
  - `CanvasMouseMove` の `DraggingReconnection` ケース: Position 更新
  - `CanvasMouseUp` の `DraggingReconnection` ケース:
    1. `mousePos` 内のステップを `stepContainsPoint` で判定
    2. 自己ループチェック（付け替え後の from == to を防止）
    3. 有効なら `updateAt` で transition の from/to を更新（trigger は維持）
    4. 無効なら `dragging = Nothing` のみ（元の接続を維持）
    5. `DirtyState.markDirty` 呼び出し

## Phase 2: View（ドラッグハンドル + プレビュー）

### 確認事項
- パターン: `viewTransitionLine` での SVG 描画 → `Designer.elm:1294-1393`
- パターン: `viewConnectionDragPreview` でのプレビュー描画 → `Designer.elm:1401-1447`
- パターン: 出力ポートの `mousedown` イベント登録 → `Designer.elm:1206-1219`

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 接続線を選択 → 始点・終点にドラッグハンドルが表示される | 正常系 | 手動確認 |
| 2 | ドラッグハンドルからドラッグ開始 → プレビュー破線が表示される | 正常系 | 手動確認 |
| 3 | ドラッグ中にマウスを動かす → プレビュー線が追従する | 正常系 | 手動確認 |

### テストリスト

ユニットテスト（該当なし — view のレンダリングは Elm のユニットテストでは検証困難）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 実装内容

#### 2a. viewTransitionLine の拡張

`viewTransitionLine` 内で、`isSelected` が `True` のとき、始点・終点にドラッグハンドル（小さな円）を追加する。

```elm
-- 選択中の接続線にドラッグハンドルを表示
, if isSelected then
    Svg.g []
        [ -- 始点ハンドル
          Svg.circle
            [ SvgAttr.cx (String.fromFloat startPos.x)
            , SvgAttr.cy (String.fromFloat startPos.y)
            , SvgAttr.r "6"
            , SvgAttr.fill "white"
            , SvgAttr.stroke "#6366f1"
            , SvgAttr.strokeWidth "2"
            , SvgAttr.class "cursor-grab"
            , Html.Events.stopPropagationOn "mousedown"
                (Decode.map2
                    (\cx cy -> ( TransitionEndpointMouseDown index SourceEnd cx cy, True ))
                    (Decode.field "clientX" Decode.float)
                    (Decode.field "clientY" Decode.float)
                )
            ]
            []
        -- 終点ハンドル
        , Svg.circle
            [ SvgAttr.cx (String.fromFloat endPos.x)
            , SvgAttr.cy (String.fromFloat endPos.y)
            , SvgAttr.r "6"
            , SvgAttr.fill "white"
            , SvgAttr.stroke "#6366f1"
            , SvgAttr.strokeWidth "2"
            , SvgAttr.class "cursor-grab"
            , Html.Events.stopPropagationOn "mousedown"
                (Decode.map2
                    (\cx cy -> ( TransitionEndpointMouseDown index TargetEnd cx cy, True ))
                    (Decode.field "clientX" Decode.float)
                    (Decode.field "clientY" Decode.float)
                )
            ]
            []
        ]
  else
    Svg.text ""
```

ハンドルのスタイル:
- 白丸 + indigo ストローク（選択ハイライトと同じ色 `#6366f1`）
- 半径 6px（ステップポートの 7px より少し小さい）
- `cursor-grab` で掴めることを視覚的に示す

#### 2b. viewConnectionDragPreview の拡張

`viewConnectionDragPreview` に `DraggingReconnection` ケースを追加。

SourceEnd の場合: mousePos → 固定端（to ステップの入力ポート）
TargetEnd の場合: 固定端（from ステップの出力ポート）→ mousePos

プレビュー線のスタイルは既存の `DraggingConnection` と同じ（グレー破線 + 矢印）。

#### 2c. import の更新

`Data.DesignerCanvas` の import に `ReconnectEnd(..)` を追加（Designer.elm とテストの両方）。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | 自己ループ防止の検証が未定義 | 競合・エッジケース | CanvasMouseUp で「付け替え後の from == to」チェックを追加。テストリストにも追加 |
| 2回目 | DraggingReconnection 中に viewTransitionLine が元の接続線を表示し続ける問題 | 不完全なパス | DraggingReconnection 中は対象 index の接続線を非表示にしてプレビュー線のみ表示する（既存接続とプレビューが重なるのを防止） |
| 3回目 | SourceEnd 変更時の ReconnectEnd 型設計で DesignerCanvas.elm の exposing リストに追加が必要 | 未定義 | exposing リストに `ReconnectEnd(..)` を追加する旨を明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 完了基準 4 項目が Phase 構成に含まれている | OK | 端点付け替え → Phase 1, プレビュー線 → Phase 2, trigger 維持 → Phase 1 テスト, 無効ドロップ → Phase 1 テスト |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 型定義・コードスニペットで挙動が一意に確定 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | DraggingState 拡張方法、trigger 維持方針、ハンドル表示条件、ヘルパー方式の 4 判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象外セクションに明記済み |
| 5 | 技術的前提 | 前提が考慮されている | OK | SVG イベント伝播（stopPropagationOn）、座標変換（clientToCanvas）、既存 `removeAt` パターンを確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-053（SVG + Elm 直接レンダリング）、ADR-054（型安全ステートマシン）と整合 |

## 検証方法

1. `just check`（lint + テスト）で Phase 1 のユニットテスト通過を確認
2. `just dev-all` で開発サーバー起動 → ワークフロー定義画面で手動確認:
   - 接続線を選択 → 端点にドラッグハンドルが表示される
   - ハンドルをドラッグ → 破線プレビューが表示される
   - 別のステップにドロップ → 接続先が変わる、trigger は維持
   - 空白領域にドロップ → 元の接続が維持される
3. `just check-all` で全テスト通過を確認
