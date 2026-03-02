# #1004 Designer.elm の分割（1965行 → 7ファイル）

## 概要

ADR-062 の分割計画に基づき、Designer.elm（1965行）を責務別にサブモジュールへ分割する。

## スコープ

対象:
- `frontend/src/Page/WorkflowDefinition/Designer.elm` → 6 つのサブモジュールに分割

対象外:
- Detail.elm, instance.rs, New.elm の分割（別 Story: #1005, #1006, #1007）
- 機能追加や動作変更

## ADR-062 からの変更点

ADR-062 は 6 ファイル構成を計画したが、Elm のモジュール循環依存制約により Types.elm が必要。

| 項目 | ADR-062 | 本計画 |
|------|---------|--------|
| ファイル数 | 6 | 7（Types.elm 追加） |
| View サブモジュールの方式 | コールバックパターン | Types.elm からの直接 import |

理由:
1. Update.elm は Msg と CanvasState を参照する必要がある。これらが Designer.elm にあると循環依存が発生する
2. Types.elm を導入すれば、全サブモジュールが Types から import し、Designer.elm が全モジュールを統合する一方向の依存グラフになる
3. View サブモジュールも Types.elm から直接 import することで、コールバックレコードのボイラープレートを回避。全サブモジュールはページ固有（再利用不要）であり、コールバックパターンの利点（汎用性）が活きない

```
Types.elm ← Update.elm ← Designer.elm
         ← Canvas.elm ←
         ← PropertyPanel.elm ←
         ← Palette.elm ←
         ← Toolbar.elm ←
```

## 分割後のファイル構成

```
Page/WorkflowDefinition/
├── Designer.elm          (~200行: re-exports, init, subscriptions, update router, view layout)
└── Designer/
    ├── Types.elm          (~100行: Model, PageState, CanvasState, Msg, canvasElementId)
    ├── Update.elm         (~480行: handleGotDefinition, updateLoaded, helpers)
    ├── Canvas.elm         (~600行: SVG キャンバス描画)
    ├── PropertyPanel.elm  (~140行: プロパティパネル)
    ├── Palette.elm        (~75行: ステップパレット + アイコン)
    └── Toolbar.elm        (~170行: ツールバー、メッセージ、バリデーション、ステータスバー、ダイアログ)
```

注: Canvas.elm は ~600 行で 500 行閾値を超過する可能性がある。抽出後に実測し、超過する場合は ADR-062 の指針に従いステップ描画/接続線描画で追加分割する。

## 重複パターンの事前洗い出し

| 重複 | 箇所 | 対応 |
|------|------|------|
| `removeAt` | 797行（カスタム実装）| `List.Extra.removeAt` で置き換え（既にプロジェクトで使用） |
| `getAt` | 865行（カスタム実装）| `List.Extra.getAt` で置き換え（同ファイル 1758行で既に使用） |
| ベジェ曲線パスの計算 | `viewTransitionLine`(1469-1489行) と `viewPreviewLine`(1647-1667行) | 2 回のみの重複で 3 回繰り返しルールに未到達。分割後に同一モジュール内に残るため現時点では共通化しない |

## Phase 1: Types.elm の作成（基盤）

### 確認事項
- 型: `Model`, `PageState`, `CanvasState`, `Msg` の定義 → Designer.elm 59-164行
- パターン: 型の re-export パターン → Elm の標準パターン（import して exposing で再公開）
- ライブラリ: なし（既存の型定義の移動のみ）

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト: 既存の DesignerTest.elm が通ること（回帰テスト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 作業内容
1. `Designer/Types.elm` を作成し、以下を移動:
   - `canvasElementId` 定数
   - `type alias Model`
   - `type PageState`
   - `type alias CanvasState`
   - `type Msg`
   - 必要な import 文
2. Designer.elm を更新:
   - `import Page.WorkflowDefinition.Designer.Types` を追加
   - 移動した型定義を削除
   - `exposing` 句はそのまま維持（re-export）
3. テスト実行: `just test-elm`

## Phase 2: Update.elm の作成

### 確認事項
- 型: `CanvasState`, `Msg` → Types.elm（Phase 1 で作成済み）
- パターン: `List.Extra.removeAt`, `List.Extra.getAt` の API → Grep で既存使用を確認（Designer.elm 845行、1758行）
- ライブラリ: `List.Extra.removeAt` → elm-community/list-extra 8.7.0 で提供確認済み

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト: 既存の DesignerTest.elm が通ること（回帰テスト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 作業内容
1. `Designer/Update.elm` を作成し、以下を移動:
   - `handleGotDefinition` (187-234行)
   - `updateLoaded` (243-731行)
   - `syncPropertyFields` (736-746行)
   - `deleteSelectedStep` (751-771行)
   - `deleteSelectedTransition` (776-792行)
   - `handleReconnectionDrop` (808-860行)
2. 重複排除:
   - カスタム `removeAt` (797-799行) → `List.Extra.removeAt` に置き換え
   - カスタム `getAt` (865-867行) → `List.Extra.getAt` に置き換え
3. Designer.elm を更新:
   - `import Page.WorkflowDefinition.Designer.Update as DesignerUpdate` を追加
   - `update` 関数を更新: `handleGotDefinition` → `DesignerUpdate.handleGotDefinition`、`updateLoaded` → `DesignerUpdate.updateLoaded` に委譲
   - 移動した関数を削除
