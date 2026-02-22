# 計画: #787 ワークフローデザイナーのノード削除 UI とドラッグ境界制約

## Context

ワークフローデザイナーに 2 つの UX 問題がある:
1. Delete/Backspace キーによるステップ削除は実装済みだが、UI にボタンやヒントがなくユーザーが機能を発見できない
2. ノードをドラッグすると viewBox (800×600) 外に移動でき、左パネルや右パネルの下に潜り込んで操作不能になる

## 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Data/DesignerCanvas.elm` | `clampToViewBox` 関数を追加 |
| `frontend/src/Page/WorkflowDefinition/Designer.elm` | 削除ボタン UI + 境界制約の適用 |
| `frontend/tests/Data/DesignerCanvasTest.elm` | `clampToViewBox` のテスト |
| `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm` | 削除 Msg + 境界制約のテスト |

対象外: バックエンド変更、OpenAPI 変更、新規コンポーネント作成

## 設計判断

### 1. ConfirmDialog を使用しない

ステップ削除に ConfirmDialog を使わない。

理由:
- keyboard Delete が既に確認なしで削除を実行しており、ボタンだけ確認ダイアログを出すと操作の一貫性が崩れる
- デザイナーのキャンバス操作は「保存」するまでデータベースに反映されない。誤操作はリロードで復元可能
- ビジュアルエディタ（Figma 等）の UX 慣行として、要素削除に確認ダイアログは出さない

品質チェックリストの「破壊的操作の防御」は、Button.Error バリアント（赤色）による視覚的警告 + キーボードショートカットヒントで対応する。

### 2. 削除ロジックの共通化

`KeyDown "Delete"` ハンドラと新しい `DeleteSelectedStep` ボタンで同じ削除ロジックを使う。ヘルパー関数 `deleteSelectedStep` を抽出し、重複を排除する。

### 3. clampToViewBox の適用箇所

`clampToViewBox` を `DesignerCanvas.elm` に純粋関数として追加し、`Designer.elm` の以下の箇所で適用:
- `DraggingExistingStep` ハンドラ（既存ステップのドラッグ中）
- `CanvasMouseUp` の `DraggingNewStep`（新規ドロップ時）
- `viewDragPreview`（ドラッグプレビュー表示。制約後の位置をプレビューに反映）

制約範囲: ステップサイズ (180×90) を考慮し、x ∈ [0, 620], y ∈ [0, 510]

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: DesignerCanvas に clampToViewBox を追加

#### 確認事項
- 型: `Position`, `Dimensions` → `frontend/src/Data/DesignerCanvas.elm` (L52-55, L214-215)
- パターン: `snapToGrid` のテストパターン → `frontend/tests/Data/DesignerCanvasTest.elm`
- ライブラリ: Elm `min`/`max` → 標準ライブラリの Basics

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ユーザーが既存ステップを viewBox 内でドラッグする（位置変わらず） | 正常系 | ユニット |
| 2 | ユーザーが既存ステップを viewBox 外にドラッグする（制約される） | 準正常系 | ユニット |
| 3 | ユーザーがパレットから viewBox 外にドロップする（制約される） | 準正常系 | ユニット |

#### テストリスト

ユニットテスト:
- [ ] viewBox 内の座標はそのまま返す
- [ ] 右端を超える x は 620 (viewBoxWidth - stepWidth) に制約される
- [ ] 下端を超える y は 510 (viewBoxHeight - stepHeight) に制約される
- [ ] 負の x は 0 に制約される
- [ ] 負の y は 0 に制約される

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

#### 実装内容

`DesignerCanvas.elm` に追加:

```elm
clampToViewBox : Position -> Position
clampToViewBox pos =
    { x = pos.x |> max 0 |> min (viewBoxWidth - stepDimensions.width)
    , y = pos.y |> max 0 |> min (viewBoxHeight - stepDimensions.height)
    }
```

expose リストに `clampToViewBox` を追加。

### Phase 2: Designer.elm に境界制約を適用

#### 確認事項
- パターン: `DraggingExistingStep` ハンドラ → `Designer.elm:180-196`
- パターン: `CanvasMouseUp` の `DraggingNewStep` → `Designer.elm:217-232`
- パターン: `viewDragPreview` → `Designer.elm:1373-1424`
- パターン: DesignerTest の既存テスト → `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm`

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ユーザーが既存ステップを viewBox 境界付近にドラッグ → 制約される | 準正常系 | ユニット（DesignerTest） |
| 2 | ユーザーがパレットからステップを viewBox 外にドロップ → 制約される | 準正常系 | ユニット（DesignerTest） |

#### テストリスト

