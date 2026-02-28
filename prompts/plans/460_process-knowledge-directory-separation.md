# #460 prompts/ ディレクトリの役割再定義 — プロセス知識の分離

## Context

#458 で改善記録（`prompts/improvements/`）のスコープを「AI エージェント運用」→「開発プロセス全般」に拡大した。これにより `prompts/`（＝LLM プロンプト）という名前と `improvements/` の内容に概念的なずれが生じた。

プロジェクトには3種の知識がある:

| 知識の種類 | 格納先（現在） | 格納先（変更後） |
|-----------|---------------|----------------|
| プロダクト知識 | `docs/` | `docs/`（変更なし） |
| AI セッション知識 | `prompts/` | `prompts/`（runs, plans, recipes のみに） |
| プロセス知識 | `prompts/` に混在 | `process/`（新設） |

**決定**: `improvements/` と `reports/` を新しいトップレベルディレクトリ `process/` に移動する。

## 対象

- `prompts/improvements/`（76 ファイル）→ `process/improvements/`
- `prompts/reports/`（6 ファイル）→ `process/reports/`
- 外部参照の更新（~25 ファイル）
- `process/README.md` 新規作成
- `prompts/README.md` 更新
- ADR-050 作成
- 基本設計書更新

## 対象外

- `prompts/runs/`, `prompts/plans/`, `prompts/recipes/` — 移動しない
- `prompts/runs/` や `prompts/plans/` 内の歴史的記録でのパス言及 — 過去のスナップショットであり、更新は歴史の改竄。コスト対効果が見合わない

## 相対リンクの影響分析

`prompts/improvements/` → `process/improvements/` の移動ではルートからの深度が同じ（2階層）のため、`../../` で始まる相対リンクはそのまま動作する。

**例外（要修正）**: 1件のみ
- `improvements/2026-02/2026-02-05_1555_*.md` L63: `../../recipes/debug/delete-root-owned-directory.md`
  - sibling ディレクトリ参照。移動後は `../../../prompts/recipes/debug/delete-root-owned-directory.md` に変更

---

## Phase 1: ファイル移動 + README 作成

`git mv` でファイルを移動し、README を作成・更新する。

### 作業内容

1. `process/` ディレクトリ作成
2. `git mv prompts/improvements process/improvements`
3. `git mv prompts/reports process/reports`
4. `process/README.md` 新規作成
5. `process/improvements/README.md` 内のパス例示を更新（L40, L47: `prompts/improvements/` → `process/improvements/`）
6. `process/reports/README.md` 内のパス例示を更新
7. sibling 参照の修正（上記の1件）
8. `prompts/README.md` から improvements/reports の記述を削除

### 確認事項

- [ ] `git mv` が rename として追跡されることの確認 → `git diff --cached --diff-filter=R`
- [ ] 移動後のファイル内相対リンクが動作すること → README の `../../.claude/rules/` リンクが root に到達するか

### テストリスト

ユニットテスト（該当なし — ドキュメント構造変更）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証:
- [ ] `git diff --cached --diff-filter=R` で rename が検出される
- [ ] `process/improvements/README.md` のリンクが正しい深度

---

## Phase 2: 外部参照の更新

`prompts/improvements` → `process/improvements`、`prompts/reports` → `process/reports` の置換を行う。

### 更新対象ファイル

**ルール・スキル・スクリプト**:

| ファイル | 変更内容 |
|---------|---------|
| `CLAUDE.md` | ドキュメント階層テーブル、改善記録パス、wrap-up 検証 grep パターン |
| `.claude/rules/problem-solving.md` | 改善記録 README リンク |
| `.claude/rules/docs.md` | 命名規則テーブル、改善の経緯リンク |
| `.claude/rules/latest-practices.md` | 改善の経緯リンク |
| `.claude/rules/data-store.md` | 改善記録リンク |
| `.claude/skills/wrap-up/SKILL.md` | 改善記録の配置先 |
| `.claude/skills/assess/SKILL.md` | レポート保存先、Glob パス |
| `.claude/skills/retro/SKILL.md` | Glob/Grep パス |
| `.claude/skills/review-and-merge/SKILL.md` | 改善の経緯リンク |
| `scripts/check-improvement-records.sh` | glob パターン、エラーメッセージ |

**CI/CD ワークフロー**:

