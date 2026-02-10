# ApproverSelector コンポーネント抽出

## 概要

ADR-043 優先順位 3 に基づき、`Page/Workflow/New.elm`（1115行）から承認者選択の UI とキーボードナビゲーションロジックを `Component/ApproverSelector.elm` に抽出した。純粋なリファクタリングであり、動作変更なし。

## 実施内容

### Phase 1: Component/ApproverSelector.elm を作成

TDD で新コンポーネントを作成した。

- `ApproverSelectorTest.elm`: `handleKeyDown` と `init` のテスト 8件（Red）
- `ApproverSelector.elm`: 型定義、ロジック、view 実装（Green）

コンポーネント API:
- `ApproverSelection` 型: `NotSelected | Selected UserItem`
- `State` type alias: 4フィールド（selection, search, dropdownOpen, highlightIndex）を統合
- `KeyResult` 型: `NoChange | Navigate Int | Select UserItem | Close`
- `handleKeyDown`: 純粋関数としてキーボードナビゲーションロジックを抽出
- `view`: config record パターンで 8つのコールバック/データを受け取る

### Phase 2: Page/Workflow/New.elm を修正

- Model の 4フィールドを `approver : ApproverSelector.State` に統合
- 5つのメッセージハンドラを `model.approver` 経由に修正
- `handleApproverKeyDown` を `ApproverSelector.handleKeyDown` + `KeyResult` パターンマッチに置換
- 7つの view 関数を `ApproverSelector.view` 呼び出しに置換
- `Json.Decode` import を削除（keydown デコーダがコンポーネントに移動）

### Phase 3: NewTest.elm を修正

- `ApproverSelection(..)` の import 元を `Component.ApproverSelector` に変更
- `modelWithUsers` のフィールドアクセスをネスト構造に更新
- assertion のフィールドアクセスを `sut.approver.xxx` に更新

## 判断ログ

- config record パターンを採用（Nested TEA は ADR-043 で却下済み）
- `KeyResult` 型で副作用を分離: 純粋なナビゲーション結果を返し、`markDirty` や `validationErrors` 更新は親ページに残す
- view スコープ: ステップヘッダーと label はページ固有レイアウトとして親に残し、選択 UI・ドロップダウン・エラー表示をコンポーネントが担当

## 成果物

### コミット

- `#290 Extract ApproverSelector component from Page/Workflow/New.elm`

### 行数変化

| ファイル | 変更前 | 変更後 |
|---------|--------|--------|
| New.elm | 1115 | 977（-138行） |
| ApproverSelector.elm | — | 318（新規） |
| ApproverSelectorTest.elm | — | 144（新規） |
| NewTest.elm | 265 | 271（+6行） |

### 作成・更新ファイル

- `frontend/src/Component/ApproverSelector.elm`（新規）
- `frontend/tests/Component/ApproverSelectorTest.elm`（新規）
- `frontend/src/Page/Workflow/New.elm`（修正）
- `frontend/tests/Page/Workflow/NewTest.elm`（修正）
- `prompts/plans/mighty-drifting-duckling.md`（計画ファイル）

### PR

- [#391](https://github.com/ka2kama/ringiflow/pull/391)（Draft）
