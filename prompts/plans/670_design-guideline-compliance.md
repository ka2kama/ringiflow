# #670 既存ビューをデザインガイドラインに準拠させる

## コンテキスト

Epic #445 でデザインガイドライン（`docs/03_詳細設計書/13_デザインガイドライン.md`）と品質レンズを整備した。しかし、策定前に書かれた既存ビューにはガイドラインとの乖離がある。新規コードのみガイドラインに従い既存コードが従わない状態は割れ窓となるため、遡及的に準拠させる。

全変更は CSS クラスの修正・コンポーネント置換であり、ロジック変更はない。

## 対象

- パンくずリストの統一（7ファイル）
- セクションカードの統一（2ファイル）
- インラインスピナーの共有コンポーネント化（1ファイル、2箇所）
- バッジ色パターンの統一（3ファイル）
- フォーカススタイルの補完（1ファイル）
- スペーシングの修正（1ファイル）

## 対象外

- ダッシュボードの拡張
- ビジュアルの洗練（ゼロ→プラス）
- 新規コンポーネントの追加
- `Data/AuditLog.elm` のデッドコード削除（`resultToCssClass` は未使用だが別 Issue で対応）

## 設計判断

### パンくずの統一パターン

現在2パターンが混在:
- Group A（Workflow.Detail, Task.Detail）: `mb-6 flex items-center gap-2 text-sm`、separator `/`
- Group B（User.*, Role.*）: `mb-4 text-sm text-secondary-500`、separator `>`

統一先: ガイドラインの `mb-4` + Group A の flex レイアウト + `/` セパレータ

```elm
nav [ class "mb-4 flex items-center gap-2 text-sm" ]
    [ a [ href ..., class "text-secondary-500 hover:text-primary-600 transition-colors" ] [ text "親ページ" ]
    , span [ class "text-secondary-400" ] [ text "/" ]
    , span [ class "text-secondary-900 font-medium" ] [ text "現在のページ" ]
    ]
```

理由:
- `mb-4` はガイドライン定義値（`mb-6` は title 用であり breadcrumb 用ではない）
- `flex items-center gap-2` で縦方向の整列が安定する
- `/` は Group B の `>` より Web 標準的（Google, GitHub 等で採用）
- `gap-2` により `mx-2` は不要（flex gap が間隔を確保）

### セクションカードの適用範囲

Workflow.Detail と Task.Detail の `viewBasicInfo`、`viewFormData`、`viewCommentSection` をカード化する。`viewSteps` は個別ステップが既にカード化されているため、外側のカードは不要（入れ子カードの複雑さを避ける）。

### バッジ色の統一基準

ガイドライン: `bg-{color}-50 text-{color}-600 border-{color}-200`

例外:
- Secondary（中性ステータス）: `bg-secondary-100 text-secondary-600 border-secondary-200`（ガイドラインの中性ステータス記載に従い `-100` を維持。`text` は `-600` に統一）
- Info: `border-info-300`（ガイドライン記載の例外、Blue 200 のコントラスト不足）

---

## Phase 1: 散在する小修正（パンくず、バッジ色、フォーカス、スペーシング）

### 確認事項

- [x] パターン: 現在のパンくずスタイル → 7ファイルで確認済み
- [x] パターン: バッジ色の使用箇所 → Grep で全箇所確認済み
- [x] ガイドライン: パンくず `mb-4`、バッジ `-50`/`-600`/`-200` → `13_デザインガイドライン.md` で確認済み

### 1-1. パンくずリストの統一

| ファイル | 現在 | 変更 |
|---------|------|------|
| `Page/Workflow/Detail.elm:695` | `mb-6 flex items-center gap-2 text-sm` | `mb-4` に修正 |
| `Page/Task/Detail.elm:335` | `mb-6 flex items-center gap-2 text-sm` | `mb-4` に修正 |
| `Page/User/Detail.elm:213` | `mb-4 text-sm text-secondary-500` + `>` | flex 構成 + `/` に統一 |
| `Page/User/New.elm:296` | 同上 | 同上 |
| `Page/User/Edit.elm:275` | 同上 | 同上 |
| `Page/Role/New.elm:240` | 同上 | 同上 |
| `Page/Role/Edit.elm:280` | 同上 | 同上 |

Group B の変更:
- `nav [ class "mb-4 text-sm text-secondary-500" ]` → `nav [ class "mb-4 flex items-center gap-2 text-sm" ]`
- `span [ class "mx-2" ] [ text ">" ]` → `span [ class "text-secondary-400" ] [ text "/" ]`
- リンク: `class "hover:text-primary-600 transition-colors"` → `class "text-secondary-500 hover:text-primary-600 transition-colors"`
- 末尾セグメント: `span [] [ text ... ]` → `span [ class "text-secondary-900 font-medium" ] [ text ... ]`

### 1-2. バッジ色パターンの統一

