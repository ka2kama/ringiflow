---
name: review-and-merge
description: Claude Code Action のレビューコメントを確認し、対応が必要なものは半自動で修正、問題なければマージする。
argument-hint: <PR番号（省略時は現在のブランチの PR）>
user-invocable: true
---

# レビュー確認 & マージ

Claude Code Action による自動レビューのコメントを確認し、対応が必要なものは修正案を提示、問題なければマージする。

## 引数

$ARGUMENTS

引数で PR 番号を指定した場合はその PR を対象にする。
引数がない場合は、現在のブランチに紐づく PR を対象にする。

## 手順

### Step 1: PR 状態確認 + base branch 同期

```bash
gh pr view --json number,state,isDraft,reviewDecision,statusCheckRollup,url,baseRefName
```

以下の判定に基づいて対応する:

| 状態 | 対応 |
|------|------|
| PR が存在しない | エラーメッセージを表示して終了 |
| Draft 状態 | `gh pr ready` で解除するかユーザーに確認 |
| CI 未通過 | `gh pr checks --watch` で待機するかユーザーに確認 |
| Ready 状態 | base branch 同期確認へ |

#### base branch との同期確認

Review 完了を待つ前に、base branch との同期状態を確認する。差分がある場合は rebase + push を先に行い、CI + Review のやり直しを防ぐ。

```bash
git fetch origin main
git log HEAD..origin/main --oneline
```

| 状態 | 対応 |
|------|------|
| 差分なし | Step 2 へ |
| 差分あり | rebase + push してから Step 2 へ（CI + Review が再実行される） |

改善の経緯: [review-and-merge で rebase 確認が遅い](../../../prompts/improvements/2026-02/2026-02-09_2106_review-and-mergeでrebase確認が遅い.md)

### Step 2: Claude Auto Review 完了確認

```bash
gh pr checks
```

"Claude Auto Review" のステータスを確認する:

| 状態 | 対応 |
|------|------|
| pending / in_progress | 「レビュー実行中です。完了まで待ちますか？」と確認。待つ場合は 20〜30 秒間隔でポーリング |
| success / failure | Step 3 へ |
| 見つからない | 「Claude Auto Review は CI 完了後にトリガーされます。CI の状態を確認してください」と案内 |

### Step 3: レビューコメント取得・分析

以下の 3 つの API で claude[bot] のレビュー情報を取得する。

注意: 各コマンドは `gh api` で始めること。変数代入を `&&` で繋ぐとパーミッションルール `Bash(gh *)` にマッチしなくなる。

```bash
# 1. 全レビュー（APPROVED / CHANGES_REQUESTED）
gh api "repos/{owner}/{repo}/pulls/{pr_number}/reviews" \
  --jq '[.[] | select(.user.login == "claude[bot]")]'

# 2. Review コメント（コードの特定行への指摘）
gh api "repos/{owner}/{repo}/pulls/{pr_number}/comments" \
  --jq '[.[] | select(.user.login == "claude[bot]")]'

# 3. 全 PR レベルコメント（全体フィードバック・サマリー）
gh api "repos/{owner}/{repo}/issues/{pr_number}/comments" \
  --jq '[.[] | select(.user.login == "claude[bot]")]'
```

`{owner}/{repo}` と `{pr_number}` は実際の値に置き換える。`gh pr view --json number --jq '.number'` で PR 番号を、`gh repo view --json nameWithOwner --jq '.nameWithOwner'` でリポジトリ名を取得できる。

取得した情報を以下の形式でユーザーに提示する:

```
## レビュー結果サマリー

レビュー: N 件（APPROVED: X, CHANGES_REQUESTED: Y）
Review コメント: N 件

### 全体フィードバック
#### コメント 1（Auto Review）
（内容要約）

#### コメント 2（Rules Check）
（内容要約）

### Review コメント一覧
1. `ファイルパス:行番号` — 内容要約
2. ...
```

レビュー状態に応じた分岐:

| レビュー状態 | 意味 | 次のステップ |
|-------------|------|-------------|
| APPROVED（コメントなし） | 問題なし | Step 5（マージ）へ |
| APPROVED（コメントあり） | Medium/Low 指摘あり | Step 4 で対応判断 |
| CHANGES_REQUESTED | Critical/High 指摘あり | Step 4 で修正必須 |

### Step 4: 対応（半自動）

Review コメントが存在する場合、各コメントについて以下を行う:

1. コメント内容と該当コードを表示（ファイルを読んで前後のコンテキストを含める）
2. 修正案を提示する
3. ユーザーに対応方針を確認する（修正する / スキップ / カスタム修正）
4. 承認された修正を適用する

一括適用はしない。必ずコメントごとにユーザーの判断を仰ぐ。

CHANGES_REQUESTED の場合:
- 全コメントへの対応完了後、`just check-all` で全体チェックを実行
- コミット・プッシュ
- Claude Auto Review が再実行されるため、**Step 2 に戻る**
- レビューが APPROVED になるまでこのループを繰り返す

APPROVED（コメントあり）の場合:
- 対応するかどうかはユーザーの判断に委ねる
- 対応した場合は同様にコミット・プッシュ → Step 2 に戻る
- 対応しない場合は Step 5 へ

対応完了後、レビューコメントに返信する（resolve のため）:

```bash
# 対応した場合
gh api "repos/{owner}/{repo}/pulls/{pr_number}/comments/{comment_id}/replies" \
  -f body="修正しました。"

# スキップした場合（理由を記載）
gh api "repos/{owner}/{repo}/pulls/{pr_number}/comments/{comment_id}/replies" \
  -f body="（スキップ理由）"
```

### Step 5: マージ

マージ前にユーザーに最終確認を求める。

```bash
gh pr merge --squash --delete-branch
just clean-branches
```

マージ完了後、PR の URL を表示して終了する。
