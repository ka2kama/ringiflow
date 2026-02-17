# Issue #628 コンポーネントのビジュアルポリッシュ

## Context

Epic #445 の Story 7。#627（カラーパレット刷新）と #612（フォント導入）で整えたデザイン基盤の上に、コンポーネントレベルの磨きを加える。

現状の主な課題:
- カード: shadow / border の使い方が不統一。セクションカードに shadow なし
- テーブル: User List のみ洗練されたスタイル。Task/Workflow List は素朴
- Badge: 旧パレット（`gray-*`, `green-*`, `red-*`）が残存。ボーダーなし
- フォーム: `focus:` と `focus-visible:` の混在
- 余白: `px-2.5`, `py-2.5`, `py-1.5` が 4px グリッドから外れる

## スコープ

対象: Badge, FormField, テーブル3ページ, KPI カード, 詳細ページセクションカード, デザインガイドライン
対象外: Button.elm（統一済み）、ロジック変更、新規コンポーネント追加

---

## Phase 1: 旧パレット排除 + Badge ポリッシュ

波及範囲が最も広いため最初に実施。

### 対象ファイル

- `frontend/src/Component/Badge.elm` (L33)
- `frontend/src/Data/WorkflowInstance.elm` (L219-260)
- `frontend/src/Data/AuditLog.elm` (L133-143)
- `frontend/src/Data/AdminUser.elm` (L38-48)
- `frontend/src/Page/AuditLog/List.elm` (L425-435)
- `frontend/src/Page/Role/List.elm` (L289-295)
- `frontend/src/Page/User/Detail.elm` (L352)

### 変更内容

**Badge.elm (L33)**: `border` 追加 + `px-2.5` → `px-2`（4px グリッド準拠）

```elm
-- As-Is
"inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium "
-- To-Be
"inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium "
```

`py-0.5`（2px）はバッジの行内要素として視覚的に適切な最小値。Tailwind UI でも標準的であり変更しない。

**旧パレット排除 + ボーダー色追加**: Badge に `border` を追加するため、全 colorClass にボーダー色を含める。

WorkflowInstance.elm statusToCssClass:

| Status | As-Is | To-Be |
|--------|-------|-------|
| Draft | `bg-gray-100 text-gray-600` | `bg-secondary-100 text-secondary-600 border-secondary-200` |
| Pending | `bg-warning-50 text-warning-600` | `bg-warning-50 text-warning-600 border-warning-200` |
| InProgress | `bg-info-50 text-info-600` | `bg-info-50 text-info-600 border-info-300` |
| Approved | `bg-success-50 text-success-600` | `bg-success-50 text-success-600 border-success-200` |
| Rejected | `bg-error-50 text-error-600` | `bg-error-50 text-error-600 border-error-200` |
| Cancelled | `bg-secondary-100 text-secondary-500` | `bg-secondary-100 text-secondary-500 border-secondary-200` |
| ChangesRequested | `bg-warning-50 text-warning-600` | `bg-warning-50 text-warning-600 border-warning-200` |

WorkflowInstance.elm stepStatusToCssClass:

| StepStatus | As-Is | To-Be |
|------------|-------|-------|
| StepPending | `bg-gray-100 text-gray-600` | `bg-secondary-100 text-secondary-600 border-secondary-200` |
| StepActive | `bg-warning-50 text-warning-600` | `bg-warning-50 text-warning-600 border-warning-200` |
| StepCompleted | `bg-success-50 text-success-600` | `bg-success-50 text-success-600 border-success-200` |
| StepSkipped | `bg-secondary-100 text-secondary-500` | `bg-secondary-100 text-secondary-500 border-secondary-200` |

AuditLog.elm resultToCssClass:

| Result | As-Is | To-Be |
|--------|-------|-------|
| success | `bg-green-100 text-green-800` | `bg-success-100 text-success-800 border-success-200` |
| failure | `bg-red-100 text-red-800` | `bg-error-100 text-error-800 border-error-200` |
| _ | `bg-gray-100 text-gray-800` | `bg-secondary-100 text-secondary-800 border-secondary-200` |

AuditLog/List.elm resultToBadge:

| Result | As-Is | To-Be |
|--------|-------|-------|
| success | `bg-success-100 text-success-800` | `bg-success-100 text-success-800 border-success-200` |
| failure | `bg-error-100 text-error-800` | `bg-error-100 text-error-800 border-error-200` |
| _ | `bg-secondary-100 text-secondary-800` | `bg-secondary-100 text-secondary-800 border-secondary-200` |

AdminUser.elm statusToBadge:

