# 手順書の AI 最適化

## Context

手順書（`docs/04_手順書/04_開発フロー/01_Issue駆動開発.md` (50KB) と `02_TDD開発フロー.md` (36KB)）が人間向けの書き方で、Claude Code が Issue 作業のたびに Read で 86KB を消費している。AI 最適化版を `.claude/rules/` に作成し、Read を不要にする。

**トークンコスト:**

| 項目 | 現状 | 変更後 |
|------|------|--------|
| `.claude/rules/` 自動注入 | 131KB | ~153KB (+22KB) |
| CLAUDE.md 自動注入 | 15KB | ~14KB (-1KB) |
| 手順書 Read（Issue作業時） | 86KB | 0KB |
| **Issue作業セッション合計** | **232KB** | **~167KB (-65KB)** |

## スコープ

**対象:**
- `.claude/rules/dev-flow-issue.md` 新規作成
- `.claude/rules/dev-flow-tdd.md` 新規作成
- `docs/04_手順書/04_開発フロー/01_Issue駆動開発.md` 修正
- `docs/04_手順書/04_開発フロー/02_TDD開発フロー.md` 修正
- 参照の更新（24箇所）

**対象外:**
- `prompts/runs/`, `prompts/plans/`, `process/improvements/` の歴史的記録（リンク切れ許容）
- `README.ja.md`, `README.md`（人間向け、docs を参照し続ける）
- `zoom-rhythm.md` 自体のリファクタリング（参照先の更新のみ）

## 設計判断

- **paths 指定なし**: 開発フロー全般に適用されるため常時ロード
- **命名**: `dev-flow-` プレフィックスで2ファイルをグループ化
- **アンカー互換**: 既存 docs のアンカーは維持（外部リンク互換性）。docs に「AI版は rules を参照」の注記を追加
- **ADR 不要**: ドキュメント構造の変更であり技術的設計判断ではない

---

## Phase 1: `dev-flow-issue.md` の作成

Issue駆動開発の AI ディレクティブ版。目標サイズ: 12-14KB。

#### 確認事項
- パターン: ディレクティブ形式 → `troubleshooting.md`, `pre-implementation.md` を参照
- 型: 品質チェックリスト 6.2 の全カテゴリ構成 → `01_Issue駆動開発.md` L429-L514

#### 含めるもの

| セクション | 元のソース |
|-----------|-----------|
| AI着手トリガー | L36-41 |
| Step 1: Issue精査（観点テーブル、As-Is検証、結果分岐、記録フォーマット） | L43-159 |
| Step 2: ブランチ作成（命名規則、main最新化） | L161-179 |
| Step 3: Draft PR作成（コマンド、PR本文ルール） | L181-219 |
| Step 4: 設計（4.1-4.5、収束確認、簡略化条件） | L220-355 |
| Step 5: 実装（コミット粒度、Issue進捗更新） | L356-415 |
| Step 6: 品質ゲート（6.1-6.8、品質CL 6.2全項目、品質確認6.3、計画ファイル確認6.4、base同期6.5、再Ready 6.9） | L416-627 |
| Step 7-8: レビュー確認、マージ | L629-655 |
| Step 9: 振り返り（フォーマット、転記基準、改善記録検証、TODO棚卸し） | L656-764 |
| Epic/Story運用（分解基準、テスト責任マッピング、ブランチ命名、PR紐付け、タスクリスト管理、統合検証） | L822-1024 |

#### 除外するもの（docsに残す or 削除）

- 変更履歴 L1083-1120 → 削除
- 教育的説明（「なぜ都度更新が重要か」等）→ docs に残す
- Mermaid図 → テキスト表現に圧縮
- Milestone/Label/Project Board → docs に残す
- 「よく使うコマンド」→ docs に残す
- 「運用補足」(Assignee, Project紐づけ) → docs に残す
- ADR採用理由リンク → docs に残す
- 完了基準の良い例/悪い例の教育解説 → docs に残す
- 冗長なコード例 → 最小テンプレートのみ

#### 変換ルール

- 教育的説明 → 削除
- Mermaid図 → `Step 1 → Step 2 → ...` テキスト or 削除
- 冗長なコード例 → 最小テンプレート1つ
- 条件分岐の文章 → テーブル
- 推奨「〜すること」→ ゲート条件 or 禁止事項

---

## Phase 2: `dev-flow-tdd.md` の作成

TDD開発フローの AI ディレクティブ版。目標サイズ: 8-11KB。

#### 確認事項
- パターン: Phase 1 の dev-flow-issue.md との相互参照整合
- 型: 設計原則レンズの全テーブル構成 → `02_TDD開発フロー.md` L196-L260

#### 含めるもの

| セクション | 元のソース |
|-----------|-----------|
| 確認事項の実施（ゲート条件） | L28-46 |
| Red: 二層Redモデル、コンパイルエラー解消原則（書く/書かないテーブル） | L50-137 |
| Green: 最短で通す原則 | L139-154 |
| Refactor: 設計原則レンズ全テーブル + UI/UXレンズ全テーブル | L156-261 |
| 操作パスの列挙（適用条件、手順、分類、テスト層変換） | L278-346 |
| テストリスト要件（テスト層明記、役割テーブル） | L265-436 |
| テスト設計の方向性（トップダウン/ボトムアップ） | L338-347 |
| MVP積み上げ方式の原則 | L449-460 |
| テストレビュー（Phase完了時確認観点） | L497-511 |
| E2Eテスト実行タイミング | L636-659 |
| テスト規約（sut命名） | L583-613 |

