# #860 型安全ステートマシン: 中優先度 Story のアプローチ見直し

## Context

ADR-054 の判断基準が #855 で「ADT で表現できるか」→「不正な状態遷移が型レベルで防止されているか」に変更された。Epic #822 の中優先度 Story（確認ダイアログ 3 パターン）のアプローチを新基準で再評価し、最適な対応を決定する。

### 評価結果

3 ファイルの現行パターンを ADR-054 適用基準で評価した結果:

| ファイル | 現行パターン | ADR-054 基準「状態によって有効なフィールドが異なる」 | 判定 |
|---------|------------|------------------------------------------------|------|
| Role/List.elm | `Maybe RoleItem` + `isDeleting : Bool` | 該当しない（Bool 1 つ、差異が小さい） | 変更不要 |
| User/Detail.elm | `Maybe ConfirmAction` + `isSubmitting : Bool` | 該当しない（Bool 1 つ、差異が小さい） | 変更不要 |
| WorkflowDefinition/List.elm 確認ダイアログ | `Maybe PendingAction` + `isProcessing : Bool` | 該当しない（Bool 1 つ、差異が小さい） | 変更不要 |
| WorkflowDefinition/List.elm **作成ダイアログ** | `showCreateDialog : Bool` + 4 フラットフィールド | **該当する**（4 フィールドがダイアログ非表示時に無効） | **改善** |

確認ダイアログの `Maybe PendingAction` / `Maybe RoleItem` / `Maybe ConfirmAction` は既に ADT で型安全性を提供しており、不正状態は Elm の update 関数を通じた遷移では到達不能。

## スコープ

### 対象

- WorkflowDefinition/List.elm: 作成ダイアログのフォーム状態を `Maybe CreateDialogState` に抽出
- Issue #860 本文: 評価結果を記載
- Epic #822: 完了基準・タスクリストを更新

### 対象外

- Role/List.elm: `Maybe RoleItem` + `isDeleting` は適用基準外。変更不要
- User/Detail.elm: `Maybe ConfirmAction` + `isSubmitting` は適用基準外。変更不要
- ConfirmDialog コンポーネント: 変更なし

## 設計判断

### 1. `isProcessing` の分離

現状 `isProcessing` は確認操作（公開/アーカイブ/削除）と作成送信で共有されている。

- 作成ダイアログ: `CreateDialogState.isSubmitting` に移動
- 確認ダイアログ: `isProcessing` を Model に残す

根拠: Detail.elm の `EditingState.isResubmitting` パターンに準拠。

### 2. `Maybe CreateDialogState` を使用

`type CreateDialogState = Closed | Open OpenState` ではなく `Maybe` を使用する。状態が 2 つ（開/閉）で「閉」に追加データがないため、`Maybe` が適切。`Maybe PendingAction` と同じパターン。

### 3. `viewCreateDialog` の引数を `CreateDialogState -> Html Msg` に変更

Model 全体ではなく必要なデータのみ渡し、関数の責務を明確にする。

## 実装計画

### Phase 1: 作成ダイアログ状態の型抽出

対象: `frontend/src/Page/WorkflowDefinition/List.elm`

#### 確認事項

- 型: `Model` のフィールド定義 → `List.elm` L40-52
- パターン: Detail.elm の `EditState` ADT → `Detail.elm` L138-154
- パターン: `viewCreateDialog` の引数と使用箇所 → `List.elm` L575-621

#### 変更内容

1. `CreateDialogState` 型定義を追加

```elm
{-| 作成ダイアログの状態
ダイアログが開いているときのみ存在する。
-}
type alias CreateDialogState =
    { name : String
    , description : String
    , validationErrors : Dict String String
    , isSubmitting : Bool
    }
```

2. Model を変更

```elm
-- 削除: showCreateDialog, createName, createDescription, createValidationErrors
-- 追加:
, createDialog : Maybe CreateDialogState
-- 残す（確認ダイアログ専用）:
, isProcessing : Bool
```

3. update 関数を変更

