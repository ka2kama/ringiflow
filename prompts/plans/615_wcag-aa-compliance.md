# #615 WCAG 2.1 AA 対応強化

Issue: [#615](https://github.com/ka2kama/ringiflow/issues/615)
Epic: [#445](https://github.com/ka2kama/ringiflow/issues/445)（Story 5）

## Context

CORE-06 要件「WCAG 2.1 AA 準拠（SHOULD）」のギャップを解消する。
既存の ARIA 対応は進んでいる（`role="alert"`, `aria-labelledby`, `<dialog>`, `prefers-reduced-motion`）が、以下 3 領域に未達がある:

1. カラーコントラスト: テーブルヘッダーの `text-secondary-500`（#64748b）が `bg-secondary-50`（#f8fafc）背景で ~4.4:1（AA 基準 4.5:1 未達）
2. フォームラベル: ApproverSelector 検索入力、PermissionMatrix チェックボックスに `label`/`aria-label` がない
3. フォーカスインジケータ: Button（全バリアント）、MessageAlert 閉じボタン、ApproverSelector 解除ボタンに `focus-visible` がない

## スコープ

対象:
- テーブルヘッダーのコントラスト比改善（`text-secondary-500` → `text-secondary-600`）
- ラベル未紐づけのフォーム要素に `aria-label` を追加
- `focus-visible` 未適用のインタラクティブ要素にフォーカスリングを追加
- デザインガイドライン（styles.css コメント）の更新

対象外:
- WAI-ARIA combobox パターンの完全実装（ApproverSelector に `role="combobox"`, `aria-activedescendant` 等を追加すること。WCAG AA 準拠には必須ではなく、別 Issue で扱う）
- `text-secondary-500` の全箇所変更（白背景上では ~4.6:1 で AA を満たすため、テーブルヘッダーの `bg-secondary-50` 上のみ修正）
- ダークモード対応（Epic スコープ外、CORE-06 で MAY）

## 設計判断

### テーブルヘッダーのコントラスト改善: `text-secondary-600` を採用

選択肢:
1. `text-secondary-600`（#475569）: bg-secondary-50 上で ~6.4:1、白背景上で ~6.8:1
2. `text-secondary-700`（#334155）: bg-secondary-50 上で ~8.5:1

`text-secondary-600` を採用。理由: テーブルヘッダーは `uppercase` + `tracking-wider` + `text-xs` で視覚的に十分目立つため、過度なコントラスト（700）は不要。600 で AA 基準を十分に超え（6.4:1 > 4.5:1）、かつ控えめなヘッダーの視覚的役割（本文より目立たない）を維持できる。

### ボタンのフォーカスリング: `ring-offset-2` を使用

背景色付きボタン（Primary, Success, Error, Warning）では、リングとボタンの間にオフセットがないとリングが背景色に溶け込む。`ring-offset-2` で白いギャップを設け、視認性を確保する。リング色は `ring-primary-500` で統一し、フォーム入力と一貫させる。

### MessageAlert / ApproverSelector の小ボタン: `rounded` + `ring-offset` なし

閉じボタン（×）や解除ボタンは背景透過のため、`ring-offset` は不要。`rounded` を追加してリングの角丸を確保する。

---

## Phase 1: カラーコントラスト改善

テーブルヘッダーの `text-secondary-500` → `text-secondary-600` 変更。

### 確認事項
- [ ] パターン: テーブルヘッダーのクラス文字列 → Grep 結果で 6 ファイル確認済み
- [ ] ライブラリ: Tailwind `text-secondary-600` トークン → styles.css @theme で定義済み

### 対象ファイル

| ファイル | 変更箇所 |
|---------|---------|
| `frontend/src/styles.css` | L16: デザインガイドラインコメントのテーブルヘッダー定義 |
| `frontend/src/Component/PermissionMatrix.elm` | L68, 70, 74: th 要素 |
| `frontend/src/Page/Workflow/List.elm` | L378-381: th 要素 |
| `frontend/src/Page/AuditLog/List.elm` | L332-336: th 要素 |
| `frontend/src/Page/User/List.elm` | L194-198: th 要素 |
| `frontend/src/Page/Task/List.elm` | L175-179: th 要素 |
| `frontend/src/Page/Role/List.elm` | L233-238: th 要素 |

### テストリスト

ユニットテスト（該当なし）: CSS クラス名の変更のみ。コントラスト比は CSS トークン値で決まるため、Elm テストでは検証不能。デザインガイドライン更新で記録。

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）: 視覚的な変更は目視確認。

---

## Phase 2: フォームラベル追加

### 確認事項
- [ ] 型: `PermissionMatrix.Config` → `PermissionMatrix.elm` L30-35
- [ ] パターン: 既存の `aria-label` 使用パターン → `MessageAlert.elm` L56: `attribute "aria-label" "閉じる"`, `ConfirmDialog.elm`: `attribute "aria-labelledby"`
- [ ] パターン: `PermissionMatrix.viewResourceRow` の引数構造 → L86-87: `( resourceKey, resourceLabel )` と `actions` リスト `( actionKey, actionLabel )` でラベル構築可能

### 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Component/ApproverSelector.elm` | L264: 検索入力に `attribute "aria-label" "承認者を検索"` を追加 |
| `frontend/src/Component/ApproverSelector.elm` | L232-237: 解除ボタンに `attribute "aria-label" "承認者を解除"` を追加 |
| `frontend/src/Component/PermissionMatrix.elm` | L99-106: "すべて" チェックボックスに `attribute "aria-label" (resourceLabel ++ " すべて")` を追加 |
| `frontend/src/Component/PermissionMatrix.elm` | L108-126: 個別チェックボックスに `attribute "aria-label" (resourceLabel ++ " " ++ actionLabel)` を追加。ラムダの `( actionKey, _ )` → `( actionKey, actionLabel )` に変更 |

### テストリスト

ユニットテスト:
- [ ] ApproverSelector: 検索入力に `aria-label="承認者を検索"` が存在する
- [ ] ApproverSelector: 解除ボタンに `aria-label="承認者を解除"` が存在する
- [ ] PermissionMatrix: "すべて" チェックボックスに `aria-label="ワークフロー すべて"` が存在する
- [ ] PermissionMatrix: 個別チェックボックスに `aria-label="ワークフロー 閲覧"` が存在する

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### テストファイルの変更

| ファイル | 変更内容 |
|---------|---------|
| `frontend/tests/Component/ApproverSelectorTest.elm` | `viewTests` describe ブロックを追加。`Test.Html.Query`/`Test.Html.Selector` をインポート。`view` を直接呼んで HTML 属性を検証 |
| `frontend/tests/Component/PermissionMatrixTest.elm` | 新規作成。`view` を呼んで aria-label 属性を検証 |

ApproverSelector の `view` テストには config の組み立てが必要。`users` に `RemoteData.Success [testUser1]` を使い、`state` に `Selected testUser1` を設定して選択済み表示をテスト。検索入力のテストは `state = init`（NotSelected）で描画。

PermissionMatrix の `view` テストには `Config` を組み立てる。`selectedPermissions = Set.empty`, `disabled = False`, ダミーの msg ハンドラを使用。

---

## Phase 3: フォーカスインジケータ追加

### 確認事項
- [ ] パターン: フォーム入力のフォーカスパターン → `FormField.elm`: `outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500`
- [ ] パターン: PermissionMatrix チェックボックス → L104: `outline-none focus-visible:ring-2 focus-visible:ring-primary-500`（既に対応済み）

### 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Component/Button.elm` | L142-143: `baseClass` に `outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-2` を追加 |
| `frontend/src/Component/MessageAlert.elm` | L56, L69: 閉じボタンのクラスに `rounded outline-none focus-visible:ring-2 focus-visible:ring-primary-500` を追加 |
| `frontend/src/Component/ApproverSelector.elm` | L234: 解除ボタンのクラスに `rounded outline-none focus-visible:ring-2 focus-visible:ring-primary-500` を追加 |

### テストリスト

ユニットテスト:
- [ ] Button: `baseClass` に `focus-visible:ring-2` を含む
- [ ] Button: `baseClass` に `focus-visible:ring-primary-500` を含む
- [ ] MessageAlert: 閉じボタンに `focus-visible:ring-2` クラスを含む
- [ ] ApproverSelector: 解除ボタンに `focus-visible:ring-2` クラスを含む

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### テストファイルの変更

| ファイル | 変更内容 |
|---------|---------|
| `frontend/tests/Component/ButtonTest.elm` | `baseClassTests` describe ブロックを追加。`baseClass` の文字列を直接検証 |
| `frontend/tests/Component/MessageAlertTest.elm` | フォーカスリングの存在を検証するテストを追加 |
| `frontend/tests/Component/ApproverSelectorTest.elm` | Phase 2 で追加した viewTests に含める |

Button の `baseClass` は公開されていないため、`view` で描画した HTML の class 属性を検証する方法、または `baseClass` を公開する方法がある。既存の `variantClass` が「テスト用に公開」されている前例があるため、`baseClass` も公開して直接テストする。

---

## デザインガイドライン更新

`styles.css` のコメントを更新:
- テーブルヘッダー: `text-secondary-500` → `text-secondary-600`
- ボタンのフォーカス仕様を追記: `focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-2`

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `text-secondary-500` を全箇所変更すべきかスコープが曖昧 | 曖昧 | 白背景上では AA 基準を満たす（~4.6:1）ため、テーブルヘッダー（bg-secondary-50）のみに限定。スコープ境界に明記 |
| 2回目 | Button の `baseClass` がモジュール外に非公開でテスト不可 | 既存手段の見落とし | `variantClass` と同様にテスト用に公開する前例を発見。`baseClass` を expose リストに追加 |
| 3回目 | PermissionMatrix の個別チェックボックスのラムダが actionLabel を捨てている（`( actionKey, _ )`） | 未定義 | `( actionKey, actionLabel )` に変更して aria-label の構築に使用する方針を Phase 2 に記載 |
| 4回目 | ApproverSelector の解除ボタンに aria-label がない | 不完全なパス | Phase 2 に追加。フォームラベルの Issue 完了基準「全 input 要素に適切なラベル/説明が紐づいている」にはボタンも含まれると解釈 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Grep で `text-secondary-500` の th 使用箇所を 6 ファイル確認。ラベル欠如は ApproverSelector + PermissionMatrix。フォーカス欠如は Button + MessageAlert + ApproverSelector。全対象が Phase 1-3 に含まれている |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の対象ファイル・行番号・変更内容が具体的。「必要に応じて」等の曖昧表現なし |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | コントラスト改善の色選択（600 vs 700）、ボタンの ring-offset 有無、baseClass 公開の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象外に combobox パターン、全箇所の secondary-500 変更、ダークモードを明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | WCAG 2.1 AA のコントラスト基準（4.5:1 通常 / 3:1 大文字）、Tailwind v4 の outline-none 挙動を確認済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | CORE-06（WCAG 2.1 AA SHOULD）、ADR-027（Tailwind CSS）、styles.css デザインガイドラインと整合 |