#### 除外するもの

- 変更履歴 L708-724 → 削除
- 参考資料・出典 L676-687 → docs に残す
- 既知手法との対応表 L689-704 → docs に残す
- 実践的パターン（三角測量、AAA、テストダブル）詳細例 → docs に残す
- TDD採用理由テーブル L10-16 → docs に残す
- ワークフロー全体像 Mermaid → 削除
- チェックリスト L663-672 → 品質CL と重複、削除
- 冗長なコード例 → 大幅圧縮

---

## Phase 3: 既存手順書の修正

### 3a: `01_Issue駆動開発.md`

1. 冒頭に注記追加:
   ```
   > AI 最適化版: [`.claude/rules/dev-flow-issue.md`](../../../.claude/rules/dev-flow-issue.md)
   ```
2. 変更履歴 L1083-1120 を削除
3. 手順ステップの主要セクション冒頭に注記追加（「AI版は rules を参照」）
4. 残す: 概要、Mermaid図、教育的解説、Milestone/Label/Project Board、コマンド集、運用補足

### 3b: `02_TDD開発フロー.md`

1. 冒頭に注記追加（同上形式）
2. 変更履歴 L708-724 を削除
3. 手順ステップの主要セクション冒頭に注記追加
4. 残す: 概要、Mermaid図、参考資料、既知手法との対応、実践的パターン

---

## Phase 4: 参照の更新（24箇所）

### 4a: CLAUDE.md（7箇所）

| 行 | 現在の参照 | 変更 |
|----|-----------|------|
| L72 | `docs/.../02_TDD開発フロー.md#設計原則レンズ` | `.claude/rules/dev-flow-tdd.md#設計原則レンズ` |
| L114 | 同上 | 同上 |
| L206 | `機能実装前に必ず [手順書](...) を読み...` | 削除（rules 自動注入で不要） |
| L208 | `**禁止:** 手順書を読まずに...` | 削除 |
| L210 | `[TDD 開発フロー](...) に従い...` | 削除 |
| L255 | `→ 詳細手順: [手順書: Issue 駆動開発](...)` | `→ 詳細: dev-flow-issue.md` |
| L280-281 | 手順書 #3, #6 への参照 | dev-flow-issue.md の対応セクションへ |

### 4b: `.claude/rules/zoom-rhythm.md`（8箇所）

| 行 | 現在 | 変更先 |
|----|------|--------|
| L84 | `02_TDD開発フロー.md#-refactor-設計を改善する` | `dev-flow-tdd.md#refactor-設計を改善する` |
| L90 | `01_Issue駆動開発.md#62-品質チェックリスト` | `dev-flow-issue.md#品質チェックリスト` |
| L136 | `02_TDD開発フロー.md#設計原則レンズ` | `dev-flow-tdd.md#設計原則レンズ` |
| L138 | `01_Issue駆動開発.md#62-品質チェックリスト` | `dev-flow-issue.md#品質チェックリスト` |
| L149 | 同上 | 同上 |
| L161 | 同上 | 同上 |
| L170 | `01_Issue駆動開発.md#6-品質ゲートと-ready-for-review` | `dev-flow-issue.md#品質ゲート` |
| L203 | `02_TDD開発フロー.md#操作パスの列挙テストリスト作成の前に` | `dev-flow-tdd.md#操作パスの列挙` |

### 4c: `.claude/rules/` 他ファイル（4箇所）

| ファイル | 行 | 変更先 |
|---------|-----|--------|
| `pre-implementation.md` | L81 | `dev-flow-tdd.md#確認事項の実施` |
| `code-annotations.md` | L85 | `dev-flow-issue.md#todofixme-の棚卸し` |
| `problem-solving.md` | L12 | `dev-flow-issue.md#issue-精査` |
| `structural-review.md` | L98 | `dev-flow-issue.md#品質チェックリスト` |

### 4d: `.claude/skills/start/SKILL.md`（2箇所）

| 行 | 変更先 |
|----|--------|
| L113 | `dev-flow-issue.md#issue-精査` |
| L143 | `dev-flow-issue.md#draft-pr-作成` |

### 4e: その他（3箇所）

| ファイル | 変更内容 |
|---------|---------|
| `.github/pull_request_template.md` L16 | `dev-flow-issue.md#品質チェックリスト` |
| `docs/04_手順書/00_はじめに.md` L54, L127 | 「AI版: `.claude/rules/dev-flow-issue.md`」の注記追加 |

---

## Phase 5: 検証

- `just check-all` パス
- 新 rules サイズ: dev-flow-issue.md ≤ 14KB, dev-flow-tdd.md ≤ 11KB, 合計 ≤ 25KB
- 元手順書の全ステップ・ゲート・禁止事項が新 rules でカバーされていることの突合
- 全参照リンクが有効（grep で被参照を検索、アンカー存在確認）
