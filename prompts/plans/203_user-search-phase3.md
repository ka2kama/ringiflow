# Issue #203 Phase 3: Elm フロントエンド ユーザー検索・承認者選択 UI

## 概要

`Page/Workflow/New.elm` の承認者選択を UUID テキスト入力からオートコンプリート検索 UI に置き換える。

## 設計判断

### UI パターン: オートコンプリート型

| パターン | 判断 | 理由 |
|---------|------|------|
| A. オートコンプリート型 | **採用** | 既存テキスト入力と操作感が近い、キーボード操作と相性が良い |
| B. ドロップダウン型 | 不採用 | ユーザー数が多いと選びにくい |
| C. モーダル検索型 | 不採用 | 操作が重い、TEA 状態管理が複雑化 |

### データ取得: ページ init 時に一括取得

テナント内ユーザー数は限定的（数百人以下）なので、init 時に全件取得してフロントエンド側でフィルタリング。

### コンポーネント設計: Page 内インラインで実装

利用箇所が1つのため YAGNI に従い `Page/Workflow/New.elm` 内にインライン実装。フィルタリング関数 `filterUsers` のみ `Data/UserItem.elm` に配置してテスト可能にする。

## ファイル一覧

### 新規作成

| ファイル | 役割 |
|---------|------|
| `frontend/src/Data/UserItem.elm` | UserItem 型 + デコーダー + filterUsers |
| `frontend/src/Api/User.elm` | `GET /api/v1/users` クライアント |
| `frontend/tests/Data/UserItemTest.elm` | デコーダー + フィルタリングテスト |

### 変更

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Page/Workflow/New.elm` | Model/Msg/update/view の承認者選択を全面書き換え |

### 変更なし（確認済み）

- `Main.elm` — 公開インターフェース不変
- `Api/Workflow.elm` — `assignedTo` は引き続き UUID 文字列

## 型定義

### `Data/UserItem.elm`

```elm
type alias UserItem =
    { id : String          -- UUID（内部用）
    , displayId : String   -- "USER-N" 形式
    , displayNumber : Int
    , name : String
    , email : String
    }

decoder : Decoder UserItem       -- Json.Decode.Pipeline パターン
listDecoder : Decoder (List UserItem)  -- { "data": [...] } 形式
filterUsers : String -> List UserItem -> List UserItem  -- 名前/displayId/email 部分一致
```

### `Api/User.elm`

```elm
listUsers :
    { config : RequestConfig
    , toMsg : Result ApiError (List UserItem) -> msg
    }
    -> Cmd msg
-- 既存パターン: Api.get + UserItem.listDecoder
```

### `Page/Workflow/New.elm` の Model 変更

```elm
-- 新しい型
type ApproverSelection
    = NotSelected
    | Selected UserItem

-- Model に追加/変更するフィールド
, users : RemoteData ApiError (List UserItem)  -- 追加
, approverSearch : String              -- approverInput から変更
, approverSelection : ApproverSelection -- 追加
, approverDropdownOpen : Bool          -- 追加
, approverHighlightIndex : Int         -- 追加
-- , approverInput : String            -- 削除
```

### Msg 変更

```elm
-- 追加
| GotUsers (Result ApiError (List UserItem))
| UpdateApproverSearch String
| SelectApprover UserItem
| ClearApprover
| ApproverKeyDown String
| CloseApproverDropdown

-- 削除
-- | UpdateApproverInput String
```

## 状態遷移

```
[init] → NotSelected + search="" + dropdown=closed
  ↓ (テキスト入力)
NotSelected + search="田" + dropdown=open
  ↓ (候補クリック or Enter)
Selected userItem + search="" + dropdown=closed
  ↓ (×ボタン or ClearApprover)