- `OpenCreateDialog`: `createDialog = Just { name = "", ... }`
- `CloseCreateDialog`: `createDialog = Nothing`
- `InputCreateName/Description`: `case model.createDialog of Just dialog -> ...`
- `SubmitCreate`: `case model.createDialog of Just dialog -> ... { dialog | isSubmitting = True }`
- `GotCreateResult Ok`: `createDialog = Nothing`（`isProcessing` は変更しない）
- `GotCreateResult Err`: `{ dialog | isSubmitting = False }`（ダイアログは維持）

4. view を変更

- `viewCreateDialog : CreateDialogState -> Html Msg` に引数変更
- `model.isProcessing` → `dialog.isSubmitting` に置換
- `model.createName` → `dialog.name` 等に置換

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 新規作成 → 名前入力 → 作成 → 成功 | 正常系 | E2E（既存） |
| 2 | 新規作成 → 名前未入力 → バリデーションエラー | 準正常系 | コンパイル検証 |
| 3 | 新規作成 → キャンセル | 正常系 | コンパイル検証 |
| 4 | 確認ダイアログ（公開/アーカイブ/削除）が影響を受けない | 正常系 | E2E（既存） |

#### テストリスト

ユニットテスト（該当なし — 型リファクタリング。Elm コンパイラの型チェックが主要な検証手段）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — 既存の `workflow-definition-management.spec.ts` でリグレッション確認）

### Phase 2: Issue/Epic 更新

#### 確認事項

確認事項: なし（GitHub Issue 更新のみ）

#### 操作パス

操作パス: 該当なし（ドキュメント更新のみ）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

#### 変更内容

- Issue #860 本文: 評価結果セクションを追加
- Epic #822 完了基準: 「型安全ステートマシンで統一」→「型安全性の観点で評価され、必要な箇所が改善」に更新
- Epic #822 タスクリスト: 3 項目を評価結果に基づいて更新（WorkflowDefinition は改善済み、Role/User は変更不要）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `isProcessing` が確認・作成で共有。分離方針が未定義 | 状態依存フィールド | `CreateDialogState.isSubmitting` に移動、`isProcessing` は確認専用に残す |
| 2回目 | `GotCreateResult Err` 時にダイアログを閉じるとエラー確認不可 | 不完全なパス | `Err` 時は `createDialog` を維持し `isSubmitting = False`。`Ok` 時のみ `Nothing` |
| 3回目 | `viewCreateDialog` の引数が `Model` のままだと型抽出の利点が半減 | シンプルさ | 引数を `CreateDialogState -> Html Msg` に変更 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 3 ファイル全てに評価結果あり。実装変更は WorkflowDefinition/List.elm のみ |
| 2 | 曖昧さ排除 | OK | 各変更が具体的なコードスニペットで記述されている |
| 3 | 設計判断の完結性 | OK | `isProcessing` 分離、`Maybe` vs Custom Type、引数変更の 3 判断に根拠を記載 |
| 4 | スコープ境界 | OK | 対象・対象外を明記 |
| 5 | 技術的前提 | OK | `Ports.showModalDialog`、`<dialog>` 要素の挙動を確認済み |
| 6 | 既存ドキュメント整合 | OK | ADR-054 適用基準に合致、Detail.elm EditState パターンに準拠 |

## 検証

- Phase 1 完了後: `just check`（Elm コンパイル + テスト）
- 全体完了後: `just check-all`（リント + テスト + E2E）
- 既存 E2E テスト `workflow-definition-management.spec.ts` で作成・公開・アーカイブフローのリグレッション確認

## 主要ファイル

| ファイル | 役割 |
|---------|------|
| `frontend/src/Page/WorkflowDefinition/List.elm` | リファクタリング対象 |
| `frontend/src/Page/Workflow/Detail.elm` L138-154 | 参照パターン（EditState） |
| `docs/05_ADR/054_型安全ステートマシンパターンの標準化.md` | 設計判断の根拠 |
| `tests/e2e/tests/workflow-definition-management.spec.ts` | リグレッション検証 |
