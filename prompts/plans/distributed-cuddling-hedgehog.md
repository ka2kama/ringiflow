# `/review-and-merge` スキル作成計画

## 概要

PR を Ready for Review にした後の「レビュー確認 → 対応 → マージ」フローを一つの Claude Code スキルとして実装する。

## 作成するファイル

- `.claude/skills/review-and-merge/SKILL.md`

## スキル定義

```yaml
name: review-and-merge
description: Claude Code Action のレビューコメントを確認し、対応が必要なものは半自動で修正、問題なければマージする。
argument-hint: <省略可。PR 番号を指定>
user-invocable: true
```

## フロー設計

### Step 1: PR 状態確認

現在のブランチに紐づく PR の状態を確認する。

```bash
# PR 情報取得（番号、状態、Draft か、レビュー判定、チェック状況）
gh pr view --json number,state,isDraft,reviewDecision,statusCheckRollup,url
```

判定ロジック:
| 状態 | 対応 |
|------|------|
| PR が存在しない | エラー終了 |
| Draft 状態 | `gh pr ready` で解除するか確認 |
| CI 未通過 | `gh pr checks --watch` で待機するか確認 |
| Ready 状態 | Step 2 へ |

### Step 2: Claude Auto Review 完了待ち

```bash
# Claude Auto Review のステータスを確認
gh pr checks
```

判定ロジック:
| 状態 | 対応 |
|------|------|
| "Claude Auto Review" が pending/in_progress | 完了まで待つか確認 |
| "Claude Auto Review" が success/failure | Step 3 へ |
| "Claude Auto Review" が見つからない | CI 完了後にトリガーされる旨を案内 |

### Step 3: レビューコメント取得・分析

```bash
# PR 番号取得
PR_NUMBER=$(gh pr view --json number --jq '.number')
REPO=$(gh repo view --json nameWithOwner --jq '.nameWithOwner')

# claude[bot] のレビュー状態
gh api "repos/${REPO}/pulls/${PR_NUMBER}/reviews" \
  --jq '[.[] | select(.user.login == "claude[bot]")] | last'

# インラインコメント（コード固有の指摘）
gh api "repos/${REPO}/pulls/${PR_NUMBER}/comments" \
  --jq '[.[] | select(.user.login == "claude[bot]")]'

# PR レベルコメント（全体フィードバック）
gh api "repos/${REPO}/issues/${PR_NUMBER}/comments" \
  --jq '[.[] | select(.user.login == "claude[bot]")] | last'
```

取得した情報を以下の形式で整理してユーザーに提示:

```
## レビュー結果サマリー

レビュー状態: APPROVED / CHANGES_REQUESTED
インラインコメント: N 件

### コメント一覧
1. [ファイル:行] 内容... → 対応推奨 / 参考情報
2. ...
```

分類基準（Claude Auto Review の承認判断に対応）:
| レビュー状態 | 意味 | 対応 |
|-------------|------|------|
| APPROVED（コメントなし） | 問題なし | Step 5（マージ）へ |
| APPROVED（コメントあり） | Medium/Low 指摘あり | Step 4 で対応判断 |
| CHANGES_REQUESTED | Critical/High 指摘あり | Step 4 で修正必須 |

### Step 4: 対応（半自動）

各コメントについて:
1. コメント内容と該当コードを表示
2. 修正案を提示
3. ユーザーに対応方針を確認（修正する / スキップ / カスタム修正）
4. 承認された修正を適用

修正後の処理:
1. `just check-all` で全体チェック
2. コミット・プッシュ
3. **再レビュー待ち**: Claude Auto Review が再実行されるので Step 2 に戻る

ユーザーに都度確認しながら進める。一括適用はしない。

### Step 5: マージ

```bash
gh pr merge --squash --delete-branch
just clean-branches
```

マージ前に最終確認をユーザーに求める。

## 設計判断

### スキル名: `review-and-merge`

`merge-pr` も候補だったが、レビュー確認が主要な責務であることを名前に反映した。

### フロー開始時点: PR が存在する状態から

`gh pr ready` もスキル内でカバーする（Draft なら解除を提案）。ただし PR 作成自体はスコープ外。Issue 駆動開発手順書の Ready for Review セクション（6.1〜6.5）のうち、6.4（CI 確認）以降をカバーする。

### 再レビューループの扱い

修正をプッシュすると Claude Auto Review が再実行される。スキル内で Step 2 に戻るループを明示し、レビュー承認まで繰り返す。

## 検証方法

1. 実際の PR で `/review-and-merge` を実行し、フローが正しく動作するか確認
2. 以下のケースをカバー:
   - APPROVED（コメントなし）→ 即マージ
   - APPROVED（コメントあり）→ コメント表示 → マージ
   - CHANGES_REQUESTED → 修正 → 再レビュー → マージ
