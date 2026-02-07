# ConfirmDialog アクセシビリティ改善

## 概要

Issue #265 に対応し、ConfirmDialog コンポーネントを `div` ベースから HTML `<dialog>` 要素 + `showModal()` に移行した。WAI-ARIA Dialog パターンに準拠するフォーカストラップと ARIA ラベリングを実現。

## 背景と目的

#177 の実装時に ConfirmDialog の UI/UX ベストプラクティス照合で 2 点のギャップが発見された:

1. フォーカストラップなし（Tab でダイアログ外に移動可能）
2. ARIA ラベリング未設定（`aria-labelledby`, `aria-describedby`）

全利用箇所（承認/却下確認 × 2ページ、ナビゲーション離脱確認）に恩恵が及ぶコンポーネントレベルの改善。

## 実施内容

### Phase 1: コンポーネント + Port + JS

- `ConfirmDialog.elm`: `div` → `Html.node "dialog"` に移行
  - `aria-labelledby`, `aria-describedby` 追加
  - `preventDefaultOn "cancel"` で ESC 処理をコンポーネント内に統合
  - `backdropClickDecoder` 追加（`pointer-events` + `target.nodeName` 検出）
  - `autofocus` をキャンセルボタンに追加
- `Ports.elm`: `showModalDialog` Port 追加
- `main.js`: `showModalDialog` ハンドラ追加（`requestAnimationFrame` + `showModal()`）
- `styles.css`: `::backdrop` スタイル追加
- `ConfirmDialogTest.elm`: 7 テスト新規作成

### Phase 2: ページ側更新 + クリーンアップ

3 ファイル（`Page/Task/Detail.elm`, `Page/Workflow/Detail.elm`, `Main.elm`）に共通パターンを適用:

- `focusDialogCancel` → `Ports.showModalDialog ConfirmDialog.dialogId`
- ESC サブスクリプション削除
- `NoOp` Msg バリアント削除
- 不要 import 削除（`Browser.Dom`, `Browser.Events`, `Task`, `Util.KeyEvent`）

クリーンアップ:
- `Util/KeyEvent.elm` と `KeyEventTest.elm` 削除
- `cancelButtonId` を ConfirmDialog の公開 API から除去

## 設計上の判断

| 判断 | 選択 | 理由 |
|------|------|------|
| ダイアログ実装方式 | HTML `<dialog>` + `showModal()` | ブラウザネイティブでフォーカストラップ・ESC・backdrop を提供。ADR-031 |
| ESC キー処理 | `cancel` イベントに統合 | 各ページの個別 `subscriptions` が不要に |
| Backdrop クリック | `pointer-events` + `target.nodeName` | `::backdrop` は DOM イベント非対応のため |
| 初期フォーカス | `autofocus` 属性 | `showModal()` が自動的に `autofocus` 要素にフォーカス |
| `subscriptions` の型 | `Sub Msg`（値）に変更 | 常に `Sub.none` を返すため、elm-review の指摘に従い関数→値に |

## 成果物

コミット:
- `547abcf` #265 Migrate ConfirmDialog from div to native `<dialog>` element
- `e8c5907` #265 Fix dialog centering by adding explicit h-full w-full

変更ファイル（10 ファイル、+720/-192 行）:
- 変更: `ConfirmDialog.elm`, `Ports.elm`, `main.js`, `styles.css`, `Main.elm`, `Page/Task/Detail.elm`, `Page/Workflow/Detail.elm`
- 新規: `tests/Component/ConfirmDialogTest.elm`
- 削除: `Util/KeyEvent.elm`, `tests/Util/KeyEventTest.elm`

ドキュメント:
- ADR-031: ConfirmDialog の `<dialog>` 要素への移行
- 実装解説: ConfirmDialog の `<dialog>` 要素への移行
- 改善記録: Plan ファイルの永続性誤認による実装解説の欠落

PR: #278

## 議論の経緯

### 実装方式の選定

Plan mode で 3 つの選択肢（div + JS トラップ、`<dialog>` + `showModal()`、手動 ARIA 実装）を比較検討した。WAI-ARIA Dialog パターンの公式ページも参照し、2026 年時点では `<dialog>` が標準的な方法であることを確認して採用を決定。

### elm-review による `subscriptions` の型変更

Phase 2 完了後の `just check` で、elm-review が `subscriptions _ = Sub.none` の未使用パラメータを指摘した。elm-review の自動修正提案に従い、`subscriptions : Model -> Sub Msg`（関数）から `subscriptions : Sub Msg`（値）に変更。Main.elm 側も呼び出し方を調整した。

### ダイアログの中央配置修正

ブラウザ手動確認で、ダイアログが左上に表示される問題を発見。`<dialog>` を `showModal()` で開くとブラウザの top layer に昇格するが、`position: fixed; inset: 0;` だけでは幅・高さが全画面にならない場合がある。`h-full w-full`（`height: 100%; width: 100%;`）を明示指定して解決。

## 学んだこと

- Elm で `<dialog>` を使うには `Html.node "dialog"` が必要（標準ライブラリにヘルパーなし）
- `showModal()` は `autofocus` 属性の要素に自動フォーカスするため、`Browser.Dom.focus` + `Task.attempt` の手動フォーカスが不要になる
- `::backdrop` 疑似要素は CSS スタイリングのみ可能で、DOM イベントはバインドできない。クリック検出には `pointer-events-none`/`pointer-events-auto` と `event.target.nodeName` を組み合わせる
- `<dialog>` の `cancel` イベントで `preventDefault` しないと、ブラウザがネイティブにダイアログを閉じてしまい、Elm の状態（`pendingAction`）と不整合になる
- `<dialog>` を `showModal()` で top layer に昇格させた場合、`inset: 0` だけでは全画面に広がらない。明示的に `h-full w-full` が必要

## 次のステップ

- 将来の検討: ダイアログ閉じた後のフォーカス戻し（WAI-ARIA 推奨だが別 Issue）
