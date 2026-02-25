# #913 ワークフローデザイナー: ノードの接続ポイントを上下配置に変更する

## Context

ワークフローデザイナーのオートレイアウトはノードを縦方向（上→下）に配置するが、接続ポイントは左右（水平）に配置されている。このミスマッチにより遷移線が斜めに走り、フローの方向性が視覚的に分かりづらい。ポート位置を上下配置に変更し、遷移線がフロー方向に沿って流れるようにする。

## 変更対象

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Data/DesignerCanvas.elm` | ポート位置計算（2関数） |
| `frontend/tests/Data/DesignerCanvasTest.elm` | ポート位置テスト期待値（2テスト） |
| `frontend/src/Page/WorkflowDefinition/Designer.elm` | SVG ポート描画位置 + ベジェ曲線制御点（3箇所） |

## 対象外

- オートレイアウトのアルゴリズム自体の変更
- ステップのドラッグ移動ロジック（ポート位置関数を参照しているため自動追従する）
- E2E テスト（既存テストはステップ位置の座標を検証するもので、ポート位置の描画は検証対象外）

## Phase 1: ポート位置関数の変更（TDD）

### 確認事項

- 型: `StepNode`, `Position`, `stepDimensions` → `frontend/src/Data/DesignerCanvas.elm`（確認済み: width=180, height=90）
- パターン: 既存テストのパターン → `frontend/tests/Data/DesignerCanvasTest.elm:604-648`

### 操作パス

該当なし（ドメインロジックのみ）

### テストリスト

ユニットテスト:
- [ ] `stepOutputPortPosition`: ステップ下端中央の座標を返す — position (100, 200) → (190, 290)
  - 計算: `(100 + 180/2, 200 + 90)` = `(190, 290)`
- [ ] `stepInputPortPosition`: ステップ上端中央の座標を返す — position (300, 200) → (390, 200)
  - 計算: `(300 + 180/2, 200)` = `(390, 200)`

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 実装

`frontend/src/Data/DesignerCanvas.elm`:

```elm
-- Before
stepOutputPortPosition step =
    { x = step.position.x + stepDimensions.width
    , y = step.position.y + stepDimensions.height / 2
    }

-- After
stepOutputPortPosition step =
    { x = step.position.x + stepDimensions.width / 2
    , y = step.position.y + stepDimensions.height
    }
```

```elm
-- Before
stepInputPortPosition step =
    { x = step.position.x
    , y = step.position.y + stepDimensions.height / 2
    }

-- After
stepInputPortPosition step =
    { x = step.position.x + stepDimensions.width / 2
    , y = step.position.y
    }
```

## Phase 2: SVG ポート描画位置の変更

### 確認事項

- パターン: `viewStepNode` の SVG circle 属性 → `Designer.elm:1314-1341`

### 操作パス

該当なし（ドメインロジックのみ）

### テストリスト

ユニットテスト（該当なし — view 関数の座標変更、DesignerTest は update ロジックのテスト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 実装

`frontend/src/Page/WorkflowDefinition/Designer.elm` の `viewStepNode`:

出力ポート（行1314-1329）:
- `cx`: `dim.width` → `dim.width / 2`
- `cy`: `dim.height / 2` → `dim.height`
- コメント: 「右端中央の円」→「下端中央の円」

入力ポート（行1331-1341）:
- `cx`: `"0"` → `String.fromFloat (dim.width / 2)`
- `cy`: `dim.height / 2` → `"0"`
- コメント: 「左端中央の円」→「上端中央の円」

## Phase 3: ベジェ曲線の制御点変更

### 確認事項

- パターン: `viewTransitionLine` のベジェ曲線パス → `Designer.elm:1469-1489`
- パターン: `viewPreviewLine` のベジェ曲線パス → `Designer.elm:1645-1667`

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ノード間の接続線が上→下に自然なカーブで表示される | 正常系 | 目視確認 |
| 2 | 接続線を新規ドラッグするとプレビュー線が上→下方向に表示される | 正常系 | 目視確認 |
| 3 | 接続線端点をドラッグして付け替えるとプレビュー線が正しく表示される | 正常系 | 目視確認 |

### テストリスト

ユニットテスト（該当なし — ベジェ曲線は SVG パス文字列生成、view 関数内部）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — 既存 E2E は申請フローの検証で、描画座標は対象外）

### 実装

水平制御点 → 垂直制御点への変更。2箇所（`viewTransitionLine` と `viewPreviewLine`）で同一の変更。

```elm
-- Before（水平オフセット）
dx = abs (endPos.x - startPos.x) / 3
-- C (startX+dx) startY, (endX-dx) endY, endX endY

-- After（垂直オフセット）
dy = abs (endPos.y - startPos.y) / 3
-- C startX (startY+dy), endX (endY-dy), endX endY
```

具体的なパス生成:
```
M startX startY C startX (startY+dy), endX (endY-dy), endX endY
```

## 検証

1. `just elm-test` — ユニットテスト通過
2. `just check-all` — 全テスト通過
3. 目視確認 — `just dev-all` で開発サーバーを起動し、ワークフローデザイナーで以下を確認:
   - ポートが上下に配置されている
   - 遷移線が上→下に流れる
   - 新規接続線ドラッグのプレビューが正しい方向
   - 接続線端点ドラッグ付け替えが正常動作
   - ステップのドラッグ移動時にポートが追従する

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ベジェ曲線制御点の変更が viewPreviewLine にも必要 | 不完全なパス | Phase 3 に viewPreviewLine の変更を明記 |
| 2回目 | SVG コメント（「右端中央」→「下端中央」等）の更新漏れ | 曖昧 | Phase 2 にコメント変更を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | ポート関数2つ、SVG描画2箇所、ベジェ曲線2箇所、テスト2つ — 全て記載 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 座標計算の具体値を全て記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 判断が必要な箇所なし（座標変換のみ） |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象外セクションに記載 |
| 5 | 技術的前提 | 前提が考慮されている | OK | SVG ベジェ曲線 C コマンドの制御点仕様確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | Issue #913 の仕様と完全一致 |