ユニットテスト:
- [ ] DraggingExistingStep でドラッグ中の位置が viewBox 内に制約される
- [ ] DraggingNewStep のドロップ位置が viewBox 内に制約される

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

#### 実装内容

`DraggingExistingStep` ハンドラ (L183-187):
```elm
newPos =
    { x = DesignerCanvas.snapToGrid (canvasPos.x - offset.x)
    , y = DesignerCanvas.snapToGrid (canvasPos.y - offset.y)
    }
    |> DesignerCanvas.clampToViewBox
```

`CanvasMouseUp` の `DraggingNewStep`: `createStepFromDrop` の結果に clamp を適用:
```elm
newStep =
    DesignerCanvas.createStepFromDrop stepType model.nextStepNumber dropPos
        |> (\s -> { s | position = DesignerCanvas.clampToViewBox s.position })
```

`viewDragPreview`: snap 後に clamp を適用:
```elm
clampedPos =
    DesignerCanvas.clampToViewBox
        { x = DesignerCanvas.snapToGrid pos.x
        , y = DesignerCanvas.snapToGrid pos.y
        }
```

### Phase 3: プロパティパネルに削除ボタンを追加

#### 確認事項
- 型: `Button.Variant` → `Button.Error` が存在することを確認（Component/Button.elm）
- パターン: 他ページの削除ボタン → `Page/Role/List.elm` 等で `Button.Error` 使用
- パターン: `KeyDown "Delete"` の削除ロジック → `Designer.elm:622-656`

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ユーザーがステップを選択し、プロパティパネルの削除ボタンをクリック → ステップと関連接続線が削除される | 正常系 | ユニット（DesignerTest） |
| 2 | ステップが未選択のとき、削除ボタンが表示されない | 正常系 | ユニット（DesignerTest） |

#### テストリスト

ユニットテスト:
- [ ] DeleteSelectedStep で選択中のステップが削除される
- [ ] DeleteSelectedStep で関連する接続線も削除される
- [ ] ステップ未選択時に DeleteSelectedStep は何もしない

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

#### 実装内容

1. `Msg` 型に `DeleteSelectedStep` を追加

2. ヘルパー関数を抽出:
```elm
deleteSelectedStep : Model -> ( Model, Cmd Msg )
deleteSelectedStep model =
    case model.selectedStepId of
        Just stepId ->
            let
                ( dirtyModel, dirtyCmd ) = DirtyState.markDirty model
            in
            ( { dirtyModel
                | steps = Dict.remove stepId dirtyModel.steps
                , transitions = List.filter (\t -> t.from /= stepId && t.to /= stepId) dirtyModel.transitions
                , selectedStepId = Nothing
              }
            , dirtyCmd
            )
        Nothing ->
            ( model, Cmd.none )
```

3. `KeyDown "Delete"` のステップ削除部分をヘルパーに委譲

4. `viewStepProperties` に削除ボタンを追加（フィールドの下に配置）:
```elm
, div [ class "mt-6 border-t border-secondary-200 pt-4" ]
    [ Button.view
        { variant = Button.Error
        , disabled = False
        , onClick = DeleteSelectedStep
        }
        [ text "ステップを削除" ]
    , p [ class "mt-1 text-xs text-secondary-400" ]
        [ text "Delete キーでも削除できます" ]
    ]
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | viewDragPreview にも clampToViewBox を適用しないとプレビュー位置とドロップ位置がずれる | 不完全なパス | Phase 2 に viewDragPreview の修正を追加 |
| 1回目 | ConfirmDialog の要否が未決定 | 曖昧 | 設計判断セクションに理由付きで「使用しない」と決定 |
| 1回目 | KeyDown ハンドラと新 Msg の削除ロジック重複 | 既存手段の見落とし | ヘルパー関数 `deleteSelectedStep` を抽出して共通化 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 完了基準 4 項目すべてに対応: 削除ボタン(Phase3), 関連接続線削除(Phase3), ドラッグ制約(Phase1+2), ドロップ制約(Phase1+2) |
| 2 | 曖昧さ排除 | OK | ConfirmDialog の要否、clampToViewBox の適用箇所、制約範囲を明示 |
| 3 | 設計判断の完結性 | OK | ConfirmDialog 不使用、ロジック共通化の判断を理由付きで記載 |
| 4 | スコープ境界 | OK | 対象: フロントエンド 4 ファイル、対象外: バックエンド・OpenAPI |
| 5 | 技術的前提 | OK | Elm の `min`/`max` は Basics モジュール標準。Button.Error は既存バリアント |
| 6 | 既存ドキュメント整合 | OK | デザインガイドライン準拠（Error バリアント = 赤）、既存パターン踏襲 |
