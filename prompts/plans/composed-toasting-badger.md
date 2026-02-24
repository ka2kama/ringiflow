# #890 ダッシュボード KPI カードのデザイン改善

## Context

ダッシュボード実装時に `TODO(human)` として残された KPI カードのデザインタスク。基本的なカード構造（グリッド、ボーダー、シャドウ、色分け）は実装済みだが、デザインガイドラインとの突合・改善が必要。

Issue 精査: 続行（Issue コメントに記録済み）

## スコープ

対象:
- `frontend/src/Page/Home.elm` — KPI カードの view 関数改善、TODO 削除
- `frontend/src/Component/Icons.elm` — `checkCircle` アイコン新規追加

対象外:
- バックエンド変更（なし）
- グリッド構造やレスポンシブ設定の変更（既にガイドライン準拠）
- E2E テスト新規追加（スコープ外）

## 設計判断

### 1. アイコン選定

各 KPI カードに意味を伝えるアイコンを追加する。

| KPI カード | アイコン | 根拠 |
|-----------|---------|------|
| 承認待ちタスク | `Icons.tasks`（既存） | チェックリストアイコン。サイドバーと同じ意味 |
| 申請中 | `Icons.workflows`（既存） | ドキュメントアイコン。サイドバーと同じ意味 |
| 本日完了 | `Icons.checkCircle`（新規） | チェック付き円。既存 `tasks`（チェック + 四角）と視覚的に区別 |

### 2. アイコンサイズ: `h-5 w-5` に統一

選択肢:
- A) **`h-5 w-5` 統一（採用）** — 既存アイコンをそのまま流用。新規 `checkCircle` も同サイズ。同一行のカード間で一貫性を保つ
- B) `h-6 w-6` 統一 — 既存アイコンのサイズ変更が必要。API 変更 or バリアント追加で複雑化
- C) サイズ混在を許容 — 同じカード行で 4px の差。視覚的に不自然

`h-5 w-5`（20px）は `text-3xl`（30px）の数値に対して適切な補助サイズ。アイコンは識別要素であり、数値が主役（デザイン原則「最小の強調」）。

### 3. カードレイアウト: 中央揃え + アイコン上部配置

```
┌──────────────────┐
│       [icon]     │  h-5 w-5, textColorClass
│        42        │  text-3xl font-bold
│  承認待ちタスク    │  text-sm text-secondary-500
└──────────────────┘
```

既存の `text-center` を活かし、アイコンを数値の上に配置。SVG の `stroke="currentColor"` により `textColorClass` の色が自動継承される。

### 4. トランジション: `transition-shadow` → `transition-colors`

デザインガイドライン:「`transition-colors` — hover を持つ全要素に付与する」。プロジェクト全体で `transition-colors` が標準。`transition-shadow` の使用は KPI カードのみ。

hover 時のシャドウ変化（`shadow-sm` → `shadow-md`）のアニメーションは失われるが、ガイドライン準拠を優先。

## Phase 1: KPI カードデザイン改善

### 確認事項

- 型: `viewStatCardLink` の config レコード型 → `frontend/src/Page/Home.elm:157-164`
- パターン: Icons モジュールのアイコン定義パターン（SVG 構造、属性） → `frontend/src/Component/Icons.elm`
- パターン: `transition-colors` の使用 → プロジェクト全体で標準使用を確認済み
- ライブラリ: `Svg` / `Svg.Attributes` → `Component/Icons.elm` の既存パターン（`elm/svg 1.0.1`）

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ダッシュボードを表示し、KPI カードにアイコン・スタイルが表示される | 正常系 | 手動確認 |
| 2 | KPI カードをホバーし、トランジション表現を確認 | 正常系 | 手動確認 |
| 3 | KPI カードをクリックし、対応ページに遷移する | 正常系 | 手動確認（既存動作） |

### テストリスト

ユニットテスト（該当なし）: ビュー層のスタイル・レイアウト変更のみ。ロジック変更なし
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）: デザイン変更のみ、既存 E2E なし

### 実装手順

1. `Component/Icons.elm` に `checkCircle` アイコンを追加（`h-5 w-5`、Lucide Icons の CircleCheck 参照）
2. `Page/Home.elm` に `Component.Icons` の import を追加
3. `viewStatCardLink` の config 型に `icon : Html Msg` フィールドを追加
4. `viewStatCardLink` のビューにアイコン描画を追加、`transition-shadow` → `transition-colors` に変更
5. `viewStatsCards` の各呼び出しに `icon` フィールドを追加
6. L114 の `TODO(human)` コメントを削除
7. `just check` でコンパイル・lint 確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Plan エージェントが `checkCircle` を `h-6 w-6` で提案したが、既存アイコン（`h-5 w-5`）との混在は同一カード行で不整合 | 競合・エッジケース | 全アイコンを `h-5 w-5` に統一。シンプルさと視覚的一貫性を優先 |
| 2回目 | `transition-shadow` と `transition-colors` の Tailwind v4 での共存が `transition-property` 競合を起こす | 競合・エッジケース | `transition-colors` のみに統一。ガイドライン準拠 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 完了基準 2 項目（デザイン改善、TODO 削除）が実装手順に対応。変更ファイル 2 つ網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | アイコン選定・サイズ・レイアウト・トランジションの全判断に根拠あり |
| 3 | 設計判断の完結性 | 全差異に判断が記載 | OK | 4 つの設計判断に選択肢・理由・トレードオフを記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象 2 ファイル、対象外 3 項目を明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | SVG `stroke="currentColor"` による色継承、Tailwind `transition-property` 競合を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | デザインガイドラインと照合済み |

## 検証

- `just check` でコンパイル・lint 通過を確認
- 開発サーバー（`just dev-all`）でダッシュボードを目視確認（手動）
