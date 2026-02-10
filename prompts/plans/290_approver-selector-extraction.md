# ApproverSelector コンポーネント抽出計画

## Context

Issue #290 の残タスク（ADR-043 優先順位 3）。`Page/Workflow/New.elm`（1115行）から承認者選択の UI とキーボードナビゲーションロジックを `Component/ApproverSelector.elm` に抽出する。純粋なリファクタリングであり、動作変更なし。

## 設計判断

### 1. config record パターン（Nested TEA は使わない）

既存 Component/ の全コンポーネント（Button, ConfirmDialog, MessageAlert 等）は stateless な config record パターンを採用。ApproverSelector も同じパターンに従う。ADR-043 で Nested TEA（選択肢 B）は明確に却下済み。

### 2. State type alias でモデルフィールドをグループ化

New.elm の 4 つの承認者関連フィールドを `ApproverSelector.State` に統合し、`model.approver` として管理する。

### 3. handleKeyDown は純粋関数として抽出

キーボードナビゲーションロジックを `KeyResult` 型を返す純粋関数として抽出。副作用（`markDirty`, `validationErrors` 更新）は親ページに残す。

### 4. コンポーネントの view 範囲

コンポーネントは「選択状態表示 / 検索入力 / ドロップダウン / バリデーションエラー」を担当。"Step 3: 承認者選択" ヘッダーと label はページ固有レイアウトのため親に残す。

## コンポーネント API

```elm
module Component.ApproverSelector exposing
    ( ApproverSelection(..)
    , KeyResult(..)
    , State
    , handleKeyDown
    , init
    , view
    )

type ApproverSelection = NotSelected | Selected UserItem

type alias State =
    { selection : ApproverSelection
    , search : String
    , dropdownOpen : Bool
    , highlightIndex : Int
    }

type KeyResult = NoChange | Navigate Int | Select UserItem | Close

init : State

handleKeyDown :
    { key : String, candidates : List UserItem, highlightIndex : Int }
    -> KeyResult

view :
    { state : State
    , users : RemoteData ApiError (List UserItem)
    , validationError : Maybe String
    , onSearch : String -> msg
    , onSelect : UserItem -> msg
    , onClear : msg
    , onKeyDown : String -> msg
    , onCloseDropdown : msg
    }
    -> Html msg
```

## Phase 構成

### Phase 1: `Component/ApproverSelector.elm` を作成

#### 確認事項
- 型: `RemoteData` コンストラクタ → `krisajenkins/remotedata` パッケージ
- パターン: 既存コンポーネントの module exposing・doc comment → `Component/ConfirmDialog.elm`
- ライブラリ: `List.Extra.getAt` → Grep 確認済み（New.elm line 606）

#### 作成内容
- 型定義: `ApproverSelection`, `State`, `KeyResult`
- `init : State` — 初期状態
- `handleKeyDown` — ArrowDown/Up で Navigate、Enter で Select、Escape で Close、候補0件で NoChange
- `view` — config record を受け取り、選択状態に応じて表示を切り替え
- 内部 view: `viewSelectedApprover`, `viewSearchInput`, `viewDropdown`, `viewCandidate`, `viewNoResults`, `viewError`
  - New.elm の lines 929-1053 から移動、config 経由でメッセージコールバックを参照するように変更

#### テストリスト（`Component/ApproverSelectorTest.elm`）
- [ ] `handleKeyDown` — ArrowDown で `Navigate 1` を返す
- [ ] `handleKeyDown` — ArrowUp で循環 `Navigate` を返す（index 0 → 末尾）
- [ ] `handleKeyDown` — Enter で候補があれば `Select user` を返す
- [ ] `handleKeyDown` — Enter で候補なしなら `NoChange` を返す
- [ ] `handleKeyDown` — Escape で `Close` を返す
- [ ] `handleKeyDown` — 候補0件の ArrowDown で `NoChange` を返す
- [ ] `handleKeyDown` — 不明キーで `NoChange` を返す
- [ ] `init` — 初期状態が NotSelected, "", False, 0

### Phase 2: `Page/Workflow/New.elm` を修正

#### 確認事項
- パターン: Component import パターン → Grep `import Component.` in Page/（確認済み）