| Status | As-Is | To-Be |
|--------|-------|-------|
| active | `bg-success-100 text-success-800` | `bg-success-100 text-success-800 border-success-200` |
| inactive | `bg-secondary-100 text-secondary-800` | `bg-secondary-100 text-secondary-800 border-secondary-200` |
| _ | `bg-secondary-100 text-secondary-800` | `bg-secondary-100 text-secondary-800 border-secondary-200` |

Role/List.elm typeToBadge:

| Type | As-Is | To-Be |
|------|-------|-------|
| system | `bg-primary-100 text-primary-800` | `bg-primary-100 text-primary-800 border-primary-200` |
| custom | `bg-secondary-100 text-secondary-800` | `bg-secondary-100 text-secondary-800 border-secondary-200` |

User/Detail.elm ロールバッジ (L352):
- As-Is: `bg-primary-100 text-primary-800`
- To-Be: `bg-primary-100 text-primary-800 border-primary-200`

ボーダー色ルール: 背景色と同系統の 200 番台。info のみ `info-200` が @theme に未定義のため `info-300`（styles.css L93 で定義確認済み）。

### 確認事項
- [x] Badge.view の全呼び出し箇所を Grep で確認 → 上記のリストが網羅的か
- [x] info-300 が @theme に存在するか → styles.css L93 `--color-info-300: #93c5fd` 確認済み

### テストリスト

ユニットテスト: 該当なし（CSS クラス文字列の変更のみ）

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: `just check-all` で既存テストが壊れないことを確認

---

## Phase 2: テーブルの統一

User/List.elm をベースラインとし、Task/List.elm と Workflow/List.elm を揃える。

### 対象ファイル

- `frontend/src/Page/Task/List.elm` (L167-207)
- `frontend/src/Page/Workflow/List.elm` (L281, L377-408)

### ベースライン（User/List.elm のパターン）

```
ラッパー: div [ class "overflow-x-auto rounded-lg border border-secondary-200" ]
table:   table [ class "w-full" ]
thead:   thead [ class "bg-secondary-50" ]
th:      th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ]
tbody:   tbody [ class "divide-y divide-secondary-200 bg-white" ]
tr:      tr [ class "hover:bg-secondary-50 transition-colors" ]
```

styles.css のデザインガイドラインで `text-xs font-medium uppercase tracking-wider text-secondary-500` がテーブルヘッダーの標準と定義されている。

### 変更内容

**Task/List.elm:**

viewTaskList (L167-171): テーブルの外側にラッパー追加
```elm
-- ラッパー追加
div [ class "overflow-x-auto rounded-lg border border-secondary-200" ]
    [ viewTaskTable zone tasks ]
```

viewTaskTable (L174-188):
```elm
-- thead: "border-b border-secondary-100" → "bg-secondary-50"
-- th: "text-sm font-medium" → "text-xs font-medium uppercase tracking-wider"
-- tbody: クラスなし → "divide-y divide-secondary-200 bg-white"
```

viewTaskRow (L191): `"border-b border-secondary-100"` → `"hover:bg-secondary-50 transition-colors"`

**Workflow/List.elm:**

viewWorkflowList (L281 付近): テーブルの外側にラッパー追加
```elm
div [ class "overflow-x-auto rounded-lg border border-secondary-200" ]
    [ viewWorkflowTable zone filteredWorkflows ]
```

viewWorkflowTable (L377-390):
```elm
-- table: "w-full border-collapse" → "w-full"（border-collapse 削除）
-- thead: "border-b border-secondary-100" → "bg-secondary-50"
-- th: "text-sm font-medium" → "text-xs font-medium uppercase tracking-wider"
-- tbody: クラスなし → "divide-y divide-secondary-200 bg-white"
```

viewWorkflowRow (L393): `"border-b border-secondary-100"` → `"hover:bg-secondary-50 transition-colors"`

### 確認事項
- [x] viewTaskList と viewWorkflowList のテーブル外側の DOM 構造を確認 → ラッパー追加位置の特定

### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: `just check-all` で確認

---

## Phase 3: KPI カード + 詳細セクションカードのポリッシュ

### 対象ファイル

- `frontend/src/Page/Home.elm` (L165-174)
- `frontend/src/Page/User/Detail.elm` (L259, L339)
- `frontend/src/Page/Workflow/Detail.elm` (L780, L897, L1121, L1226, L1334)
- `frontend/src/Page/Task/Detail.elm` (L410, L584)

### 変更内容

