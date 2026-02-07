# Issue #173 完了計画: 共有 UI コンポーネント + アクセシビリティ

## 概要

Epic #173 の残り 2 Story を完了し、Epic をクローズする。

- **Story A**: 共有 UI コンポーネントライブラリを構築する（Button + Badge）
- **Story B**: アクセシビリティと UX ポリッシュを追加する

## Issue 構成

2つの Story Issue を作成し、順番に実装する。

### Issue A: 共有 UI コンポーネントライブラリ（Button + Badge）

**スコープ**: 3回ルールに基づき Button（22箇所）と Badge（5箇所）のみ

### Issue B: アクセシビリティと UX ポリッシュ

**スコープ**: 実用的な改善に絞る（完全な WCAG AA 準拠は将来課題）

---

## Story A: Button + Badge コンポーネント

### Phase 1: Button コンポーネント

**新規ファイル**: `frontend/src/Component/Button.elm`

```elm
module Component.Button exposing (Variant(..), view, link)

type Variant
    = Primary
    | Success
    | Error
    | Warning
    | Outline

-- <button> 要素（type="button" をデフォルト設定）
view :
    { variant : Variant
    , disabled : Bool
    , onClick : msg
    }
    -> List (Html msg)
    -> Html msg

-- <a> 要素（リンクをボタンスタイルで描画）
link :
    { variant : Variant
    , href : String
    }
    -> List (Html msg)
    -> Html msg
```

**設計判断**:
- children を `List (Html msg)` で受け取り、テキスト以外も許容（アイコン付きボタン等）
- `disabled` は `view`（`<button>`）のみ。`link` は無効状態を持たない
- サイズは Medium（`px-4 py-2`）に統一。デザインの統一感を優先
  - Workflow/New.elm の `px-6 py-3` → `px-4 py-2` に変更（若干小さくなる）
  - Home.elm の `px-4 py-2.5` → `px-4 py-2` に変更（微差）
- 角丸は `rounded-lg` に統一（Workflow/New.elm の `rounded` を含む）
- `type_ "button"` をデフォルト設定（form 内での意図しない submit を防止）
- `cursor-pointer` を全 variant に含める（`<button>` のデフォルトは `cursor: default` のため）
- `link` 関数では `no-underline` を基本クラスに含める（`<a>` のデフォルトテキスト装飾を抑制）
- Button.elm はレイアウト（margin 等）を扱わない。`mt-2` 等のスペーシングは親要素で制御（既存コンポーネントと同じ方針）
- ConfirmDialog 内のボタンも Button.elm に移行（既に同じパターン）

**スコープ外（Button.elm で置換しない要素）**:
- `Main.elm` ハンバーガーボタン — アイコンのみ・`p-2` パディング・完全に別パターン
- `MessageAlert.elm` ×ボタン — `bg-transparent border-0`・コンポーネント内の特殊 UI

**Outline variant の border 色統一**:
- `border-secondary-300` に統一（ConfirmDialog と同じ。secondary-100 より視認性が高い）
- 影響（border が secondary-100 → secondary-300 に変わり、わずかに濃くなるファイル）:
  - `Page/Workflow/List.elm` 再読み込みボタン
  - `Page/Workflow/Detail.elm:350` エラー再読み込みボタン
  - `Page/Workflow/New.elm:757` 下書き保存ボタン
  - `Page/Task/List.elm` 再読み込みボタン
  - `Page/Task/Detail.elm:363` エラー再読み込みボタン

**テスト**: `frontend/tests/Component/ButtonTest.elm`
- 各 Variant が正しい CSS クラスを含むか（`statusToCssClass` テストと同じパターン）
  - Primary → `bg-primary-600`, `hover:bg-primary-700`
  - Success → `bg-success-600`, `hover:bg-success-700`
  - Error → `bg-error-600`, `hover:bg-error-700`
  - Warning → `bg-warning-600`, `hover:bg-warning-700`
  - Outline → `border-secondary-300`, `bg-white`

