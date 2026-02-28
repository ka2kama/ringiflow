# Issue #271: 方法論・プロセス設計にもベストプラクティス起点を適用する

## Context

CLAUDE.md の「ベストプラクティス起点」は「全領域に適用...例外なし」と謳っているが、具体化ドキュメントは技術・ツール領域（`latest-practices.md`）のみ。problem-solving.md や zoom-rhythm.md は複数の確立された手法（A3 Thinking、OODA ループ等）を実践から帰納的に再発明したものだが、既知手法との照合がないまま運用されていた。

本 Issue は、方法論・プロセス設計にもベストプラクティス起点を明示的に適用し、既知手法との関連を文書化する。

## スコープ

**対象（6ファイル）:**

| # | ファイル | 操作 |
|---|---------|------|
| 1 | `.claude/rules/methodology-design.md` | 新規作成 |
| 2 | `docs/80_ナレッジベース/methodology/独自フレームワークと既知手法の対応.md` | 新規作成 |
| 3 | `docs/80_ナレッジベース/README.md` | カテゴリ追加 |
| 4 | `.claude/rules/problem-solving.md` | セクション追加 |
| 5 | `.claude/rules/zoom-rhythm.md` | セクション追加 |
| 6 | `CLAUDE.md` | リンク1行追加 |

**対象外:**
- コード変更
- Issue 駆動開発フローや TDD 開発フローの修正
- ADR（既存原則の明示化であり、新規の技術選定・設計判断ではないため）

## 実装順序

依存関係: Step 1-2 は他ファイルが参照する → 先に作成。Step 3-6 は独立。

### Step 1: `.claude/rules/methodology-design.md` 新規作成

`latest-practices.md` と対の関係にあるルールファイル。方法論・プロセス設計時のガイドライン。

- frontmatter: `paths: "**/*"`（`latest-practices.md` と同じパターン）
- 構成: 背景と目的 → 方針（3項: 新規作成時 / 改訂時 / 独自用語） → `latest-practices.md` との関係 → AI エージェントへの指示 → 参照
- 参照先: `CLAUDE.md`, `latest-practices.md`, ナレッジベース, セッションログ

### Step 2: `docs/80_ナレッジベース/methodology/独自フレームワークと既知手法の対応.md` 新規作成

学習効果最大化のための詳細解説。ルールファイルが「方針」、ナレッジベースが「知識」という役割分担。

構成:
- 概要
- 対応関係の全体像（Mermaid 図）
- problem-solving.md と既知手法（Gap Analysis, A3 Thinking, 5 Whys, Interest）
- zoom-rhythm.md と既知手法（OODA, Double-Loop Learning, Reflective Practice）
- Issue 駆動 + TDD と既知手法（SDD / GitHub Spec Kit）
- まとめ（独自の強み）
- 関連リソース（参考文献）

### Step 3: `docs/80_ナレッジベース/README.md` カテゴリ追加

行 21（英語カテゴリ）の後に1行追加:

```markdown
| 方法論 | `methodology/` | 独自フレームワークと既知手法の対応 |
```

### Step 4: `.claude/rules/problem-solving.md` セクション追加

行 74（`## 自己チェック` の末尾）と行 75（`## 背景`）の間に「既知手法との関連」セクションを挿入。

内容: 対応表（4行）+ 独自の追加要素 + ナレッジベースへのリンク

### Step 5: `.claude/rules/zoom-rhythm.md` セクション追加

行 378（`禁止事項` 末尾）と行 379（`## 参照`）の間に「既知手法との関連」セクションを挿入。

内容: 対応表（3行）+ 独自の追加要素（4点）+ ナレッジベースへのリンク

### Step 6: `CLAUDE.md` リンク追加

行 198-199 の間に1行挿入:

```
→ 技術・ツール領域での具体化: [最新ベストプラクティス採用方針](../../.claude/rules/latest-practices.md)
→ 方法論・プロセス設計での具体化: [方法論・プロセス設計の方針](../../.claude/rules/methodology-design.md)   ← 追加
→ 収束の方法論: [俯瞰・実装リズム](../../.claude/rules/zoom-rhythm.md)の理想駆動
```

## 検証

1. リンクの整合性: 全ファイル間の相互参照が正しいか
2. 用語の統一: 「既知手法」で統一（「確立された手法」「既存フレームワーク」と混在させない）
3. docs.md 規約: 太字は最小限、Mermaid 記法、外国語名に初出時の読み併記
4. `just check-all` は不要（ドキュメントのみの変更）

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → 挿入位置の検証 | problem-solving.md (L75 `## 背景`), zoom-rhythm.md (L379 `## 参照`), CLAUDE.md (L198-199) を実際に読んで確認 | 全挿入位置が Plan agent の提案と一致 |
| 2回目 | frontmatter の要否 | `latest-practices.md` は `paths: "**/*"` あり、`problem-solving.md` / `zoom-rhythm.md` はなし | `methodology-design.md` は `latest-practices.md` と対の関係なので同じ `paths: "**/*"` を採用 |
| 3回目 | README カテゴリの挿入位置 | 既存順序: 技術系 → クロスカッティング（英語）。方法論も非技術カテゴリ | 英語の後に追加（最新の追加は末尾が自然） |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue の3つのアイデア: (1) 既存FW調査してから設計 → Step 1 の方針、(2) 改訂時に既知手法確認 → Step 1 + Step 4-5、(3) スコープ明示 → Step 6。加えてナレッジベース（学習効果）と README 更新を追加 |
| 2 | 曖昧さ排除 | OK | 各 Step で具体的な行番号・挿入位置・内容構成を記載 |
| 3 | 設計判断の完結性 | OK | frontmatter 選択、挿入位置、ADR 不要の判断に理由を記載 |
| 4 | スコープ境界 | OK | 対象6ファイルと対象外3項目を明記 |
| 5 | 技術的前提 | OK | `.claude/rules/` の frontmatter paths 仕様を確認済み |
| 6 | 既存ドキュメント整合 | OK | セッションログの調査結果を活用、`latest-practices.md` とスコープ重複なし（技術 vs 方法論） |
