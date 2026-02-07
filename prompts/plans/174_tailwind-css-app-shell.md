# Plan: #174 Integrate Tailwind CSS and implement app shell layout

## Overview

Tailwind CSS v4 を Vite + Elm ビルドパイプラインに統合し、サイドバーナビゲーション付きのレスポンシブなアプリシェルレイアウトを実装する。既存の全ページをインラインスタイルから Tailwind クラスに移行する。

## Key Design Decisions

### Tailwind v4 (not v3)

- `@tailwindcss/vite` プラグインで Vite と直接統合（PostCSS 不要）
- `@theme` ディレクティブで CSS カスタムプロパティとしてデザイントークンを定義（ダークモード対応が容易）
- ソースファイルの自動スキャン（`content` 設定不要）
- Rust ベースエンジンで高速ビルド
- ※ Issue には `tailwind.config.js` とあるが、v4 では CSS ファースト設定が推奨

### Sidebar State: Elm Model で管理

- `sidebarOpen : Bool` を Model に追加、`ToggleSidebar` / `CloseSidebar` Msg
- ページ遷移時に自動で閉じる（`UrlChanged` ハンドラ内）
- CSS-only アプローチ（checkbox hack）は避ける — ナビゲーション連携が困難

### Footer の削除

- B2B アプリシェルではフッターは不要（サイドバー + コンテンツ領域がビューポート全体を使用）
- Copyright はサイドバー最下部に移動

---

## Phase 1: Tailwind CSS Integration (Build Pipeline)

### Install Dependencies

```bash
cd frontend && pnpm add -D tailwindcss @tailwindcss/vite
```

### Files to Create

**`frontend/src/styles.css`** — Tailwind エントリポイント + デザイントークン:
```css
@import "tailwindcss";

@theme {
  --color-primary-50: #e8f0fe;
  --color-primary-100: #d2e3fc;
  --color-primary-500: #4285f4;
  --color-primary-600: #1a73e8;
  --color-primary-700: #1967d2;

  --color-secondary-50: #f1f3f4;
  --color-secondary-100: #e8eaed;
  --color-secondary-500: #5f6368;
  --color-secondary-700: #3c4043;
  --color-secondary-900: #202124;

  --color-success-50: #e6f4ea;
  --color-success-600: #34a853;
  --color-success-700: #137333;

  --color-error-50: #fce8e6;
  --color-error-600: #d93025;
  --color-error-700: #c5221f;

  --color-warning-50: #fef7e0;
  --color-warning-600: #ea8600;
  --color-warning-700: #e37400;

  --color-info-50: #e8f0fe;
  --color-info-600: #1a73e8;
}
```

### Files to Modify

**`frontend/vite.config.js`** — Add Tailwind plugin:
```js
import tailwindcss from "@tailwindcss/vite";
// plugins: [elmPlugin(), tailwindcss()]
```

**`frontend/src/main.js`** — Import CSS:
```js
import "./styles.css";  // add at top
```

**`frontend/index.html`** — Remove `<style>` block (Tailwind preflight replaces it)

### Verification
- `just dev-web` → browser で `bg-primary-600` クラスが効くことを確認
- `just check-all` 通過

---

## Phase 2: App Shell Layout + Sidebar (Main.elm)

### Responsive Layout Design

```
Desktop (≥ 1024px / lg):
+--sidebar(w-64)--+------content area------+
|  RingiFlow      |  [TopBar]              |
|                 |                        |
|  ダッシュボード   |  Page Content          |
|  申請一覧        |  (max-w-5xl mx-auto)   |
|  タスク一覧      |                        |
|  ───────────── |                        |
|  © 2026        |                        |
+-----------------+------------------------+

Mobile (< 1024px):
+------content area------+
| [≡] TopBar             |
| Page Content           |
+------------------------+

Mobile sidebar open:
+--sidebar--+--overlay--+
| (slide in)| (dimmed)  |
+-----------+-----------+
```

### Model Changes (`Main.elm`)

```elm
type alias Model =
    { ... existing fields ...
    , sidebarOpen : Bool   -- NEW
    }

type Msg
    = ... existing ...
    | ToggleSidebar        -- NEW
    | CloseSidebar         -- NEW
```

- `import Html.Events exposing (onClick)` を追加
- `init`: `sidebarOpen = False`
- `UrlChanged`: `sidebarOpen = False` を追加
- `view` を全面改修:
  - `viewSidebar`: ナビゲーションリンク + アクティブ状態表示 + `isRouteActive` ヘルパー
  - `viewTopBar`: ハンバーガーボタン（`lg:hidden`）+ ユーザー情報
  - `viewMobileOverlay`: 半透明オーバーレイ（`lg:hidden`）
  - `viewPage`: 既存の case 式を維持
  - `pageTitle`: ルートに応じた動的タイトル
- `viewHeader`, `viewFooter` を削除

### Route.elm

`isRouteActive` ヘルパーを追加（子ルートの親ルートとのマッチング）:
- `WorkflowNew`, `WorkflowDetail _` → `Workflows` がアクティブ
- `TaskDetail _` → `Tasks` がアクティブ

---

## Phase 3: Migrate Existing Pages to Tailwind