**置換対象ファイル（全 8 ファイル・合計 18 箇所）**:
- `Component/ConfirmDialog.elm`: キャンセル + 確認ボタン（2箇所）。`confirmButtonClass` 関数を削除
- `Page/Home.elm`: viewQuickActions のリンクボタン（3箇所: Success, Primary, Warning）
- `Page/Workflow/List.elm`: 新規申請リンク（2箇所: Primary）+ 再読み込みボタン（1箇所: Outline）
- `Page/Workflow/Detail.elm`: 承認/却下ボタン（2箇所: Success, Error）+ エラー再読み込みボタン（1箇所: Outline, :350）
- `Page/Workflow/New.elm`: 申請する（Primary）+ 下書き保存（Outline）= 2箇所
- `Page/Task/List.elm`: 再読み込みボタン（1箇所: Outline）
- `Page/Task/Detail.elm`: 承認/却下ボタン（2箇所: Success, Error）+ エラー再読み込みボタン（1箇所: Outline, :363）
- `Page/NotFound.elm`: 戻るリンク（1箇所: Primary）

### Phase 2: Badge コンポーネント

**新規ファイル**: `frontend/src/Component/Badge.elm`

```elm
module Component.Badge exposing (view)

view : { colorClass : String, label : String } -> Html msg
```

**設計判断**:
- 汎用的に `colorClass` を受け取る（Status 型に依存しない）
- `statusToCssClass` / `stepStatusToCssClass` は `Data.WorkflowInstance` に残す（データ層の責務）
- Badge はビュー層の責務（共通の外観構造 `rounded-full px-2.5 py-0.5 text-xs font-medium` のみ担当）

**テスト**: Badge は引数をそのまま class に埋め込むだけなので、テストは不要（コンパイル検証で十分）

**置換対象ファイル（全 4 ファイル・合計 5 箇所）**:
- `Page/Workflow/List.elm:278`: statusToCssClass によるワークフローステータスバッジ
- `Page/Workflow/Detail.elm:376`: statusToCssClass によるワークフローステータスバッジ
- `Page/Task/List.elm:195`: stepStatusToCssClass によるステップステータスバッジ
- `Page/Task/Detail.elm:389`: statusToCssClass によるワークフローステータスバッジ
- `Page/Task/Detail.elm:467`: stepStatusToCssClass によるステップステータスバッジ

---

## Story B: アクセシビリティ改善

### 既に実装済み（対応不要）
- ✅ `<html lang="ja">` — index.html
- ✅ `<main>` 要素 — Main.elm:477 (`main_`)
- ✅ `role="dialog"` + `aria-modal` — ConfirmDialog.elm
- ✅ ESC キーでダイアログ閉じ — KeyEvent.elm
- ✅ フォーカスリング — DynamicForm.elm 他
- ✅ `<label for>` + `<input id>` — DynamicForm.elm 他
- ✅ セマンティック HTML — `<nav>`, `<header>`, `<aside>`

### Phase 3: セマンティック・ARIA 改善

| 対象 | 変更 | ファイル |
|------|------|---------|
| LoadingSpinner | `role="status"` + `aria-label="読み込み中"` 追加 | Component/LoadingSpinner.elm |
| MessageAlert | 成功/エラーに `role="alert"` 追加 | Component/MessageAlert.elm |
| ハンバーガーボタン | `aria-label="メニューを開く"` 追加 | Main.elm |
| ステータスフィルタ | `<label for>` を `<select id>` と紐付け | Page/Workflow/List.elm |
| 閉じるボタン（×） | `aria-label="閉じる"` 追加 | Component/MessageAlert.elm |
| スキップリンク | `<main>` の前にスキップリンク追加 + `main_` に `id "main-content"` を付与 | Main.elm |

### Phase 4: フォーカス管理

| 対象 | 変更 | ファイル |
|------|------|---------|
| ConfirmDialog | ダイアログ表示時にキャンセルボタンへフォーカス移動（`Browser.Dom.focus` + `Cmd`） | 各ページの update 関数 + ConfirmDialog.elm にボタン `id` 追加 |
| focus → focus-visible | `focus:` を `focus-visible:` に置換（キーボードフォーカスのみ表示） | DynamicForm.elm, Page/Task/Detail.elm, Page/Workflow/Detail.elm, Page/Workflow/New.elm |