4. テスト実行: `just test-elm`

## Phase 3: Canvas.elm の作成

### 確認事項
- 型: `CanvasState`, `Msg(..)` → Types.elm
- パターン: SVG 描画パターン → Designer.elm 1107-1737行
- ライブラリ: `Svg`, `Svg.Attributes`, `Svg.Events` → 既存使用パターン確認

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト: 既存の DesignerTest.elm が通ること（回帰テスト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 作業内容
1. `Designer/Canvas.elm` を作成し、以下を移動:
   - `viewCanvasArea` (1107-1132行)
   - `viewCanvasBackground` (1140-1149行)
   - `viewGrid` (1157-1201行)
   - `viewSteps` + `viewStepNode` (1206-1342行)
   - `viewArrowDefs` + `viewArrowMarker` (1354-1381行)
   - `viewTransitions` + `viewTransitionLine` (1393-1555行)
   - `viewReconnectionHandleLayer` + `viewReconnectionHandles` (1406-1590行)
   - `viewConnectionDragPreview` + `viewPreviewLine` (1601-1678行)
   - `viewDragPreview` (1686-1737行)
2. Designer.elm を更新:
   - `import Page.WorkflowDefinition.Designer.Canvas as DesignerCanvas` を追加（ローカル `DesignerCanvas` alias と競合注意 → `import ... as Canvas` に変更）
   - `viewLoaded` 内の `viewCanvasArea` → `Canvas.viewCanvasArea` に委譲
3. 抽出後の行数を確認。500行超過の場合はさらに分割を検討
4. テスト実行: `just test-elm`

注: import alias の競合について — 既存の `import Data.DesignerCanvas as DesignerCanvas` と競合するため、Canvas サブモジュールは `as Canvas` で import する。

## Phase 4: PropertyPanel.elm の作成

### 確認事項
- 型: `CanvasState`, `Msg(..)` → Types.elm
- パターン: FormField の使用パターン → Designer.elm 1804-1884行

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト: 既存の DesignerTest.elm が通ること（回帰テスト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 作業内容
1. `Designer/PropertyPanel.elm` を作成し、以下を移動:
   - `viewPropertyPanel` (1751-1777行)
   - `viewNoSelection` (1782-1785行)
   - `viewTransitionProperties` (1790-1817行)
   - `viewStepProperties` (1822-1858行)
   - `viewStepTypeSpecificFields` (1863-1885行)
2. Designer.elm を更新:
   - `viewLoaded` 内の `viewPropertyPanel` → `PropertyPanel.view` に委譲
3. テスト実行: `just test-elm`

## Phase 5: Palette.elm の作成

### 確認事項: なし（既知のパターンのみ）

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト: 既存の DesignerTest.elm が通ること（回帰テスト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 作業内容
1. `Designer/Palette.elm` を作成し、以下を移動:
   - `viewPalette` (1028-1038行)
   - `viewPaletteItem` (1043-1058行)
   - `viewStepIcon` (1063-1099行)
2. Designer.elm を更新:
   - `viewLoaded` 内の `viewPalette` → `Palette.view` に委譲
3. テスト実行: `just test-elm`

## Phase 6: Toolbar.elm の作成

### 確認事項: なし（既知のパターンのみ）

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト: 既存の DesignerTest.elm が通ること（回帰テスト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 作業内容
1. `Designer/Toolbar.elm` を作成し、以下を移動:
   - `viewToolbar` (955-1009行)
   - `viewMessages` (1014-1020行)
   - `viewValidationPanel` (1893-1910行)
   - `viewValidationError` (1918-1932行)
   - `viewStatusBar` (1937-1946行)
   - `viewPublishDialog` (1951-1965行)
2. Designer.elm を更新:
   - `viewLoaded` 内の各関数呼び出しを Toolbar モジュールに委譲
3. テスト実行: `just test-elm`

## 最終検証

- [ ] Designer.elm ≤ 500 行
- [ ] 各サブモジュール ≤ 500 行
- [ ] `just check` 通過
- [ ] E2E テスト通過（`just test-e2e` で確認、UI 変更がないためデザイナーフローは既存テストで検証）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Elm の循環依存制約で ADR-062 の 6 ファイル構成が不可能 | アーキテクチャ不整合 | Types.elm を追加して 7 ファイル構成に変更 |
| 2回目 | `removeAt` / `getAt` が `List.Extra` と重複 | 既存手段の見落とし | Phase 2 で `List.Extra` に置き換え |
| 3回目 | `import Data.DesignerCanvas as DesignerCanvas` と Canvas サブモジュールの alias 競合 | 競合 | Canvas サブモジュールは `as Canvas` で import |
| 4回目 | Canvas.elm が ~600 行で閾値超過の可能性 | 不完全なパス | 抽出後に実測し、超過時は追加分割する方針を明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Designer.elm の全関数（28 Msg + 30 関数）が 6 サブモジュールのいずれかに割り当て済み |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | Canvas.elm の行数超過は「実測後に判断」と明記。他に不確定要素なし |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | ADR-062 との差異（Types.elm 追加、直接 import）に理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: Designer.elm のみ。対象外: 他の Story (#1005-1007) を明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | Elm の循環依存制約、List.Extra の API 確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-062, ADR-043 と整合。差異は理由付きで記載 |
