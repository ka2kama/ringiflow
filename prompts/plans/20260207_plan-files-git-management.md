# Plan: 計画ファイル（plan files）を git 管理下に配置する

## Context

Claude Code の plan mode で作成される計画ファイルは、現在 `~/.claude/plans/` に保存されている（全プロジェクト共通、git 管理外）。

プロジェクトの `prompts/` 配下には、セッションログ（`runs/`）、運用改善記録（`improvements/`）、操作パターン（`recipes/`）が git 管理されているが、計画ファイルだけが git 外に取り残されている。

計画ファイルには、設計ブラッシュアップループの記録や収束確認チェックリストなど、セッションログでは拾いきれない「設計の試行錯誤の過程」が含まれている。これをログとして永続化し、設計思考の変遷を追跡可能にしたい。

## To-Be

- 計画ファイルが `prompts/plans/` に保存され、git で管理される
- `prompts/` 配下の他のディレクトリと同様に README で運用方針が明示されている
- CLAUDE.md のドキュメント体系表に反映されている

## 対象・対象外

対象:
- `.claude/settings.json` に `plansDirectory` 設定を追加
- `prompts/plans/README.md` を作成
- `prompts/README.md` を更新（plans への言及を追加）
- `CLAUDE.md` のドキュメント体系表を更新

対象外:
- 既存の `~/.claude/plans/` 内ファイルの移行（全プロジェクト混在のため）
- 計画ファイルのリネーム（Claude Code が自動生成する命名は制御不可）
- 月別サブディレクトリの導入（計画ファイルは runs ほど数が多くないため不要）

## 設計判断

### 1. 保存先: `prompts/plans/` vs `.claude/plans/`

`prompts/plans/` を採用。

理由:
- `prompts/` は「AI との協働の記録」という括りで、計画ファイルもこのカテゴリに属する
- `.claude/` は設定・ルール・フック等の「ツール設定」が置かれている場所で、コンテンツの保管場所ではない
- 既存の `runs/`, `improvements/`, `recipes/` と並ぶのが自然

### 2. 命名規則: ランダム名を許容

Claude Code が自動生成する命名（例: `clever-napping-panda.md`）をそのまま受け入れる。

理由:
- Claude Code が内部で管理するファイル名を制御する手段がない
- 計画ファイルの1行目にはタイトル（例: `# Plan: #288 Prevent DevAuth...`）が記載されており、内容は自己文書化されている
- リネームの手間に見合うメリットがない

### 3. ディレクトリ構造: フラット

月別サブディレクトリは導入しない。

理由:
- 計画ファイルは runs（127+）に比べて少量（プロジェクト単位では更に少ない）
- Claude Code が `plansDirectory` 直下にファイルを作成するため、サブディレクトリは Claude Code の動作と噛み合わない

### 4. 既存ファイルの移行: しない

`~/.claude/plans/` の既存46ファイルは移行しない。

理由:
- 全プロジェクトの計画ファイルが混在しており、仕分けが必要
- 今後のファイルから自動的に `prompts/plans/` に保存される
- 必要に応じて個別に手動で移動すれば十分

## 実装計画

### Step 1: `prompts/plans/` ディレクトリと README を作成

`prompts/plans/README.md`:

```markdown
# prompts/plans/

Claude Code の plan mode で作成された計画ファイルを保存する。

## 概要

計画ファイルには、設計段階での思考過程が記録されている:

- 設計判断とその理由
- ブラッシュアップループの記録（設計の反復改善過程）
- 収束確認チェックリスト

セッションログ（`runs/`）が「何をしたか」を記録するのに対し、計画ファイルは「どう考えが変遷したか」を記録する。

## ファイル命名

Claude Code が自動生成するランダム名（例: `clever-napping-panda.md`）をそのまま使用する。ファイルの内容は1行目のタイトル（例: `# Plan: #288 ...`）で識別する。

## ディレクトリ構造

フラット構造。月別サブディレクトリは使用しない。

```text
prompts/plans/
├── clever-napping-panda.md
├── abstract-orbiting-codd.md
└── README.md
```
```

### Step 2: `prompts/README.md` を更新

「何を置くか」セクションに plans を追加:

```diff
 - セッションログ（実施記録）→ [`prompts/runs/`](runs/)
 - 操作パターン集（再現可能な手順）→ [`prompts/recipes/`](recipes/)
+- 計画ファイル（設計の思考過程）→ [`prompts/plans/`](plans/)
```

### Step 3: `.claude/settings.json` に `plansDirectory` を追加

```diff
 {
   "$schema": "https://json-schemastore.org/claude-code-settings.json",
   "language": "japanese",
+  "plansDirectory": "./prompts/plans",
   "env": {
```

注意: `plansDirectory` に関する既知のバグ（[Issue #14186](https://github.com/anthropics/claude-code/issues/14186)）が報告されている。設定後に実際に `prompts/plans/` に保存されるか検証が必要。

### Step 4: CLAUDE.md のドキュメント体系表を更新

```diff
 | 知りたいこと | 参照先 |
 |-------------|--------|
 ...
+| 設計の思考過程 | [`prompts/plans/`](prompts/plans/) |
 | セッションログ | [`prompts/runs/`](prompts/runs/) |
```

## 検証

1. `.claude/settings.json` に `plansDirectory` を設定後、plan mode に入って計画ファイルが `prompts/plans/` に作成されることを確認
2. 作成されない場合（バグ）、回避策を検討（シンボリックリンク等）

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 変更が必要なファイルが全て計画に含まれている | OK | settings.json, prompts/README.md, prompts/plans/README.md, CLAUDE.md の4ファイル |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Step の変更内容が diff レベルで具体的 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 保存先、命名規則、ディレクトリ構造、移行の4判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 「対象・対象外」セクションで明示 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | plansDirectory の既知バグに言及、検証手順を記載 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | prompts/README.md のルール、docs.md の命名規則と整合 |

### ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → 技術的前提の確認 | plansDirectory の既知バグ調査、既存 README の構造確認 | 検証ステップを追加。README のフォーマットを runs/improvements のパターンに合わせた |
| 2回目 | 命名規則の整合性確認 | docs.md の命名規則セクション確認 | plans は Claude Code 自動生成のため命名規則テンプレートの対象外。README に理由を明記 |