| ファイル | 変更内容 |
|---------|---------|
| `.github/workflows/monthly-assess.yaml` | Issue テキスト内のパス |
| `.github/workflows/weekly-retro.yaml` | sparse-checkout、find パス、Issue テキスト |

**ドキュメント**:

| ファイル | 変更内容 |
|---------|---------|
| `docs/30_基本設計書/02_プロジェクト構造設計.md` | ディレクトリ構造図に `process/` を追加 |
| `docs/60_手順書/04_開発フロー/01_Issue駆動開発.md` | 改善記録パス、grep コマンド |
| `docs/70_ADR/046_Story-per-PRブランチ戦略.md` | 改善記録リンク |
| `docs/80_ナレッジベース/methodology/AI思考特性の分析ガイド.md` | パスとリンク |
| `docs/80_ナレッジベース/methodology/SRE的アプローチ.md` | 改善記録 README リンク |
| `docs/80_ナレッジベース/methodology/AIエージェント主導開発のアプローチ比較.md` | 改善記録リンク |
| `README.md` | 改善記録パス |

### 確認事項

- [ ] パターン: 各ファイルでの `prompts/improvements` の正確な出現位置 → Grep で実測

### テストリスト

検証:
- [ ] 古いパスへの参照が残っていないこと（歴史的記録を除く）
- [ ] `scripts/check-improvement-records.sh` が正常動作すること

---

## Phase 3: ADR 作成

### 作業内容

`docs/70_ADR/050_プロセス知識ディレクトリの分離.md` を作成する。

内容:
- ステータス: 承認済み
- コンテキスト: #458 でのスコープ拡大、`prompts/` との概念的ずれ
- 選択肢: A (improvements のみ移動)、B (prompts リネーム)、C (README のみ)、D (improvements + reports 移動)
- 決定: D を採用
- 帰結: 三層知識構造（docs/prompts/process）の確立

### 確認事項: なし（既知のパターンのみ）

### テストリスト

検証:
- [ ] ADR テンプレートに準拠していること

---

## 検証方法（全 Phase 完了後）

```bash
# 1. 古いパスの残存チェック（歴史的記録を除外）
grep -r "prompts/improvements" --include="*.md" --include="*.sh" --include="*.yaml" \
  --exclude-dir=prompts/runs --exclude-dir=prompts/plans .
grep -r "prompts/reports" --include="*.md" --include="*.sh" --include="*.yaml" \
  --exclude-dir=prompts/runs --exclude-dir=prompts/plans .

# 2. 改善記録バリデーション
./scripts/check-improvement-records.sh

# 3. 全体品質ゲート
just check-all
```

---

## 設計判断

1. **歴史的記録は更新しない**: `prompts/runs/` と `prompts/plans/` 内のパス言及は過去のスナップショット。~37 ファイルの大量 diff は git blame の有用性を低下させる。
2. **`process/` をトップレベルに配置**: `docs/` は連番体系のプロダクト知識用。改善記録は異なる運用サイクル（AI が頻繁に読み書き）を持つため独立ディレクトリが適切。
3. **Phase を3つに分割**: Phase 1 で `git mv` を先行させ、rename 追跡を確実にする。他の変更と混ぜると rename 検出が弱くなる。

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | sibling ディレクトリ参照（improvements → recipes）が1件ある | 不完全なパス | Phase 1 に修正を追加 |
| 2回目 | 歴史的記録の更新方針が未定義 | 曖昧 | 「対象外」セクションに明記し、設計判断に理由を記載 |
| 3回目 | `prompts/README.md` の更新が漏れていた | 未定義 | Phase 1 に README 更新を追加 |

---

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 移動対象と更新対象がすべて計画に含まれている | OK | Explore エージェントの全参照分析と突合。improvements 54参照元、reports 9参照元を分類済み。歴史的記録は対象外として理由付きで除外 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 「あれば」「必要に応じて」を検索し該当なし。各 Phase の作業が具体的ファイル名で記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 歴史的記録の扱い、移動先の選択、Phase 分割理由をすべて記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 「対象」「対象外」セクションが存在 |
| 5 | 技術的前提 | git mv の rename 追跡条件 | OK | git mv を独立コミットにすることで rename 検出を確実にする |
| 6 | 既存ドキュメント整合 | 基本設計書・ADR と矛盾がない | OK | 基本設計書の更新を Phase 2 に含む。ADR テンプレートを確認済み。次番号 050 |
