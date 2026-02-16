# Issue #492: ApproverSelector の UserRef → UserItem 変換でダミー値を排除する

## Context

`Page/Workflow/Detail.elm` L403 の `StartEditing`（再提出モード開始）で、既存の承認者情報 `UserRef`（id + name のみ）を `UserItem`（id + displayId + displayNumber + name + email）に変換する際にダミー値をハードコードしている:

```elm
{ id = ref.id, name = ref.name, displayNumber = 0, displayId = "", email = "" }
```

`viewSelectedApprover` が `user.displayId` を表示するため、空文字列が表示されるバグがある。型設計を改善し、ダミー値を排除する。

## 設計判断

### `ApproverSelection` を 3 バリアントに拡張

```elm
type ApproverSelection
    = NotSelected
    | Preselected UserRef        -- ワークフローの既存データから事前設定
    | Selected UserItem          -- ドロップダウンから明示的に選択
```

**選択理由:**
- Elm の "make impossible states impossible" 哲学に合致。ダミー値が型レベルで不要になる
- `Preselected` は「事前に選択されている」の明確な英語表現
- 既存の `NotSelected` / `Selected` コンストラクタは変更なし（破壊的変更の最小化）

**不採用の代替案:**
- `Selected { id, name, detail : Maybe { displayId, email } }`: 単一バリアントだが、Maybe のハンドリングが煩雑。不正な組み合わせを表現可能にしてしまう
- API 呼び出しでフル情報を取得: 不必要なネットワークリクエスト。表示に必要な情報は name で十分

### `selectedUserId` ヘルパーの導入

```elm
selectedUserId : ApproverSelection -> Maybe String
```

`buildApprovers`/`buildResubmitApprovers` で 3 パターン分岐を `Maybe.map` に置き換え、コードを簡潔にする。`validate*` 関数でも `== Nothing` で判定可能。

### `viewSelectedApprover` のシグネチャ変更

`UserItem -> msg -> Html msg` → `String -> Maybe String -> msg -> Html msg`

`Preselected` の場合は名前のみ表示、`Selected` の場合は名前 + displayId を表示する。

## 対象と対象外

**対象:**
- `Component/ApproverSelector.elm` — 型定義 + ヘルパー + ビュー
- `Page/Workflow/Detail.elm` — ダミー値バグの修正
- `Page/Workflow/New.elm` — 網羅的パターンマッチ対応
- テスト — `selectedUserId` のテスト追加

**対象外:**
- バックエンド API（変更不要）
- `Data/UserRef.elm`, `Data/UserItem.elm`（変更不要）
- `Api/Workflow.elm`（`StepApproverRequest` は `{ stepId, assignedTo }` で変更不要）

---

## Phase 1: ApproverSelector の型変更 + ヘルパー追加

### 確認事項
- [x] 型: `ApproverSelection` の定義 → `Component/ApproverSelector.elm` L58-60, `NotSelected | Selected UserItem` の 2 バリアント
- [x] 型: `UserRef` の定義 → `Data/UserRef.elm` L32-35, `{ id : String, name : String }`
- [x] パターン: exposing リスト → `Component/ApproverSelector.elm` L1-8, `ApproverSelection(..)` で全コンストラクタ公開
- [x] ライブラリ: `Data.UserRef` の import パス → `Data.UserRef exposing (UserRef)` (WorkflowInstance.elm, WorkflowComment.elm, Task.elm で使用済み)

### 変更内容

**`Component/ApproverSelector.elm`:**

1. `import Data.UserRef exposing (UserRef)` を追加
2. `ApproverSelection` に `Preselected UserRef` バリアントを追加
3. `selectedUserId` ヘルパー関数を追加し、exposing リストに追記
4. `view` 関数のパターンマッチに `Preselected ref` ケースを追加
5. `viewSelectedApprover` のシグネチャを `String -> Maybe String -> msg -> Html msg` に変更