各ファイルのインラインスタイル / orphaned CSS クラスを Tailwind ユーティリティクラスに置換。

### Migration Order & Files

1. **`Data/WorkflowInstance.elm`** — `statusToCssClass` を Tailwind クラスに変更
   - `"status-draft"` → `"bg-gray-100 text-gray-600"`
   - `"status-pending"` → `"bg-warning-50 text-warning-600"`
   - etc.

2. **`Page/Home.elm`** — ~41 inline style 属性
   - KPI カード、クイックアクション、レイアウト

3. **`Page/NotFound.elm`** — ~14 inline style 属性（最小）

4. **`Page/Workflow/New.elm`** — ~104 inline style 属性（最大）
   - フォーム入力パターンを確立（DynamicForm と共通）

5. **`Form/DynamicForm.elm`** — ~37 inline style 属性
   - WorkflowNew と同じフォーム入力パターンを適用

6. **`Page/Workflow/List.elm`** — orphaned CSS クラスを置換

7. **`Page/Workflow/Detail.elm`** — orphaned CSS クラスを置換

8. **`Page/Task/List.elm`** — orphaned CSS クラスを置換

9. **`Page/Task/Detail.elm`** — orphaned CSS クラスを置換

### Common Pattern Mappings

| Old | Tailwind |
|-----|----------|
| `btn btn-primary` | `inline-flex items-center px-4 py-2 rounded-lg font-medium bg-primary-600 text-white hover:bg-primary-700 transition-colors` |
| `btn btn-secondary` | `inline-flex items-center px-4 py-2 rounded-lg font-medium border border-secondary-100 text-secondary-700 hover:bg-secondary-50 transition-colors` |
| `btn-success` | `bg-success-600 text-white hover:bg-success-700` |
| `btn-danger` | `bg-error-600 text-white hover:bg-error-700` |
| `loading` | `text-center py-8 text-secondary-500` |
| `error-message` | `p-4 bg-error-50 text-error-700 rounded-lg` |
| `page-header` | `flex items-center justify-between mb-6` |

### Human Collaboration Point

KPI カードのデザイン（`Page/Home.elm`）は既存 TODO(human) があるため、ここで Learn by Doing を実施する。

---

## Phase 4: ADR-027

**Create**: `docs/05_ADR/027_Tailwind_CSS導入.md`

- Context: MVP 完了後の UI/UX 改善、CORE-06 デザインシステム要件
- Options: Tailwind v3, Tailwind v4, elm-css, Plain CSS with BEM
- Decision: Tailwind v4 + `@tailwindcss/vite`
- Consequences: 型安全性の議論（コンポーネント境界で確保）、ビルドパイプラインへの影響

---

## Phase 5: Commit & Verification

### Commit Strategy（Phase ごと）

1. `Add Tailwind CSS v4 with design tokens` — Phase 1
2. `Implement sidebar navigation and app shell layout` — Phase 2
3. `Migrate pages from inline styles to Tailwind` — Phase 3（ファイル単位でも可）
4. `Add ADR-027 for Tailwind CSS adoption` — Phase 4

### Verification Checklist

- [ ] `just check-all` passes
- [ ] All pages render correctly with Tailwind styles
- [ ] Sidebar navigation: click each item → correct page loads, active state shown
- [ ] Responsive (< 1024px): hamburger appears, sidebar hidden
- [ ] Mobile menu: hamburger → sidebar slides in with overlay → click nav item → sidebar closes
- [ ] No regression on existing functionality (workflow CRUD, task approval)

---

## Critical Files

| File | Action |
|------|--------|
| `frontend/src/styles.css` | **CREATE** — Tailwind entry + design tokens |
| `frontend/vite.config.js` | MODIFY — Add `@tailwindcss/vite` plugin |
| `frontend/src/main.js` | MODIFY — `import "./styles.css"` |
| `frontend/index.html` | MODIFY — Remove inline `<style>` |
| `frontend/src/Main.elm` | **MAJOR REWRITE** — App shell, sidebar, Model/Msg changes |
| `frontend/src/Route.elm` | MODIFY — Add `isRouteActive` helper |
| `frontend/src/Data/WorkflowInstance.elm` | MODIFY — `statusToCssClass` → Tailwind classes |
| `frontend/src/Page/Home.elm` | MODIFY — Inline styles → Tailwind |
| `frontend/src/Page/NotFound.elm` | MODIFY — Inline styles → Tailwind |
| `frontend/src/Page/Workflow/New.elm` | MODIFY — Inline styles → Tailwind |
| `frontend/src/Form/DynamicForm.elm` | MODIFY — Inline styles → Tailwind |
| `frontend/src/Page/Workflow/List.elm` | MODIFY — CSS classes → Tailwind |
| `frontend/src/Page/Workflow/Detail.elm` | MODIFY — CSS classes → Tailwind |
| `frontend/src/Page/Task/List.elm` | MODIFY — CSS classes → Tailwind |
| `frontend/src/Page/Task/Detail.elm` | MODIFY — CSS classes → Tailwind |
| `docs/05_ADR/027_Tailwind_CSS導入.md` | **CREATE** — ADR |