**KPI カード (Home.elm L165):**
```elm
-- As-Is
"block rounded-lg p-6 text-center no-underline transition-shadow hover:shadow-md "
-- To-Be
"block rounded-lg border border-secondary-200 p-6 text-center no-underline shadow-sm transition-shadow hover:shadow-md "
```

変更点: `border border-secondary-200` + `shadow-sm` 追加（常時の微細な浮遊感）

**セクションカード共通パターン:**

| 区分 | As-Is | To-Be |
|------|-------|-------|
| セクションカード | `rounded-lg border border-secondary-200 bg-white p-6` | `rounded-lg border border-secondary-200 bg-white p-6 shadow-sm` |
| セクションカード (p-4) | `rounded-lg border border-secondary-100 p-4` | `rounded-lg border border-secondary-200 bg-white p-4 shadow-sm` |
| 入れ子カード | `rounded-lg border border-secondary-100 p-4` | `rounded-lg border border-secondary-200 bg-white p-4` |
| 入れ子カード (p-3) | `rounded-lg border border-secondary-100 p-3` | `rounded-lg border border-secondary-200 bg-white p-3` |

設計判断:
- ボーダー色統一: `secondary-100` → `secondary-200`（User/Detail.elm がベースライン。より視認性の高い区切り）
- `shadow-sm`: セクションレベルのカードのみ。入れ子カードにシャドウを付けると視覚的に重くなる
- `bg-white`: 明示的に指定し、背景色を確保

具体的な変更箇所:

| ファイル | 行 | 種別 | 変更 |
|---------|-----|------|------|
| User/Detail.elm | L259 | セクション | `shadow-sm` 追加 |
| User/Detail.elm | L339 | セクション | `shadow-sm` 追加 |
| Workflow/Detail.elm | L780 | セクション | `secondary-100→200` + `bg-white shadow-sm` 追加 |
| Workflow/Detail.elm | L1226 | セクション | `secondary-100→200` + `bg-white shadow-sm` 追加 |
| Workflow/Detail.elm | L1334 | 入れ子 | `secondary-100→200` + `bg-white` 追加 |
| Workflow/Detail.elm | L1121 | 入れ子 | `secondary-100→200` + `bg-white` 追加 |
| Workflow/Detail.elm | L897 | セマンティック | 変更なし（warning ボーダー/背景は意図的） |
| Task/Detail.elm | L410 | セクション | `secondary-100→200` + `bg-white shadow-sm` 追加 |
| Task/Detail.elm | L584 | 入れ子 | `secondary-100→200` + `bg-white` 追加 |

### 確認事項
- [x] `shadow-sm` が Tailwind v4 デフォルトスケールに含まれるか → 含まれる（@theme 再宣言不要）
- [x] Workflow/Detail.elm の全カードパターンの網羅性 → viewStepProgress, viewApprovalSection, viewStep, viewCommentItem, viewResubmitSection の 5 箇所

### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: `just check-all` で確認

---

## Phase 4: フォームスタイル統一 + 余白グリッド

### 対象ファイル

- `frontend/src/Component/FormField.elm` (L46, L95)
- `frontend/src/Main.elm` (L844)
- `frontend/src/Page/Workflow/List.elm` (L325)

### 変更内容

**FormField.elm - フォーカスリング統一 (L46 inputClass):**

```elm
-- As-Is
"w-full rounded-lg border px-3 py-2 text-sm "
    ++ "border-secondary-300 focus:border-primary-500 focus:ring-primary-500"
-- To-Be
"w-full rounded-lg border px-3 py-2 text-sm outline-none "
    ++ "border-secondary-300 focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:border-primary-500"
```

エラー時も同様に `focus:` → `focus-visible:ring-2 focus-visible:`。

`focus-visible` はキーボードフォーカス時のみリングを表示する現代的なベストプラクティス。Task/Detail.elm L425 で既に使用されているパターン。

**FormField.elm - viewTextArea (L95):**
同様に `focus:` → `focus-visible:ring-2 focus-visible:` + `outline-none` 追加。

**Main.elm - サイドバーナビ (L844):**
```elm
-- As-Is
"flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors "
-- To-Be
"flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors "
```

`py-2.5`（10px）→ `py-2`（8px）: 4px グリッド準拠。

**Workflow/List.elm - フィルタセレクト (L325):**
```elm
-- As-Is
"rounded border border-secondary-100 bg-white px-3 py-1.5 text-sm"
-- To-Be
"rounded-lg border border-secondary-300 bg-white px-3 py-2 text-sm"
```

変更点: `rounded` → `rounded-lg`（ガイドライン統一）、`border-secondary-100` → `border-secondary-300`（FormField と統一）、`py-1.5`（6px）→ `py-2`（8px）