#### 変更内容
1. `import Component.ApproverSelector as ApproverSelector exposing (ApproverSelection(..))` 追加
2. `type ApproverSelection` 定義を削除（lines 60-64）
3. Model: 4 フィールド → `approver : ApproverSelector.State`
4. `init`: 4 初期値 → `approver = ApproverSelector.init`
5. update: 5 メッセージハンドラを `model.approver` 経由に修正
   - `ClearApprover` → `approver = ApproverSelector.init` で簡素化
   - `ApproverKeyDown` → `ApproverSelector.handleKeyDown` + `KeyResult` パターンマッチ
6. `handleApproverKeyDown` 関数を完全削除（lines 566-631）
7. `validateFormWithApprover`: `model.approverSelection` → `model.approver.selection`
8. `Submit` ハンドラ: `model.approverSelection` → `model.approver.selection`（3箇所）
9. `viewApproverSection`: 内部の case 分岐を `ApproverSelector.view` 呼び出しに置換
10. 6 つの view 関数を削除（lines 929-1053: viewSelectedApprover〜viewApproverError）
11. exposing リストから `ApproverSelection(..)` を削除
12. `import Json.Decode as Decode` を削除（keydown デコーダがコンポーネントに移動。`List.Extra` は `getSelectedDefinition` で使用するため残す）

#### テストリスト
- [ ] `just check` 通過（コンパイルエラーなし）

### Phase 3: `Page/Workflow/NewTest.elm` を修正

#### 確認事項: なし（既知のパターンのみ）

#### 変更内容
1. import: `ApproverSelection(..)` を `Component.ApproverSelector` から import
2. `modelWithUsers` のフィールド: `approverSearch` → `approver.search` 等
3. assertion のフィールドアクセス: `sut.approverHighlightIndex` → `sut.approver.highlightIndex` 等

#### テストリスト
- [ ] `just check` 通過（全既存テスト合格）

## 行数見積もり

| ファイル | 変更前 | 変更後 |
|---------|--------|--------|
| New.elm | 1115 | ~929（削減 ~186行）|
| ApproverSelector.elm | — | ~200（新規）|
| ApproverSelectorTest.elm | — | ~80（新規）|
| NewTest.elm | 265 | ~270（微増）|

## リスク

| リスク | 対策 |
|--------|------|
| Elm の nested record update 不可 | `let approver = model.approver` パターンを一貫して使用 |
| `Json.Decode` 削除可否 | New.elm で他に使用なし（keydown デコーダのみ）→ 削除可能 |

## 検証方法

```bash
just check-all  # lint + test + API test（全通過で完了）
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `view` が `RemoteData` を受け取るか `List UserItem` か未決定 | 曖昧 | Loading/Failure メッセージ表示が承認者選択の責務 → `RemoteData` を渡す |
| 2回目 | ステップヘッダー・label をコンポーネントに含めるか | 責務の明確さ | ステップ番号はページ固有 → 親に残す |
| 3回目 | `ClearApprover` で全フィールド個別リセット | シンプルさ | `ApproverSelector.init` をそのまま使える |
| 4回目 | `List.Extra` import の削除可否 | 既存手段の見落とし | `getSelectedDefinition` で使用 → 残す |
| 5回目 | `viewApproverError` が `Dict` に依存 | 責務の明確さ | 親が `Dict.get` → `Maybe String` でコンポーネントに渡す |
| 6回目 | 候補0件の ArrowDown/Up | エッジケース | 実質変化なし → `NoChange` を返す |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 承認者関連コード全体（型、update 5ハンドラ、handleApproverKeyDown、view 7関数、validation の一部、Submit 内参照3箇所）を計画に含めた |
| 2 | 曖昧さ排除 | OK | ブラッシュアップループ6回で RemoteData の扱い、ヘッダー配置、ClearApprover 実装、List.Extra、validationError の渡し方、候補0件時の挙動を確定 |
| 3 | 設計判断の完結性 | OK | config record パターン、State 統合、KeyResult 設計、ApproverSelection 配置、view 範囲の5判断に理由を記載 |
| 4 | スコープ境界 | OK | 対象: 承認者選択の型・view・キーボードロジック。対象外: markDirty（ページ固有副作用）、ステップヘッダー（ページ固有レイアウト）、Submit ロジック |
| 5 | 技術的前提 | OK | Elm nested record update 不可の制約、preventDefaultOn デコーダ移動を考慮 |
| 6 | 既存ドキュメント整合 | OK | ADR-043「Component 抽出（選択肢 A）」に一致。~180行削減の見積もりとも整合 |
