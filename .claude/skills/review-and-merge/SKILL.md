---
name: review-and-merge
description: Claude Code Action のレビューコメントを確認し、対応が必要なものは半自動で修正、問題なければマージする。
argument-hint: <省略可。PR 番号を指定>
user-invocable: true
---

# レビュー確認 & マージ

Claude Code Action による自動レビューのコメントを確認し、対応が必要なものは修正案を提示、問題なければマージする。

## 引数

$ARGUMENTS

引数で PR 番号を指定した場合はその PR を対象にする。
引数がない場合は、現在のブランチに紐づく PR を対象にする。

## 手順

### Step 1: PR 状態確認

```bash
gh pr view --json number,state,isDraft,reviewDecision,statusCheckRollup,url
```

以下の判定に基づいて対応する:

| 状態 | 対応 |
|------|------|
| PR が存在しない | エラーメッセージを表示して終了 |
| Draft 状態 | `gh pr ready` で解除するかユーザーに確認 |
| CI 未通過 | `gh pr checks --watch` で待機するかユーザーに確認 |
| Ready 状態 | Step 2 へ |

### Step 2: Claude Auto Review 完了確認

```bash
gh pr checks
```

"Claude Auto Review" のステータスを確認する:

| 状態 | 対応 |
|------|------|
| pending / in_progress | 「レビュー実行中です。完了まで待ちますか？」と確認。待つ場合は定期的にポーリング |
| success / failure | Step 3 へ |
| 見つからない | 「Claude Auto Review は CI 完了後にトリガーされます。CI の状態を確認してください」と案内 |

### Step 3: レビューコメント取得・分析

以下の 3 つの API で claude[bot] のレビュー情報を取得する:

```bash
PR_NUMBER=$(gh pr view --json number --jq '.number')
REPO=$(gh repo view --json nameWithOwner --jq '.nameWithOwner')

# 1. レビュー状態（APPROVED / CHANGES_REQUESTED）
gh api "repos/${REPO}/pulls/${PR_NUMBER}/reviews" \
  --jq '[.[] | select(.user.login == "claude[bot]")] | last'

# 2. インラインコメント（コード固有の指摘）
gh api "repos/${REPO}/pulls/${PR_NUMBER}/comments" \
  --jq '[.[] | select(.user.login == "claude[bot]")]'

# 3. PR レベルコメント（全体フィードバック・サマリー）
gh api "repos/${REPO}/issues/${PR_NUMBER}/comments" \
  --jq '[.[] | select(.user.login == "claude[bot]")] | last'
```

取得した情報を以下の形式でユーザーに提示する:

```
## レビュー結果サマリー

レビュー状態: APPROVED / CHANGES_REQUESTED
インラインコメント: N 件

### 全体フィードバック
（PR レベルコメントの内容）

### インラインコメント一覧
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

インラインコメントが存在する場合、各コメントについて以下を行う:

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
gh api "repos/${REPO}/pulls/${PR_NUMBER}/comments/${COMMENT_ID}/replies" \
  -f body="修正しました。"

# スキップした場合（理由を記載）
gh api "repos/${REPO}/pulls/${PR_NUMBER}/comments/${COMMENT_ID}/replies" \
  -f body="（スキップ理由）"
```

### Step 5: マージ

マージ前にユーザーに最終確認を求める。

```bash
gh pr merge --squash --delete-branch
just clean-branches
```

マージ完了後、PR の URL を表示して終了する。
