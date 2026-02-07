# #265 ConfirmDialog アクセシビリティ改善

## Context

Issue #265: ConfirmDialog の WAI-ARIA Dialog パターン準拠。

現在の ConfirmDialog は `div` ベースで実装されており、2点のギャップがある:
1. フォーカストラップなし（Tab でダイアログ外に移動可能）
2. ARIA ラベリング未設定（`aria-labelledby`, `aria-describedby`）

## 設計判断

### HTML `<dialog>` 要素 + `showModal()` の採用

`div` ベースのまま JS でフォーカストラップを実装する方法もあるが、HTML `<dialog>` + `showModal()` に移行する。

| 観点 | div + JS フォーカストラップ | `<dialog>` + `showModal()` |
|------|--------------------------|---------------------------|
| フォーカストラップ | 自前実装（エッジケース多） | ブラウザネイティブ |
| ESC キー処理 | 手動（現状の方式） | `cancel` イベント |
| 外部依存 | focus-trap ライブラリ or 自前 | なし |
| ブラウザサポート | 全て | Chrome 37+, Firefox 98+, Safari 15.4+, Edge 79+ |
| ベストプラクティス | 旧来 | 2026年の標準 |

### ESC キー処理: dialog の cancel イベントに統合

現状は各ページ（3箇所）で `Browser.Events.onKeyDown (KeyEvent.escKeyDecoder CancelAction)` を個別実装。`<dialog>` 移行後は `cancel` イベントでコンポーネント内に統合する。

副作用: `Util/KeyEvent.elm` が未使用になるため削除する（全3利用箇所がこの変更で置換される）。

### バックドロップクリック: pointer-events + target 検出

`::backdrop` 疑似要素は DOM イベントをバインドできないため:
1. `<dialog>` を全画面透明に設定
2. 中間 flex コンテナに `pointer-events-none`
3. ダイアログボックスに `pointer-events-auto`
4. `event.target.nodeName === "DIALOG"` でバックドロップクリックを検出

### cancelButtonId → dialogId に公開 API を変更

- 旧: ページ側で `Browser.Dom.focus ConfirmDialog.cancelButtonId` を呼び出し
- 新: ページ側で `Ports.showModalDialog ConfirmDialog.dialogId` を呼び出し
- 初期フォーカスは `autofocus` 属性で `showModal()` が自動処理

## スコープ

対象:
- ARIA ラベリング（`aria-labelledby`, `aria-describedby`）
- フォーカストラップ（`showModal()` によるネイティブ実装）
- `div` → `<dialog>` 移行
- ESC キー処理の移行（ページ個別 → コンポーネント内 `cancel` イベント）
- オーバーレイの移行（div → `::backdrop` + click detection）
- 未使用モジュール削除（`Util/KeyEvent`）

対象外:
- ダイアログ閉じた後のフォーカス戻し（WAI-ARIA 推奨だが別 Issue）
- アニメーション（開閉トランジション）

## 実装計画

### Phase 1: ConfirmDialog コンポーネント + Port + JS

テストリスト:
- [ ] dialog 要素で描画される
- [ ] aria-labelledby がタイトル要素の ID を参照する
- [ ] aria-describedby がメッセージ要素の ID を参照する
- [ ] タイトル要素に正しい ID が付与されている
- [ ] メッセージ要素に正しい ID が付与されている
- [ ] バックドロップクリックデコーダ: DIALOG ノードへのクリックで成功する
- [ ] バックドロップクリックデコーダ: 子要素へのクリックで失敗する

変更ファイル:

1. **`frontend/tests/Component/ConfirmDialogTest.elm`** (新規)
   - view のテスト（ARIA 属性、ID、要素タグ）
   - backdropClickDecoder のテスト（`Decode.decodeString` パターン）

2. **`frontend/src/Component/ConfirmDialog.elm`**
   - `div` → `Html.node "dialog"` に変更
   - `aria-labelledby`, `aria-describedby` 追加
   - `preventDefaultOn "cancel"` で ESC 処理（ネイティブ閉じを防止し onCancel 発火）
   - `backdropClickDecoder` 追加（`event.target.nodeName` チェック）
   - `autofocus True` をキャンセルボタンに追加
   - `dialogId`, `titleId`, `messageId` の ID 定数追加
   - 公開 API: `ActionStyle(..)`, `backdropClickDecoder`, `cancelButtonId`, `dialogId`, `view`
     - `cancelButtonId` は Phase 2 完了後に非公開化

3. **`frontend/src/Ports.elm`**
   - `showModalDialog : String -> Cmd msg` Port 追加

4. **`frontend/src/main.js`**
   - `showModalDialog` ハンドラ追加（`requestAnimationFrame` + `showModal()`）

5. **`frontend/src/styles.css`**
   - `#confirm-dialog::backdrop { background: rgb(0 0 0 / 0.5); }` 追加

### Phase 2: ページ側の更新 + クリーンアップ

3ファイル共通の変更パターン:

| 変更 | 内容 |
|------|------|
| focusDialogCancel | → `Ports.showModalDialog ConfirmDialog.dialogId` |
| ESC subscription | 削除 |
| focusDialogCancel 関数 | 削除 |
| NoOp Msg バリアント | 削除 |
| import 整理 | `Browser.Dom`, `Browser.Events`, `Task`, `Util.KeyEvent` 削除 |