NotSelected + search="" + dropdown=closed
```

キーボード: ArrowDown/Up でハイライト移動、Enter で選択、Escape でドロップダウン閉じる

## View 構造

```
Step 3: 承認者選択
├── [Selected の場合]
│   └── 選択済み表示（名前 + USER-N + ×ボタン）
│       bg-primary-50 border-primary-200 rounded-lg p-3
│
├── [NotSelected の場合]
│   ├── 検索入力（placeholder: "名前で検索..."）
│   └── [dropdown=open かつ候補あり]
│       └── 候補リスト（absolute z-10 shadow-lg max-h-60 overflow-y-auto）
│           └── 各候補: 名前 / USER-N · email
│
└── バリデーションエラー
```

## 技術的注意点

1. **onBlur/onClick 競合**: `Process.sleep 200 |> Task.perform (\_ -> CloseApproverDropdown)` で遅延クローズ
2. **キーボードスクロール防止**: `Html.Events.preventDefaultOn "keydown"` で ArrowDown/Up のデフォルト動作を抑制
3. **ハイライトリセット**: 検索テキスト変更時に `approverHighlightIndex = 0` にリセット

## テストリスト

### `tests/Data/UserItemTest.elm`

**デコーダーテスト:**
1. 全フィールドをデコード
2. 必須フィールドがない場合はエラー
3. data フィールドから一覧をデコード
4. 空の一覧をデコード
5. data フィールドがない場合はエラー

**フィルタリングテスト:**
6. 名前で部分一致フィルタリング
7. display_id でフィルタリング
8. email でフィルタリング
9. 大文字小文字を無視
10. 空クエリは空リストを返す
11. 一致なしは空リストを返す
12. 前後の空白をトリム

## 実装順序（TDD）

### Step A: Data/UserItem.elm（型 + デコーダー）
- 参考: `Data/WorkflowDefinition.elm`, `tests/Data/WorkflowDefinitionTest.elm`
1. Red: デコーダーテスト（1-5）
2. Green: 型定義 + デコーダー実装
3. Refactor

### Step B: Data/UserItem.elm（filterUsers）
1. Red: フィルタリングテスト（6-12）
2. Green: `filterUsers` 実装
3. Refactor

### Step C: Api/User.elm
- 参考: `Api/WorkflowDefinition.elm`
- `Api.get` + `UserItem.listDecoder` の薄いラッパー（テスト不要）

### Step D: Page/Workflow/New.elm（Model/Msg/init/update）
- Model に新フィールド追加、init に `fetchUsers` 追加（`Cmd.batch`）
- update に新 Msg ハンドラ実装
- `validateFormWithApprover` を `ApproverSelection` ベースに変更
- `submitWorkflow` の `approverInput` を `approverSelection` から UUID 取得に変更

### Step E: Page/Workflow/New.elm（view）
- `viewApproverSection` 書き換え: 選択済み表示 / 検索入力 + ドロップダウン

### Step F: 結合確認
- `just check-all` 通過
- コミット

## 対象外

- ログインユーザー自身の候補除外（将来の改善）
- ページネーション / サーバーサイドフィルタリング
- `Component/UserSearch.elm` としての汎用化（YAGNI）
- ARIA 属性の完全実装（基本的な role は付与、フル実装は将来）

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Data/UserItem.elm, Api/User.elm の新規作成、Page/Workflow/New.elm の変更、テストファイルをすべて計画に含めた。Main.elm 変更不要も確認 |
| 2 | 曖昧さ排除 | OK | 全ての型定義、Msg バリアント、CSS クラス、テストケースを具体的に記載 |
| 3 | 設計判断の完結性 | OK | UI パターン（3案）、取得タイミング（2案）、コンポーネント設計、filterUsers 配置の判断を記載 |
| 4 | スコープ境界 | OK | 対象・対象外を明示。自己除外、ページネーション、汎用化を対象外として記載 |
| 5 | 技術的前提 | OK | onBlur/onClick 競合の対策、preventDefaultOn、ハイライトリセットを明記 |
| 6 | 既存ドキュメント整合 | OK | OpenAPI 仕様の UserItem（5フィールド）と型定義一致。BFF の UserItemData と対応。Issue #203 完了基準4項目すべてカバー |
