---
name: resume
description: セッション再開時のコンテキスト復元を自動化する。ブランチから Issue を特定し、残タスクと作業状況を把握して再開する。
argument-hint: <省略可。Issue 番号>
user-invocable: true
---

# セッション再開

`/wrap-up`（セッション終了時）と対になる、セッション開始時のスキル。
コンテキストを自動復元し、効率的に作業を再開する。

## 引数

$ARGUMENTS

引数で Issue 番号を指定した場合は、その Issue のコンテキストを復元する。
引数がない場合は、現在のブランチから Issue 番号を自動検出する。

## 手順

### Step 1: Issue を特定

現在のブランチ名から Issue 番号を抽出する:

```bash
git branch --show-current
```

ブランチ名のパターン: `feature/{Issue番号}-{機能名}` / `fix/{Issue番号}-{バグ名}`

| 状態 | 対応 |
|------|------|
| ブランチ名に Issue 番号あり | 抽出して Step 2 へ |
| 引数で Issue 番号指定あり | 指定番号を使って Step 2 へ |
| main ブランチ + 引数なし | オープンな Issue 一覧を表示し、ユーザーに選択を促す |
| ブランチ名に番号なし + 引数なし | ユーザーに Issue 番号を確認 |

main ブランチで Issue を選択した場合、対応するブランチに切り替える:

```bash
# オープンな Issue 一覧
gh issue list --state open --limit 20

# 対応するブランチが存在するか確認
git branch --list "feature/<Issue番号>-*" "fix/<Issue番号>-*"

# ブランチが存在すれば切り替え
git checkout <ブランチ名>
```

対応するブランチが存在しない場合は、新規ブランチ作成を提案する。

### Step 2: Issue の状態を確認

```bash
gh issue view <Issue番号>
```

以下を確認し、整理する:

- Issue タイトルと概要
- 完了基準のチェックリスト（✅/⬜ の状態）
- 実装計画（Phase の進捗、テストリスト）

### Step 3: 直近の作業を確認

```bash
# このブランチのコミット一覧
git log --oneline --reverse main..HEAD

# 変更ファイルの統計
git diff --stat main...HEAD
```

Draft PR がある場合は PR の状態も確認する:

```bash
gh pr view --json number,state,isDraft,title,url 2>/dev/null
```

### Step 4: セッションログを参照

関連するセッションログを検索する:

```bash
# 直近のセッションログ
ls -t prompts/runs/$(date +%Y-%m)/*.md 2>/dev/null | head -5

# Issue 番号で検索
grep -rl "#<Issue番号>" prompts/runs/ 2>/dev/null
```

関連するセッションログがあれば内容を読み込み、前回の作業状況を把握する。
なければスキップする（セッションログの有無は作業再開に影響しない）。

### Step 5: コンテキストを提示して作業開始

収集した情報を以下の形式で提示する:

```
## セッション再開: #<Issue番号> <タイトル>

### 現在の状況
- ブランチ: <ブランチ名>
- PR: <PR URL（あれば）>
- 直近の作業: <最新コミットのサマリー>
- 進捗: <Phase X 完了 / Phase Y 作業中>

### 残タスク
- [ ] タスク1
- [ ] タスク2

### 次にやるべきこと
<Issue の実装計画とコミット履歴から特定した、次の具体的アクション>
```

ユーザーに「この方針で進めますか？」と確認する。
合意が得られたら、必要なファイルを読み込んで作業を開始する。
