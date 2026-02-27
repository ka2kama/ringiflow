---
name: start
description: Issue のコンテキストを構築して作業を開始する。ブランチ・Draft PR の作成まで完結させ、コンテキストクリアによるステップ喪失を防止する。
argument-hint: <省略可。Issue 番号>
user-invocable: true
---

# 作業開始

`/wrap-up`（セッション終了時）と対になる、セッション開始時のスキル。
Issue のコンテキストを構築し、ワークフローの基盤（ブランチ・Draft PR）を確立してから作業を開始する。

改善の経緯: [コンテキストクリア後のワークフローステップ喪失](../../../process/improvements/2026-02/2026-02-24_2106_コンテキストクリア後のDraft-PR作成漏れ.md)

## 引数

$ARGUMENTS

引数で Issue 番号を指定した場合は、その Issue のコンテキストを構築する。
引数がない場合は、現在のブランチから Issue 番号を自動検出する。

## 手順

### Step 1: main を最新化

```bash
git fetch origin main
```

main ブランチにいる場合は `git pull origin main` で最新化する。feature ブランチにいる場合は、差分があれば rebase する:

```bash
# 差分があるか確認
git log HEAD..origin/main --oneline

# 差分があれば rebase
git rebase origin/main
```

### Step 2: Issue を特定

現在のブランチ名から Issue 番号を抽出する:

```bash
git branch --show-current
```

ブランチ名のパターン: `feature/{Issue番号}-{機能名}` / `fix/{Issue番号}-{バグ名}`

| 状態 | 対応 |
|------|------|
| ブランチ名に Issue 番号あり | 抽出して Step 3 へ |
| 引数で Issue 番号指定あり | 指定番号を使って Step 3 へ |
| main / detached HEAD + 引数なし | オープンな Issue 一覧を表示し、ユーザーに選択を促す |
| ブランチ名に番号なし + 引数なし | ユーザーに Issue 番号を確認 |

### Step 3: ワークフロー状態を検出

Issue 番号が確定したら、ブランチと PR の状態からワークフロー状態を判定する。

```bash
# 現在のブランチ
git branch --show-current

# Issue 用ブランチの存在確認（main / detached HEAD の場合）
git branch --list "feature/<Issue番号>-*" "fix/<Issue番号>-*"

# PR の存在・状態確認（feature ブランチ上の場合）
gh pr view --json number,state,isDraft,title,url 2>/dev/null
```

| 状態 | ブランチ | PR | 検出条件 |
|------|---------|-----|---------|
| A: 新規着手 | なし | なし | main / detached HEAD + Issue 用ブランチなし |
| B: PR 欠落 | あり | なし | feature ブランチ上 + `gh pr view` が失敗 |
| C: 通常再開 | あり | あり（Draft） | feature ブランチ + PR が Draft |
| D: レビュー中 | あり | あり（Ready） | feature ブランチ + PR が非 Draft |

判定結果を提示する:

```
### ワークフロー状態: <状態名>
- ブランチ: <ブランチ名 / なし>
- PR: <あり（Draft / Ready） / なし>
```

main / detached HEAD で Issue 用ブランチが存在する場合は、切り替えてから PR を確認する:

```bash
git checkout <ブランチ名>
```

### Step 3.5: 精査完了の検証

Issue に精査コメントが存在するか確認する:

```bash
gh issue view <Issue番号> --json comments --jq '[.comments[].body | select(test("## Issue 精査"))] | length'
```

| 結果 | 判定 |
|------|------|
| > 0 | 精査済み |
| = 0 | 未精査 |

精査の判定結果を Step 4 の状態別フローに引き継ぐ。

### Step 4: 状態別フロー

#### 状態 A: 新規着手

Issue 駆動開発フロー（Step 1-3）をこの中で実行する。

**4a-1: Issue の状態を確認**

```bash
gh issue view <Issue番号>
```

以下を確認し、整理する:

- Issue タイトルと概要
- 完了基準のチェックリスト（✅/⬜ の状態）
- 実装計画（あれば）

**4a-2: Issue 精査**