変更ファイル:

1. **`frontend/src/Page/Task/Detail.elm`**
   - `import Ports` 追加
   - `focusDialogCancel` → `Ports.showModalDialog ConfirmDialog.dialogId`（L270, L276）
   - `focusDialogCancel` 関数削除（L272-274）
   - `subscriptions`: ESC 購読削除 → 常に `Sub.none`（L329-335）
   - `NoOp` 削除（L139, L219-220）
   - 不要 import 削除: `Browser.Dom`(L32), `Browser.Events`(L33), `Task`(L53), `Util.KeyEvent`(L56)

2. **`frontend/src/Page/Workflow/Detail.elm`** — 同パターン
   - `import Ports` 追加
   - `focusDialogCancel` → `Ports.showModalDialog ConfirmDialog.dialogId`（L254, L258）
   - `focusDialogCancel` 関数削除（L256-258）
   - `subscriptions`: ESC 購読削除 → 常に `Sub.none`（L307-314）
   - `NoOp` 削除（L144, L250-251）
   - 不要 import 削除: `Browser.Dom`(L34), `Browser.Events`(L35), `Task`(L52), `Util.KeyEvent`(L55)

3. **`frontend/src/Main.elm`**
   - `focusDialogCancel` → `Ports.showModalDialog ConfirmDialog.dialogId`（L292）
   - `focusDialogCancel` 関数削除（L503-506）
   - `subscriptions`: pendingNavigation の ESC 購読削除、`Sub.batch` 不要に（L519-537）
   - `NoOp` 削除（L273, L401-402）
   - 不要 import 削除: `Browser.Dom`(L15), `Browser.Events`(L16), `Task`(L34), `Util.KeyEvent`(L36)
   - 注: `Ports` は既に import 済み（L29）

4. **`frontend/src/Component/ConfirmDialog.elm`**
   - 公開 API から `cancelButtonId` を除去

5. **`frontend/src/Util/KeyEvent.elm`** — 削除
6. **`frontend/tests/Util/KeyEventTest.elm`** — 削除

7. `just check` で全体確認

## 技術的前提

| 前提 | 説明 |
|------|------|
| `Html.node "dialog"` | Elm に `<dialog>` ヘルパーがないため `Html.node` で生成（プロジェクト初使用） |
| `autofocus` + `showModal()` | `showModal()` は `autofocus` 属性の要素に自動フォーカスする |
| `preventDefaultOn "cancel"` | ESC 時のネイティブ閉じを防止し、Elm の状態管理に委ねる。`Page/Workflow/New.elm` に既存パターンあり |
| `requestAnimationFrame` | Port → `showModal()` の呼び出しで DOM 更新完了を保証 |
| `!dialog.open` チェック | 二重 `showModal()` 呼び出しによる `InvalidStateError` を防止 |
| `Test.Html.Selector.tag "dialog"` | 動作未確認。不可の場合は `id` セレクタで代替 |

## 検証方法

1. `just check` でコンパイル + テスト通過
2. `just dev-all` で開発サーバー起動
3. ブラウザで手動確認:
   - 承認/却下ダイアログ → フォーカスがキャンセルボタンにある
   - Tab/Shift+Tab でダイアログ内のみ循環
   - ESC でダイアログが閉じる
   - バックドロップクリックでダイアログが閉じる
   - ナビゲーション離脱ダイアログも同様

### ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版 → コンパイル整合性 | Phase 1 で cancelButtonId を非公開にするとページ側でエラー | cancelButtonId の非公開化を Phase 2 完了後に移動 |
| 2回目 | NoOp 削除可否 | 3ファイルで NoOp の使用箇所を Grep | 全て focusDialogCancel のみ → 削除可能 |
| 3回目 | import Task 削除可否 | 3ファイルでの Task 使用箇所を Grep | 全て Task.attempt (focusDialogCancel) のみ → 削除可能 |
| 4回目 | Main.elm subscriptions 簡素化 | Sub.batch の構造確認 | ESC 削除後 batch 不要 → case 文のみに |
| 5回目 | KeyEvent.elm 削除可否 | src/ 全体で KeyEvent の使用箇所を Grep | 3箇所すべて ConfirmDialog の ESC 処理 → 削除可能 |
| 6回目 | バックドロップクリックのデコーダ設計 | pointer-events-none/auto パターン vs stopPropagation | stopPropagation は no-op Msg が必要 → pointer-events + target 検出を採用 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 3利用箇所すべてカバー。変更ファイル11個（新規1、変更6、削除2、Phase1で4+Phase2で7）を特定 |
| 2 | 曖昧さ排除 | OK | 各ファイルの変更を行番号レベルで特定。1つの不確実点（tag "dialog" テスト）に代替策を明記 |
| 3 | 設計判断の完結性 | OK | 4つの判断（dialog採用、ESC統合、backdrop実装、API変更）すべてに理由を記載 |
| 4 | スコープ境界 | OK | 対象6項目、対象外2項目を明記 |
| 5 | 技術的前提 | OK | Html.node, autofocus, preventDefaultOn, rAF, dialog.open, Test.Html の6前提を確認 |
| 6 | 既存ドキュメント整合 | OK | Issue #265 の要件（フォーカストラップ + ARIA）をカバー。WAI-ARIA Dialog パターンと整合 |