### テストリスト

ユニットテスト (`tests/Component/ApproverSelectorTest.elm`):
- [ ] `selectedUserId` が `NotSelected` に対して `Nothing` を返す
- [ ] `selectedUserId` が `Selected user` に対して `Just user.id` を返す
- [ ] `selectedUserId` が `Preselected ref` に対して `Just ref.id` を返す

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 2: Detail.elm のバグ修正 + New.elm のパターンマッチ対応

### 確認事項
- [x] パターン: `StartEditing` のダミー値箇所 → `Detail.elm` L401-407, `Selected { id = ref.id, ... dummy ... }` 確認済み
- [x] パターン: `validateResubmit` → `Detail.elm` L621-637, `NotSelected/Selected` の 2 分岐
- [x] パターン: `buildResubmitApprovers` → `Detail.elm` L642-654, `Selected user -> user.id` / `NotSelected -> Nothing`
- [x] パターン: `validateFormWithApprover` → `New.elm` L562-582, Detail.elm と同構造
- [x] パターン: `buildApprovers` → `New.elm` L664-676, Detail.elm と同構造

### 変更内容

**`Page/Workflow/Detail.elm`:**

1. L403: `Selected { id = ref.id, ... dummy ... }` → `Preselected ref`
2. `validateResubmit`: `selectedUserId` を使って簡潔に書き換え
3. `buildResubmitApprovers`: `selectedUserId` + `Maybe.map` に書き換え

**`Page/Workflow/New.elm`:**

1. `validateFormWithApprover`: `selectedUserId` を使って簡潔に書き換え
2. `buildApprovers`: `selectedUserId` + `Maybe.map` に書き換え

注: `New.elm` は `Preselected` を生成しないが、Elm の網羅的パターンマッチに対応するため `selectedUserId` 経由で処理する。

### テストリスト

ユニットテスト:
- [ ] 既存テスト全パス確認（`NewTest.elm` L252 の `Selected testUser1` は変更不要 — コンストラクタ名不変）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `selectedUserId` が実際にコードを簡潔にするか未検証 | 既存手段の見落とし | `buildApprovers`/`validate*` の具体コードと比較し、DRY に寄与することを確認 |
| 2回目 | `Detail.elm` への `Data.UserRef` import が必要かどうか | 未定義 | `Preselected ref` の `ref` は `Maybe UserRef` から取り出した値。`ApproverSelector` が `Preselected` コンストラクタと `UserRef` 型を expose するため、`Detail.elm` では `ApproverSelector.Preselected ref` で使用可能。ただし型アノテーションで `UserRef` を参照する箇所はないため `Data.UserRef` の直接 import は不要 |
| 3回目 | `viewSelectedApprover` のシグネチャ変更で email 等の将来的な表示拡張余地が失われる | シンプルさ | 現状 name + displayId のみ表示。YAGNI に従い現状で十分。将来必要になれば再リファクタリング |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | `ApproverSelection` 参照箇所: ApproverSelector.elm(定義+view)、Detail.elm(4箇所)、New.elm(4箇所)、テスト2ファイル。全て計画に含まれている |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全変更箇所に具体的なコード変更を記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 3バリアント設計の採用理由、`selectedUserId` 導入の判断、`viewSelectedApprover` シグネチャ変更の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象3ファイル+テスト、対象外（バックエンド・Data層・Api層）を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Elm の網羅的パターンマッチにより `New.elm` でも `Preselected` 分岐が必須。`ApproverSelection(..)` で全コンストラクタが自動 expose される |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 関連する ADR なし。Issue #492 の対策案1と一致する設計 |

## 検証方法

1. `cd frontend && pnpm run test` — 全 Elm テストパス
2. `just check` — lint + テスト通過
3. 手動確認: ワークフロー詳細画面 → 再提出モード開始 → 既存承認者が名前のみで表示される（空の displayId が表示されない）
