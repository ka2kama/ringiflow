# デザイントークン体系化

## Context

Issue #445「デザイン・UI/UX の品質をベストプラクティス水準に引き上げる」の最初の Story。

CORE-06 要件の「タイポグラフィ統一」がギャップとして残っている。実測の結果、コード上の使用パターンは既に体系的だが、デザインシステムとしての明文化がない。本 Story では `styles.css` にデザイン語彙を明文化し、既存コードの一貫性の穴（1箇所）を修正する。

## スコープ

対象:
- `frontend/src/styles.css` のドキュメント強化（タイポグラフィ・shadow・border-radius のデザインガイドライン）
- `frontend/src/Page/Home.elm` の `rounded-xl` → `rounded-lg` 統一（1箇所）

対象外:
- Tailwind デフォルト値の `@theme` 再宣言（デフォルトと同一値の再宣言は冗長。メンテナンス負担が増すだけ）
- セマンティックトークン（`--shadow-dropdown` 等）の導入（別 Story）
- ダークモード（CORE-06 で MAY/将来）
- マージンの不統一修正（`mb-3/4/6` はページ固有レイアウト判断）
- レスポンシブブレークポイントのトークン化

## 設計判断

### デフォルト値を `@theme` で再宣言しない

| 比較 | 再宣言する | 再宣言しない（採用） |
|------|----------|-------------------|
| メリット | 意図の明示 | メンテナンスフリー、Tailwind アップデートに追従 |
| デメリット | Tailwind 更新時に値が乖離するリスク | コードから「意図」が直接見えない |
| 対策 | — | コメントで体系を文書化 |

理由: カラートークン（`--color-primary-*` 等）はプロジェクト固有の値なので `@theme` に**必要**。一方、shadow (`shadow-md/lg/xl`)、border-radius (`rounded-lg/full`)、typography (`text-2xl/lg/sm/xs`) は全て Tailwind v4 デフォルトそのまま使用しており、再宣言する理由がない。

代わりにファイルヘッダーのコメントでデザイン語彙（使用するトークン一覧とその役割）を文書化する。

### `rounded-xl` → `rounded-lg` に統一

`rounded-xl`（0.75rem=12px）は KPI カード 1 箇所のみ。他 66 箇所が `rounded-lg`（0.5rem=8px）。一貫性のため統一する。視覚的変化は軽微（角丸が 4px 小さくなる）。

## Phase 1: `styles.css` のデザインガイドライン文書化

### 変更内容

`frontend/src/styles.css` のファイルヘッダーコメントを拡充し、デザイン語彙を明文化する。`@theme` ブロック内のカテゴリコメントも整理する。

追加するガイドライン:

```css
/**
 * Tailwind CSS エントリポイント + デザイントークン
 *
 * @theme ディレクティブで CSS カスタムプロパティとしてデザイントークンを定義する。
 * 詳細: [ADR-027](../../docs/70_ADR/027_Tailwind_CSS導入.md)
 *
 * === デザインガイドライン ===
 *
 * 以下は Tailwind v4 デフォルトスケールを使用（@theme での再宣言は不要）。
 * プロジェクトのデザイン語彙として、使用するトークンとその役割を定義する。
 *
 * タイポグラフィ:
 *   ページタイトル   : text-2xl font-bold text-secondary-900
 *   セクション見出し : text-lg font-semibold text-secondary-900
 *   サブセクション   : text-sm font-semibold text-secondary-700
 *   テーブルヘッダー : text-xs font-medium uppercase tracking-wider text-secondary-500
 *   UI 要素          : text-sm font-medium
 *   KPI 数値         : text-3xl font-bold
 *
 * シャドウ:
 *   hover:shadow-md : インタラクティブカード（KPI カード）
 *   shadow-lg       : ドロップダウン、ポップオーバー
 *   shadow-xl       : モーダル、ダイアログ
 *
 * 角丸:
 *   rounded-lg   : 標準（ボタン、カード、入力、アラート、パネル）
 *   rounded-full : 丸型（バッジ、アバター、スピナー）
 *
 * テキストカラー（セマンティクス）:
 *   text-secondary-900 : プライマリテキスト（見出し、重要情報）
 *   text-secondary-700 : セカンダリテキスト（フォームラベル、説明文）
 *   text-secondary-500 : ターシャリテキスト（ヘルパー、ヘッダー）
 */
```

### 確認事項

- [x] Tailwind CSS v4 `@theme` のプロパティ名空間 → `frontend/node_modules/tailwindcss/theme.css` で確認済み。`--shadow-*`, `--radius-*`, `--text-*`, `--font-weight-*` が正確な名前空間
- [x] デフォルト値の確認 → `--shadow-md/lg/xl`, `--radius-lg` の値を theme.css L349-363 で確認済み
- [x] `@theme` でデフォルトと同一値を再宣言した場合の挙動 → オーバーライドされる（機能的に問題ないが冗長）

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

ビルド検証:
- [ ] `just check` 通過（Elm コンパイル + Vite ビルド成功）

## Phase 2: `rounded-xl` → `rounded-lg` 統一

### 変更内容

`frontend/src/Page/Home.elm` L168:

```elm
-- Before
, class ("block rounded-xl p-6 text-center no-underline transition-shadow hover:shadow-md " ++ config.bgColorClass)

-- After
, class ("block rounded-lg p-6 text-center no-underline transition-shadow hover:shadow-md " ++ config.bgColorClass)
```

### 確認事項

- [x] `rounded-xl` の使用箇所 → Grep 結果: `frontend/src/Page/Home.elm` L168 の 1 箇所のみ
- [x] 既存パターンの確認 → `rounded-lg` が 66 箇所で圧倒的主流。Button.elm の `baseClass` も `rounded-lg`

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

ビルド検証:
- [ ] `just check` 通過
- [ ] `just check-all` 通過

## 検証方法

1. `just check-all` で全テスト通過
2. `just dev-all` で開発サーバー起動し、ダッシュボード（`/`）の KPI カードが `rounded-lg` で正常表示されることを目視確認
3. `rounded-xl` が codebase 内に残っていないことを Grep で確認

## 主要ファイル

- `frontend/src/styles.css` — デザインガイドラインコメント追加
- `frontend/src/Page/Home.elm` L168 — `rounded-xl` → `rounded-lg`
- `frontend/node_modules/tailwindcss/theme.css` — デフォルト値の参照元（読み取り専用）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Plan agent が提案した shadow/radius のデフォルト再宣言は冗長 | シンプルさ | デフォルト値の再宣言をやめ、コメント文書化に変更。Tailwind v4 theme.css で実値を確認し、プロジェクト固有値（色）とデフォルト値（shadow/radius/typography）を区別 |
| 2回目 | typography を `@theme` に定義すると `--text-*--line-height` との連携が壊れるリスク | 技術的前提 | theme.css L299-324 で `--text-*` と `--text-*--line-height` のペア構造を確認。再宣言しない方針を裏付け |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | typography（6パターン）、shadow（3段階）、border-radius（2種）、テキストカラー（3段階）を全て文書化。`rounded-xl` の統一を含む |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各ガイドラインのクラス名が具体的。変更箇所がファイル名・行番号で特定されている |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | デフォルト再宣言の是非、`rounded-xl` 統一の判断理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | スコープセクションで対象 2 点、対象外 5 点を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Tailwind v4 `@theme` の挙動、`--text-*--line-height` ペア構造を theme.css で確認済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | ADR-027（Tailwind CSS 導入）の CSS-first 方針と整合。CORE-06 のタイポグラフィ統一要件に対応 |