[Issue 駆動開発 > Issue 精査](../../../.claude/rules/dev-flow-issue.md#1-issue-精査) に従い、精査を実施する。精査結果を Issue コメントとして記録する。

| 精査結果 | アクション |
|---------|-----------|
| 続行 / 修正して続行 | 4a-3 へ進む |
| 再構成 | 新 Issue の作成を案内して終了 |
| 破棄 | Issue クローズを案内して終了 |

**4a-3: ブランチ作成**

worktree 環境と通常環境で分岐する:

```bash
# worktree 判定: .worktree-slot ファイルの存在
# → レシピ: prompts/recipes/worktree環境でのブランチ作成.md

# worktree 環境
git checkout -b feature/<Issue番号>-<slug> origin/main

# 通常環境
git checkout main && git pull origin main
git checkout -b feature/<Issue番号>-<slug>
```

ブランチ名の生成:
- prefix: `feature/`（新機能、Story）、`fix/`（バグ修正）
- slug: Issue タイトルから英数字+ハイフンのスラッグを生成（`scripts/worktree/issue.sh` の生成ロジックと同じ）

**4a-4: Draft PR 作成**

[Issue 駆動開発 > Draft PR 作成](../../../.claude/rules/dev-flow-issue.md#3-draft-pr-作成) に従い、Draft PR を作成する。

```bash
# 空コミットで Draft PR を作成
git commit --allow-empty -m "#<Issue番号> WIP: <Issue タイトル（英語）>"
git push -u origin HEAD
gh pr create --draft --title "#<Issue番号> <英語タイトル>" --body "<PR本文>"
```

PR 本文: `.github/pull_request_template.md` を読み込み、`## Issue` セクションに `Closes #<Issue番号>` を設定する。AI エージェントは `--body` で本文を直接指定し、末尾に署名を追加する。

#### 状態 B: PR 欠落の復旧

ブランチは存在するが PR がない状態。

```
検出: ブランチ <ブランチ名> は存在しますが、PR がありません。
```

**精査の確認（Step 3.5 の結果を使用）:**

| 精査状態 | アクション |
|---------|-----------|
| 精査済み | Draft PR 作成へ進む |
| 未精査 | 精査を実施してから Draft PR 作成へ進む |

未精査の場合、状態 A の 4a-2（Issue 精査）と同じ手順で精査を実施する。精査結果が「破棄」「再構成」の場合は、Draft PR を作成せずに終了する。

コミットの有無・push 状態を確認し、分岐する:

```bash
# コミットの有無
git log --oneline main..HEAD

# リモートとの差分
git log --oneline origin/<ブランチ名>..HEAD 2>/dev/null
```

| コミット状態 | 対応 |
|-------------|------|
| コミットなし | 空コミット → push → PR 作成 |
| コミットあり + 未 push | push → PR 作成 |
| コミットあり + push 済み | PR 作成のみ |

PR 作成は 4a-4 と同じ手順。

復旧後、Issue の状態を確認する（Step 4 状態 C と同じ処理）。

#### 状態 C: 通常再開

**精査の確認（Step 3.5 の結果を使用）:**

未精査の場合、警告を表示して精査を先に実施する:

```
検出: Issue #<番号> の精査コメントが見つかりません。作業を続行する前に精査を実施します。
```

精査を実施し、結果に応じて分岐する:
- 続行 / 修正して続行: 通常再開フローへ
- 再構成 / 破棄: 対応を案内して終了

Issue の状態と直近の作業を確認する。

```bash
# Issue の状態
gh issue view <Issue番号>

# コミット一覧
git log --oneline --reverse main..HEAD

# 変更ファイルの統計
git diff --stat main...HEAD

# PR の状態
gh pr view --json number,state,isDraft,title,url
```

以下を確認し、整理する:

- Issue タイトルと概要
- 完了基準のチェックリスト（✅/⬜ の状態）
- 実装計画（Phase の進捗、テストリスト）
- 直近のコミット
- PR の状態

#### 状態 D: レビュー中

**精査の確認（Step 3.5 の結果を使用）:**

未精査の場合、注記を表示する:

```
注記: Issue #<番号> の精査コメントが見つかりません。レビュー中のため作業は継続しますが、精査コメントの追加を推奨します。
```

PR とレビューの状態を表示し、`/review-and-merge` を案内する。

```bash
# PR の状態
gh pr view --json number,state,isDraft,title,url,reviewDecision

# レビューコメント
gh pr view --comments
```

```
PR #<番号> は Ready for Review 状態です。
レビューの確認とマージを行うには `/review-and-merge` を実行してください。
```

### Step 5: セッションログを参照

関連するセッションログを検索する:

```bash
# 直近のセッションログ
ls -t prompts/runs/$(date +%Y-%m)/*.md 2>/dev/null | head -5

# Issue 番号で検索
grep -rl "#<Issue番号>" prompts/runs/ 2>/dev/null
```

関連するセッションログがあれば内容を読み込み、前回の作業状況を把握する。
なければスキップする（セッションログの有無は作業開始に影響しない）。

### Step 6: コンテキストを提示して作業開始

収集した情報を状態別の形式で提示する。

#### 状態 A の出力（新規着手後）

```
## 作業開始: #<Issue番号> <タイトル>

### 基盤構築済み
- ブランチ: <ブランチ名> ✅
- Draft PR: <PR URL> ✅
- Issue 精査: <結果> ✅

### 残タスク
- [ ] タスク1（完了基準から）
- [ ] タスク2

### 次にやるべきこと
設計フェーズに進みます。
```

#### 状態 B の出力（復旧後）

```
## 作業再開: #<Issue番号> <タイトル>

### 復旧した基盤
- Draft PR: <PR URL> ✅（新規作成）

### 現在の状況
- ブランチ: <ブランチ名>
- コミット: N 件
- 直近の作業: <最新コミットのサマリー>

### 残タスク
- [ ] タスク1
- [ ] タスク2

### 次にやるべきこと
<Issue の進捗から特定した次のアクション>
```

#### 状態 C の出力（通常再開）

```
## 作業再開: #<Issue番号> <タイトル>

### 現在の状況
- ブランチ: <ブランチ名>
- PR: <PR URL>
- 直近の作業: <最新コミットのサマリー>
- 進捗: <Phase X 完了 / Phase Y 作業中>

### 残タスク
- [ ] タスク1
- [ ] タスク2

### 次にやるべきこと
<Issue の実装計画とコミット履歴から特定した、次の具体的アクション>
```

#### 状態 D の出力（レビュー中）

```
## 作業状況: #<Issue番号> <タイトル>

### 現在の状態
- ブランチ: <ブランチ名>
- PR: <PR URL>（Ready for Review）
- レビュー: <APPROVED / CHANGES_REQUESTED / pending>

### 次にやるべきこと
`/review-and-merge` でレビュー確認とマージを実行してください。
```

コンテキスト提示後、必要なファイルを読み込んで作業を開始する。

**禁止:** 確立されたフレームワーク（Issue 駆動開発 → 精査 → 設計 → TDD）に従う場合に「進めてよろしいですか？」等の確認を挟むこと。確立されたプロセスへの確認は安全策ではなく摩擦である。
