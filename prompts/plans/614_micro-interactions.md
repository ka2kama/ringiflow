# 実装計画: #614 マイクロインタラクションの改善

## Context

Epic #445（デザイン・UI/UX の品質をベストプラクティス水準に引き上げる）の Story 4。
構造的品質（#611, #612, #613）とビジュアル品質（#627, #628）の整備が完了し、残るインタラクション品質を改善する。

状態変化にトランジションがなく即座に表示/非表示される要素（MessageAlert、ConfirmDialog）、hover エフェクトの一貫性欠如（12箇所）、`prefers-reduced-motion` 未対応の 3 つのギャップを解消する。

## スコープ

対象:

- CSS `@keyframes` アニメーション定義（MessageAlert fade-in、ConfirmDialog scale-in）
- MessageAlert コンポーネントへのアニメーションクラス追加 + テスト
- ConfirmDialog コンポーネントへのアニメーションクラス追加 + テスト
- hover 要素への `transition-colors` 追加（12箇所）
- `prefers-reduced-motion` メディアクエリのグローバル対応
- styles.css デザインガイドライン更新

対象外:

- fade-out / close アニメーション: Elm の Virtual DOM は条件分岐で DOM 要素を即座に削除するため、CSS のみでは不可能。Elm 側の状態管理変更（Dismissing 状態 + タイマー）が必要で、MessageAlert の公開 API 変更を伴う。別 Issue として検討
- MessageAlert の自動消去タイマー: 現在の完了基準に含まれない
- パン屑リンクの `text-secondary-500` 追加: ビジュアル一貫性の問題であり、micro-interaction のスコープ外

## 設計判断

### 1. アニメーション手法: `@keyframes` を採用（`@starting-style` ではなく）

`@starting-style`（Baseline 2024）ではなく `@keyframes` を採用する。

- Elm は条件分岐で DOM 要素を生成/削除する。要素が DOM に追加された瞬間に `animation` が自動再生される `@keyframes` が、Elm の Virtual DOM パッチモデルと自然に統合される
- `@starting-style` は `display: none → block` のトランジションに適しているが、Elm のパターン（要素が DOM に存在しない → 存在する）とは異なる
- `@keyframes` はブラウザ互換性リスクがゼロ

### 2. ConfirmDialog アニメーション: CSS `dialog[open]` ターゲティング

ConfirmDialog の内容 div に Tailwind ユーティリティクラスを付与するのではなく、CSS `#confirm-dialog[open] .dialog-content` でターゲティングする。

理由:

- Elm が dialog 要素を DOM に追加 → 次フレームで `showModal()` が `open` 属性を追加 → CSS セレクタがマッチしてアニメーション開始。このタイミングが正しい（backdrop 表示と同期）
- ユーティリティクラス方式だと DOM 追加時点（`showModal()` 前）にアニメーションが開始し、backdrop と非同期になる
- アニメーションの関心を CSS に閉じ込め、Elm コンポーネントはセマンティッククラス（`dialog-content`）のみを知る

Elm 側の変更: 内容 div に `dialog-content` クラスを追加（CSS ターゲティング用）。

### 3. prefers-reduced-motion: グローバル CSS メディアクエリ

Tailwind の `motion-reduce:` variant を個別要素に適用するのではなく、グローバルなメディアクエリで全アニメーション/トランジションを一括無効化する。

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

- 個別対応より包括的で、新しいアニメーション追加時の対応漏れがない
- `0.01ms`（`0ms` ではなく）: 一部ブラウザで `animationend` イベントが発火しない問題を回避
- Andy Bell の Modern CSS Reset 等、多くの CSS ベースラインで採用されているパターン

### 4. MessageAlert アニメーション: Tailwind カスタムアニメーション

`@theme` で `--animate-alert-in` を定義し、Tailwind ユーティリティクラス `animate-alert-in` として使用する。

- `animate-spin`（LoadingSpinner）と同じパターンで一貫性がある
- アニメーション効果: fade-in + slide-down（`opacity: 0` + `translateY(-0.5rem)` → 通常状態）
- Duration: 200ms ease-out（notification 系 UI の標準的な値）

## Phase 1: CSS アニメーション基盤

### 概要

`@keyframes` 定義、`@theme` アニメーショントークン、`prefers-reduced-motion` メディアクエリを `styles.css` に追加する。

### 変更ファイル