### 確認事項
- [x] FormField.elm の全フォーカスパターン → inputClass（viewTextField/viewSelectField 共用）と viewTextArea の計 2 箇所。viewReadOnlyField はフォーカス不要
- [x] Main.elm のサイドバーナビの親要素の余白 → `py-2` 変更後のレイアウト確認

### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: `just check-all` で確認

---

## Phase 5: デザインガイドライン更新

### 対象ファイル

- `frontend/src/styles.css` (L7-38 コメント部分)

### 変更内容

既存のシャドウ・角丸セクションを拡張し、テーブル・バッジ・フォーム・カードのスタイル定義を追加。

```
シャドウ（既存を拡張）:
  shadow-sm     : セクションカード（詳細ページの情報カード）  ← 追加
  hover:shadow-md : インタラクティブカード（KPI カード）
  shadow-lg       : ドロップダウン、ポップオーバー
  shadow-xl       : モーダル、ダイアログ

テーブル（新規追加）:
  ラッパー     : overflow-x-auto rounded-lg border border-secondary-200
  thead        : bg-secondary-50
  th           : （既存のテーブルヘッダー定義を参照）
  tbody        : divide-y divide-secondary-200 bg-white
  tr(データ行) : hover:bg-secondary-50 transition-colors

バッジ（新規追加）:
  構造    : inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium
  色指定  : bg-{color}-{shade} text-{color}-{shade} border-{color}-{shade}

フォーム入力（新規追加）:
  構造    : w-full rounded-lg border px-3 py-2 text-sm outline-none
  フォーカス : focus-visible:ring-2 focus-visible:ring-{color}-500 focus-visible:border-{color}-500
  ボーダー(通常) : border-secondary-300
  ボーダー(エラー) : border-error-300

セクションカード（新規追加）:
  セクション : rounded-lg border border-secondary-200 bg-white shadow-sm + p-4 or p-6
  入れ子     : rounded-lg border border-secondary-200 bg-white + p-3 or p-4 (shadow なし)
```

### 確認事項
- [x] 既存のデザインガイドラインコメントとの整合性

### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: CSS コメントのみ。`just check-all` で最終確認

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | AuditLog.elm に `bg-green/bg-red`（旧 Tailwind カラー）が残存。Issue の As-Is では `bg-gray` のみ指摘 | 既存手段の見落とし | Phase 1 に AuditLog.elm の旧パレット排除を追加 |
| 2回目 | Badge に `border` を追加すると全 colorClass にボーダー色が必要。Grep で 7 ファイル 16 箇所を特定 | 不完全なパス | Phase 1 に全呼び出し箇所のボーダー色追加を網羅的に記載 |
| 3回目 | FormField の `focus:` と Task/Detail の `focus-visible:` が不統一 | アーキテクチャ不整合 | Phase 4 に FormField のフォーカスリング統一を追加 |
| 4回目 | Workflow/List.elm のフィルタセレクトが `rounded`（`rounded-lg` ではない）、`border-secondary-100` で不整合 | アーキテクチャ不整合 | Phase 4 にフィルタセレクトの統一を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了基準 6 項目が全て計画に含まれている | OK | カード(Phase 3), テーブル(Phase 2), フォーム(Phase 4), Badge(Phase 1), 余白(Phase 1,4), check-all(各Phase) |
| 2 | 曖昧さ排除 | 全ての変更が具体的な Tailwind クラスで指定されている | OK | 各 Phase の As-Is/To-Be テーブルで全クラスを明示 |
| 3 | 設計判断の完結性 | 全てのスタイル差異に統一方針が記載されている | OK | テーブル(User/List ベースライン), ボーダー色(secondary-200統一), Badge(border+200番台ルール), フォーカスリング(focus-visible統一) |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 冒頭のスコープセクションで明示 |
| 5 | 技術的前提 | Tailwind v4 のクラスが正しい | OK | focus-visible: Task/Detail.elm L425 で既使用、shadow-sm: Tailwind デフォルト、info-300: styles.css L93 で定義 |
| 6 | 既存ドキュメント整合 | styles.css のデザインガイドラインと矛盾がない | OK | shadow 階層（sm < md < lg < xl）は整合的。テーブルヘッダーは既存ガイドライン L16 と一致 |

## 検証方法

1. 各 Phase 完了後に `cd frontend && pnpm run build` で Elm コンパイル確認
2. 全 Phase 完了後に `just check-all` で lint + test + API test + E2E test
3. 開発サーバー（`just dev-all`）でビジュアル確認（ダッシュボード、一覧ページ、詳細ページ、申請フォーム）