| ファイル | 現在 | 変更 |
|---------|------|------|
| `Data/AdminUser.elm:42` | `bg-success-100 text-success-800` | `bg-success-50 text-success-600` |
| `Data/AdminUser.elm:45,48` | `text-secondary-800` | `text-secondary-600` |
| `Page/AuditLog/List.elm:428` | `bg-success-100 text-success-800` | `bg-success-50 text-success-600` |
| `Page/AuditLog/List.elm:431` | `bg-error-100 text-error-800` | `bg-error-50 text-error-600` |
| `Page/AuditLog/List.elm:434` | `bg-secondary-100 text-secondary-800` | `text-secondary-600` |
| `Page/Role/List.elm:288` | `bg-primary-100 text-primary-800` | `bg-primary-50 text-primary-600` |
| `Page/Role/List.elm:291` | `text-secondary-800` | `text-secondary-600` |

### 1-3. フォーカススタイルの補完

`Page/Workflow/List.elm:321`:
```
"rounded-lg border border-secondary-300 bg-white px-3 py-2 text-sm"
```
→ 追加: `outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500`

### 1-4. スペーシングの修正

`Page/Home.elm:126`:
```elm
div [ class "mt-4 grid gap-4 sm:grid-cols-3" ]
```
→ `mt-4` を除去:
```elm
div [ class "grid gap-4 sm:grid-cols-3" ]
```

### テストリスト

ユニットテスト（該当なし）— CSS クラスのみの変更

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `just check`（コンパイル + lint）で破壊がないことを確認

---

## Phase 2: 詳細ページ構造の統一（セクションカード + インラインスピナー）

### 確認事項

- [x] パターン: セクションカードのスタイル → ガイドライン `rounded-lg border border-secondary-200 bg-white shadow-sm` + `p-6`
- [x] パターン: LoadingSpinner コンポーネントの使い方 → `LoadingSpinner.view`（引数なし）
- [x] パターン: User/Detail.elm の viewBasicInfo が既にカード化されている → 参照パターンとして使用

### 2-1. Workflow/Detail.elm — セクションカード化

以下の関数の最外 `div []` を `div [ class "rounded-lg border border-secondary-200 bg-white p-6 shadow-sm" ]` に変更:

| 関数 | 行 | 現在 |
|------|---|------|
| `viewBasicInfo` | 860 | `div []` |
| `viewFormData` | 1008 | `div []` |
| `viewEditableFormData` | 910 | `div []` |
| `viewCommentSection` | 1081 | `div []` |

### 2-2. Task/Detail.elm — セクションカード化

| 関数 | 行 | 現在 |
|------|---|------|
| `viewBasicInfo` | 525 | `div []` |
| `viewFormData` | 542 | `div []` |

### 2-3. Workflow/Detail.elm — インラインスピナー排除

**L1015-1018**（フォームデータ読み込み中）:
```elm
-- 現在
div [ class "flex flex-col items-center justify-center py-8" ]
    [ div [ class "h-8 w-8 animate-spin rounded-full border-4 border-secondary-100 border-t-primary-600" ] []
    , p [ class "mt-4 text-secondary-500" ] [ text "読み込み中..." ]
    ]
-- 変更後
LoadingSpinner.view
```

**L1088-1091**（コメント読み込み中）:
```elm
-- 現在
div [ class "flex items-center gap-2 py-4 text-sm text-secondary-500" ]
    [ div [ class "h-4 w-4 animate-spin rounded-full border-2 border-secondary-200 border-t-primary-600" ] []
    , text "読み込み中..."
    ]
-- 変更後
LoadingSpinner.view
```

### テストリスト

ユニットテスト（該当なし）— CSS クラスのみの変更

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `just check`（コンパイル + lint）で破壊がないことを確認

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `Data/AdminUser.elm` と `Page/Role/List.elm` にもバッジ色のずれがある | 既存手段の見落とし | Phase 1-2 の対象ファイルに追加 |
| 2回目 | `Task/Detail.elm` にも同じセクションカード不統一がある | 状態網羅漏れ | Phase 2-2 を追加 |
| 3回目 | `Data/AuditLog.elm:resultToCssClass` が未使用のデッドコード | 既存手段の見落とし | 本 Issue のスコープ外（別途対応）として対象外に記載 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 監査で発見された全乖離が計画に含まれている | OK | 6カテゴリすべてを Phase 1-2 でカバー。Grep で追加の乖離も発見し反映済み |
| 2 | 曖昧さ排除 | 各変更の before/after が具体的 | OK | 全変更箇所にファイル名・行番号・具体的な CSS クラスを記載 |
| 3 | 設計判断の完結性 | パンくずの統一パターン、バッジ色の例外が明記 | OK | 設計判断セクションに選択理由とガイドラインとの対応を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象外にダッシュボード拡張、ビジュアル洗練、デッドコード削除を記載 |
| 5 | 技術的前提 | Tailwind CSS クラスの追加のみでロジック変更なし | OK | 全変更が CSS クラスの修正 or コンポーネント置換 |
| 6 | 既存ドキュメント整合 | デザインガイドラインとの照合済み | OK | ガイドラインのパンくず・バッジ・セクションカード・フォーカスの定義と照合済み |