- `frontend/src/styles.css`

### 変更内容

1. `@keyframes alert-in` を定義（fade-in + slide-down）
2. `@keyframes dialog-in` を定義（scale-in + fade-in）
3. `@keyframes backdrop-in` を定義（fade-in）
4. `@theme` ブロックに `--animate-alert-in` を追加
5. 既存の `#confirm-dialog::backdrop` ルールに `animation` を追加
6. `#confirm-dialog[open] .dialog-content` ルールを追加
7. `@media (prefers-reduced-motion: reduce)` メディアクエリを追加

### 確認事項

- [x] Tailwind v4 の `@theme` で `--animate-*` を定義すると `animate-*` ユーティリティクラスが生成される → 公式ドキュメントで確認済み
- [x] `@keyframes` の配置位置 → `@theme` ブロック内に配置する（`--animate-*` と対応する `@keyframes` は同じ `@theme` 内）。コンポーネント固有の `@keyframes`（dialog-in, backdrop-in）は `@theme` 外に配置

### テストリスト

ユニットテスト（該当なし — CSS のみの変更）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 2: MessageAlert アニメーション

### 概要

MessageAlert コンポーネントにアニメーションクラスを追加し、テストを作成する。

### 変更ファイル

- `frontend/src/Component/MessageAlert.elm`
- `frontend/tests/Component/MessageAlertTest.elm`（新規作成）

### 変更内容

1. `viewSuccessMessage` の `Just` ブランチの `div` クラスに `animate-alert-in` を追加
2. `viewErrorMessage` の `Just` ブランチの `div` クラスに `animate-alert-in` を追加
3. MessageAlertTest を新規作成（ConfirmDialogTest の `Query`/`Selector` パターンに従う）

### 確認事項

- [x] `Test.Html.Selector.class` の動作 → プロジェクト内に既存使用なし。テストで使用して動作確認済み（`Selector.class "animate-alert-in"` でクラス属性マッチ）
- [x] MessageAlert.view の引数型 → `MessageAlert.elm` L37-42 で確認済み。`{ onDismiss : msg, successMessage : Maybe String, errorMessage : Maybe String }`

### テストリスト

ユニットテスト:

- [x] 成功メッセージが表示されるとき `animate-alert-in` クラスを含む
- [x] エラーメッセージが表示されるとき `animate-alert-in` クラスを含む
- [x] 成功メッセージが Nothing のとき alert 要素が存在しない
- [x] エラーメッセージが Nothing のとき alert 要素が存在しない

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 3: ConfirmDialog アニメーション

### 概要

ConfirmDialog コンポーネントの内容 div に `dialog-content` クラスを追加し、既存テストを拡張する。アニメーション自体は Phase 1 で定義した CSS ルール（`#confirm-dialog[open] .dialog-content`）で適用される。

### 変更ファイル

- `frontend/src/Component/ConfirmDialog.elm`
- `frontend/tests/Component/ConfirmDialogTest.elm`

### 変更内容

1. `view` 関数の L101 の内容 div クラスに `dialog-content` を追加
   - 現在: `"pointer-events-auto w-full max-w-md rounded-lg bg-white p-6 shadow-xl"`
   - 変更後: `"dialog-content pointer-events-auto w-full max-w-md rounded-lg bg-white p-6 shadow-xl"`
2. ConfirmDialogTest の `viewTests` に `dialog-content` クラスの存在テストを追加

### 確認事項

- [x] ConfirmDialog.view の内容 div の現在のクラス → `ConfirmDialog.elm` L101 で確認。`"pointer-events-auto w-full max-w-md rounded-lg bg-white p-6 shadow-xl"`
- [x] ConfirmDialogTest の既存テストパターン → `ConfirmDialogTest.elm` L59-92 で確認。`Query.fromHtml` → `Query.has`/`Query.find` パターン

### テストリスト

ユニットテスト:

- [x] ダイアログの内容要素に `dialog-content` クラスが含まれる

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 4: hover トランジション統一

### 概要

hover 効果があるのに `transition-colors` がない要素に追加する。

### 変更ファイル（10 ファイル、12箇所）

パン屑リンク（5 ファイル、6箇所）:

1. `frontend/src/Page/User/New.elm` L297 — `"hover:text-primary-600"` → `"hover:text-primary-600 transition-colors"`
2. `frontend/src/Page/User/Edit.elm` L276 — 同上
3. `frontend/src/Page/User/Edit.elm` L278 — 同上
4. `frontend/src/Page/User/Detail.elm` L214 — 同上
5. `frontend/src/Page/Role/New.elm` L241 — 同上
6. `frontend/src/Page/Role/Edit.elm` L281 — 同上

インタラクティブ要素（5 ファイル、6箇所）:

7. `frontend/src/Main.elm` L898 — `"... hover:bg-secondary-50 lg:hidden"` に `transition-colors` 追加
8. `frontend/src/Page/Workflow/List.elm` L347 — `"ml-1 hover:text-success-900"` に `transition-colors` 追加
9. `frontend/src/Page/User/List.elm` L215 — `"font-medium text-primary-600 hover:text-primary-800"` に `transition-colors` 追加
10. `frontend/src/Page/Workflow/List.elm` L394 — `"text-primary-600 hover:text-primary-700 hover:underline"` に `transition-colors` 追加
11. `frontend/src/Component/ApproverSelector.elm` L234 — `"... hover:text-secondary-600 ..."` に `transition-colors` 追加
12. `frontend/src/Component/ApproverSelector.elm` L326 — `" hover:bg-primary-50"` に `transition-colors` 追加

### 確認事項

- [x] 各箇所の現在のクラス文字列 → Read で全12箇所を確認済み。計画通りのクラス文字列を確認
- [x] Main.elm L840 のナビ項目は L844 の基底クラスに `transition-colors` が含まれるため対象外 → 前セッションで Read で確認済み

### テストリスト

ユニットテスト（該当なし — CSS クラス文字列の追記のみ、ロジック変更なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 5: デザインガイドライン更新

### 概要

styles.css のガイドラインコメントにアニメーション・トランジションパターンを追記する。

### 変更ファイル

- `frontend/src/styles.css`

### 変更内容

ガイドラインコメントに以下を追加:

- アニメーション: `animate-alert-in`（通知表示）、`dialog-in`（ダイアログ開閉）
- トランジション: hover を持つ全要素に `transition-colors` を付与するルール
- `prefers-reduced-motion`: グローバルメディアクエリで一括無効化

### 確認事項

確認事項: なし（既知のパターンのみ）

### テストリスト

ユニットテスト（該当なし — ドキュメントのみ）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Main.elm L840 は L844 の基底クラスに `transition-colors` が含まれるため対象外 | 事実的妥当性 | hover 対象リストから除外 |
| 2回目 | ConfirmDialog のアニメーションを Tailwind ユーティリティクラスで付与すると `showModal()` 前にアニメーションが開始する | 技術的前提 | CSS `#confirm-dialog[open] .dialog-content` ターゲティングに変更。backdrop と同期してアニメーション開始 |
| 3回目 | パン屑リンクに `text-secondary-500` も欠落（Workflow/Detail, Task/Detail との不一致） | 既存パターン整合 | ビジュアル一貫性の問題であり #614 のスコープ外として「対象外」に明記 |
| 4回目 | Tailwind v4 で `@theme { --animate-* }` がユーティリティクラスに自動マッピングされるか未確認 | 技術的前提 | Phase 1 の確認事項に追加。`animate-spin` は Tailwind 組み込みのため別途確認が必要 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 完了基準の全項目がカバーされている | OK | 状態変化トランジション → Phase 1-3、hover 一貫性 → Phase 4、prefers-reduced-motion → Phase 1、just check-all → 各 Phase で検証 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更箇所がファイル名+行番号で特定済み。CSS コードスニペットを設計判断に提示 |
| 3 | 設計判断の完結性 | 全選択肢に判断理由が記載されている | OK | `@keyframes` vs `@starting-style`、`dialog[open]` ターゲティング vs ユーティリティクラス、グローバル vs 個別 `prefers-reduced-motion` の各判断に理由記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | fade-out/close アニメーション、自動消去タイマー、パン屑の `text-secondary-500` を対象外として明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Elm Virtual DOM の DOM 削除パターン、`showModal()` の `requestAnimationFrame` タイミング、`@keyframes` のブラウザサポート、Tailwind v4 の `@theme` → ユーティリティクラス生成を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | styles.css のデザインガイドライン（テーブル行 `transition-colors` パターン）と整合。ADR-027（Tailwind CSS 導入）の方針に準拠 |