**設計判断（ダイアログフォーカスについて）**:
- HTML の `autofocus` 属性は動的に追加された要素では機能しない（ページロード時のみ）
- 正しいアプローチ: `Browser.Dom.focus "confirm-dialog-cancel"` を `Cmd` で発行。ダイアログ表示を切り替える `update` 関数で `Task.attempt` 経由でフォーカス移動する
- 影響: ConfirmDialog を使う各ページ（Workflow/Detail, Task/Detail）の `update` 関数に `Cmd` 追加が必要
- 完全なフォーカストラップは Elm では複雑（DOM API + ポート必要）なため将来の課題として記録

### Phase 5: フォーム a11y 改善

| 対象 | 変更 | ファイル |
|------|------|---------|
| フォームエラー | `aria-invalid="true"` 追加（バリデーションエラー時） | Form/DynamicForm.elm |
| フォームエラー | エラーメッセージに `id` を付与し `aria-describedby` で紐付け | Form/DynamicForm.elm |

---

## 実装順序

1. Issue A 作成 → ブランチ作成 → Draft PR（完了済み: Issue #209, PR #210）
2. **Phase 1**: Button コンポーネント（TDD: テスト → 実装 → 各ページ置換）
3. **Phase 2**: Badge コンポーネント（実装 → 各ページ置換）
4. Issue A の PR を完了 → マージ
5. Issue B 作成 → ブランチ作成 → Draft PR
6. **Phase 3**: セマンティック・ARIA 改善
7. **Phase 4**: フォーカス管理
8. **Phase 5**: フォーム a11y 改善
9. Issue B の PR を完了 → マージ
10. Epic #173 のチェックボックス更新 → クローズ

---

## 変更対象ファイル一覧

### 新規作成
- `frontend/src/Component/Button.elm`
- `frontend/src/Component/Badge.elm`
- `frontend/tests/Component/ButtonTest.elm`

### Story A で変更
- `frontend/src/Component/ConfirmDialog.elm` — Button.elm 利用、`confirmButtonClass` 削除
- `frontend/src/Page/Home.elm` — Button.link 利用
- `frontend/src/Page/Workflow/List.elm` — Button.link / Button.view + Badge.view
- `frontend/src/Page/Workflow/Detail.elm` — Button.view + Badge.view
- `frontend/src/Page/Workflow/New.elm` — Button.view（サイズ・角丸が統一される）
- `frontend/src/Page/Task/List.elm` — Button.view + Badge.view
- `frontend/src/Page/Task/Detail.elm` — Button.view + Badge.view
- `frontend/src/Page/NotFound.elm` — Button.link

### Story B で変更
- `frontend/src/Component/ConfirmDialog.elm` — キャンセルボタンに `id` 追加
- `frontend/src/Component/LoadingSpinner.elm` — a11y 属性追加
- `frontend/src/Component/MessageAlert.elm` — role="alert" + aria-label
- `frontend/src/Main.elm` — スキップリンク + `main_` に `id "main-content"` + ハンバーガー aria-label
- `frontend/src/Page/Workflow/List.elm` — label 紐付け
- `frontend/src/Page/Workflow/Detail.elm` — focus-visible + ダイアログ表示時の `Browser.Dom.focus` Cmd
- `frontend/src/Page/Workflow/New.elm` — focus-visible
- `frontend/src/Page/Task/Detail.elm` — focus-visible + ダイアログ表示時の `Browser.Dom.focus` Cmd
- `frontend/src/Form/DynamicForm.elm` — aria-invalid + aria-describedby + focus-visible

## 検証方法

1. `just check-all` — リント + テスト通過
2. `just dev-all` で開発サーバー起動し、以下を目視確認:
   - 各ページのボタンが正しく表示されること
   - バッジの色が正しいこと
   - Tab キーでフォーカスが見えること
   - ESC キーでダイアログが閉じること
   - スキップリンクが機能すること（Tab で表示 → Enter で main へ）
