# ADR-031: ConfirmDialog の HTML `<dialog>` 要素への移行

## ステータス

承認済み

## コンテキスト

Issue #265 で ConfirmDialog のアクセシビリティ改善が求められた。具体的には:

1. フォーカストラップ: Tab キーの循環をダイアログ内に閉じ込める（WAI-ARIA Dialog パターン）
2. ARIA ラベリング: `aria-labelledby`, `aria-describedby` の設定

当時の ConfirmDialog は `div` ベースで実装されており、フォーカストラップは自前で実装する必要があった。

## 検討した選択肢

### 選択肢 1: div + JavaScript フォーカストラップ

現在の `div` ベースの構造を維持し、JavaScript（または Elm Port 経由）でフォーカストラップを実装する。

評価:
- 利点: 既存構造の変更が最小限
- 欠点: フォーカストラップの自前実装はエッジケースが多い（Shadow DOM、iframe、動的に追加される要素など）。focus-trap ライブラリの追加か自前実装が必要

### 選択肢 2: HTML `<dialog>` + `showModal()`（採用）

`div` から HTML `<dialog>` 要素に移行し、`showModal()` メソッドでモーダル表示する。

評価:
- 利点: フォーカストラップ、ESC キー処理、`::backdrop` 表示がブラウザネイティブで提供される。外部依存なし
- 欠点: `showModal()` は命令的 API のため Elm Port が必要。`<dialog>` は Elm の標準ライブラリにヘルパーがなく `Html.node "dialog"` を使用

### 選択肢 3: WAI-ARIA Dialog パターンの手動実装

`div` に `role="dialog"`, `aria-modal="true"` を設定し、フォーカストラップ・ESC 処理・backdrop をすべて手動で実装する。

評価:
- 利点: 最大限の制御が可能
- 欠点: 実装量が多く、ブラウザ間の差異への対応が必要。2026 年時点では `<dialog>` が標準的な方法

### 比較表

| 観点 | div + JS トラップ | `<dialog>` + `showModal()` | 手動 ARIA 実装 |
|------|------------------|---------------------------|---------------|
| フォーカストラップ | ライブラリ or 自前 | ブラウザネイティブ | 自前 |
| ESC キー処理 | 手動 | `cancel` イベント | 手動 |
| Backdrop | 手動 | `::backdrop` 疑似要素 | 手動 |
| 外部依存 | focus-trap 等 | なし | なし |
| ブラウザサポート | 全て | Chrome 37+, Firefox 98+, Safari 15.4+ | 全て |
| 実装量 | 中 | 小 | 大 |

## 決定

選択肢 2「HTML `<dialog>` + `showModal()`」を採用する。

主な理由:
1. フォーカストラップ・ESC・backdrop がブラウザネイティブで提供され、自前実装のエッジケースを回避できる
2. 2026 年時点で全主要ブラウザがサポートしており、[WAI-ARIA Dialog パターン](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)でも `<dialog>` の使用が推奨されている
3. 外部ライブラリ不要で依存関係が増えない

Elm Port が必要になる点はトレードオフとして受け入れる。`showModal()` は命令的 API だが、Port の実装は単純（`requestAnimationFrame` + `showModal()` の数行）であり、保守コストは低い。

## 帰結

### 肯定的な影響

- フォーカストラップがブラウザネイティブで動作し、エッジケースへの対応が不要
- ESC キー処理がコンポーネント内の `cancel` イベントに統合され、各ページの `subscriptions` から ESC 購読が不要に
- `Util/KeyEvent` モジュールが不要になり削除（コードの純減: -192 行）
- `::backdrop` 疑似要素により、オーバーレイの CSS がシンプルに

### 否定的な影響・トレードオフ

- `showModal()` のために Elm Port（`showModalDialog`）が必要
- `<dialog>` の `cancel` イベントで `preventDefault` しないとネイティブの閉じ動作が Elm の状態と不整合になる（`preventDefaultOn "cancel"` で対応済み）
- `::backdrop` 疑似要素は DOM イベントをバインドできないため、backdrop クリック検出に `pointer-events` トリックが必要

### 関連ドキュメント

- Issue: [#265](https://github.com/ka2kama/ringiflow/issues/265)
- PR: [#278](https://github.com/ka2kama/ringiflow/pull/278)
- 参考: [WAI-ARIA Dialog (Modal) Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-07 | 初版作成 |
