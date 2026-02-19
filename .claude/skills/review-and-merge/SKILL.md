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

### 対象 PR の決定

1 回の呼び出しで処理する PR は 1 つのみ。

| 条件 | 動作 |
|------|------|
| 引数あり | 指定された PR を対象にする |
| 引数なし + 現在のブランチに PR あり | 現在のブランチの PR を対象にする |
| 引数なし + 現在のブランチに PR なし | Ready 状態の PR を一覧表示し、ユーザーに選択を求める |

改善の経緯: [review-and-merge で複数 PR を同時処理しようとした](../../../process/improvements/2026-02/2026-02-19_0234_review-and-mergeで複数PRを同時処理しようとした.md)

## 手順

### Step 1: PR 状態確認 + base branch 同期

PR 状態と base branch 同期を一括で確認する。分離して実行しないこと。

```bash
gh pr view --json number,state,isDraft,reviewDecision,statusCheckRollup,url,baseRefName
git fetch origin main && git log HEAD..origin/main --oneline
```

以下の判定に基づいて対応する:

| PR 状態 | 対応 |
|---------|------|
| PR が存在しない | エラーメッセージを表示して終了 |
| Draft 状態 | `gh pr ready` で解除するかユーザーに確認 |
| CI 未通過 | `gh pr checks --watch` で待機するかユーザーに確認 |

| base branch 同期 | 対応 |
|------------------|------|
| 差分なし | Step 2 へ |
| 差分あり | rebase + push → 既存レビューコメントの先行確認（後述）→ Step 2 へ |

#### CI 再実行時の既存レビューコメント先行確認

rebase + push 後は CI + Review が再実行される。前回のレビューサイクルで既にコメントが存在する場合、CI 完了を待たずに先行して確認・対応する:

1. rebase + push 後、Step 3 の API で既存のレビューコメントを取得する
2. コメントがあれば Step 4 の手順で対応する
3. 対応完了後、Step 2 に進んで新しい CI + Review の完了を待つ
4. 新しいレビューで追加コメントがあれば、通常フロー（Step 3 → Step 4）で対応する

コメントがない場合はそのまま Step 2 へ進む。

改善の経緯:
- [review-and-merge で rebase 確認が遅い](../../../process/improvements/2026-02/2026-02-09_2106_review-and-mergeでrebase確認が遅い.md)
- [review-and-merge で base branch 同期確認をスキップ](../../../process/improvements/2026-02/2026-02-14_2120_review-and-mergeでbase-branch同期確認をスキップ.md)
- [CI 再実行時にレビューコメント確認を後回しにする](../../../process/improvements/2026-02/2026-02-17_1748_CI再実行時にレビューコメント確認を後回しにする.md)

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

#### メタデータ抽出

取得した各 PR レベルコメント（3 の結果）について、メタデータブロックを抽出する:

```bash
# コメント本文からメタデータを抽出（grep の -z オプションで複数行マッチ）
echo "$COMMENT_BODY" | grep -Pzo '(?<=<!-- review-metadata\n)[\s\S]*?(?=\n-->)' || echo ""
```

メタデータが存在する場合はそれを使用し、存在しない場合は従来通り本文を解釈する。

取得した情報を以下の形式でユーザーに提示する:

```
## レビュー結果サマリー

レビュー: N 件（APPROVED: X, CHANGES_REQUESTED: Y）
Review コメント: N 件

### 全体フィードバック

#### コメント 1（Auto Review）

**メタデータ:**
- Type: auto-review
- 重大度: Critical=0, High=1, Medium=2, Low=3
- 対応要否: false（APPROVED）

（内容要約）

#### コメント 2（Rules Check）

**メタデータ:**
- Type: rules-check
- 重大度: Critical=0, High=0, Medium=0, Low=0
- 対応要否: false（pass）

（内容要約）

注: メタデータが存在しない場合（古いコメント）は、「メタデータ:」セクションを省略し、従来通り本文を解釈してサマリーを提示する。

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

指摘を含むコメントが存在する場合、各コメントについて以下を行う。対象は Review コメント（inline）と PR コメント（全体フィードバック内の指摘）の両方。

1. コメント内容と該当コードを表示（ファイルを読んで前後のコンテキストを含める）
2. 修正案を提示する
3. ユーザーに対応方針を確認する（修正する / スキップ / 延期 / カスタム修正）
4. 承認された修正を適用する
5. 修正に対応するテストが存在するか確認する。存在しない場合はテストを追加する

対応方針の判断基準:

| 方針 | 使い分け |
|------|---------|
| 修正する | この PR で対応すべき指摘 |
| スキップ | 対応不要と判断した指摘（理由を返信に記載） |
| 延期 | 妥当だが今回のスコープ外。Issue を作成して追跡する |
| カスタム修正 | 提案とは異なる方法で修正する |

**延期を選択した場合:**
1. 指摘内容を元に Issue を作成する（`gh issue create`）
2. Issue 番号を含む返信をレビューコメントに投稿する（下記テンプレート参照）
3. 延期返信には **必ず Issue 番号を含める**こと（成果物要件）。Issue 番号のない延期返信は禁止

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

#### Review コメント（inline）への返信

対応完了後、レビューコメントに返信し、スレッドを resolve する:

```bash
# 対応した場合
gh api "repos/{owner}/{repo}/pulls/{pr_number}/comments/{comment_id}/replies" \
  -f body="修正しました。"

# スキップした場合（理由を記載）
gh api "repos/{owner}/{repo}/pulls/{pr_number}/comments/{comment_id}/replies" \
  -f body="（スキップ理由）"

# 延期した場合（Issue 番号必須）
gh api "repos/{owner}/{repo}/pulls/{pr_number}/comments/{comment_id}/replies" \
  -f body="Issue #<番号> で対応予定です。"
```

返信後、未 resolve のスレッドを resolve する:

```bash
# 未 resolve スレッドの ID を取得
gh api graphql -f query='
query {
  repository(owner: "{owner}", name: "{repo}") {
    pullRequest(number: {pr_number}) {
      reviewThreads(first: 100) {
        nodes { id isResolved }
      }
    }
  }
}'

# 各未 resolve スレッドを resolve
gh api graphql -f query='
mutation {
  resolveReviewThread(input: {threadId: "{thread_id}"}) {
    thread { isResolved }
  }
}'
```

注意: `required_review_thread_resolution` ブランチ保護ルールにより、未 resolve スレッドがあるとマージがブロックされる。返信だけでは自動 resolve されないため、明示的に resolve が必要。

#### PR コメント（全体フィードバック）への返信

PR コメント（issues/comments）に指摘事項が含まれる場合も、対応完了後に返信コメントを投稿する。PR コメントはスレッド構造を持たないため、元コメントの URL を含めて関連性を明示する。

```bash
# 元コメントの URL を取得（Step 3 で取得済みの html_url を使用）
# 対応した場合
gh api "repos/{owner}/{repo}/issues/{pr_number}/comments" \
  -f body="[コメント]({comment_url}) への対応: 修正しました。"

# スキップした場合（理由を記載）
gh api "repos/{owner}/{repo}/issues/{pr_number}/comments" \
  -f body="[コメント]({comment_url}) への対応: （スキップ理由）"

# 延期した場合（Issue 番号必須）
gh api "repos/{owner}/{repo}/issues/{pr_number}/comments" \
  -f body="[コメント]({comment_url}) への対応: Issue #<番号> で対応予定です。"
```

注意: PR コメントには resolve の仕組みがない。返信の有無が Step 5 の未対応検証で使用される。

改善の経緯:
- [レビュー返信の検討事項に Issue 追跡が欠如](../../../process/improvements/2026-02/2026-02-11_2234_レビュー返信の検討事項にIssue追跡が欠如.md)
- Issue [#451](https://github.com/ka2kama/ringiflow/issues/451): review-and-merge でレビュースレッドの resolve 漏れ

### Step 5: マージ

マージ前に以下を確認する。

#### 未対応レビュー指摘のゼロ検証

Step 3 と同じ API で最新のレビュー状態を**再取得**し、未対応の指摘がゼロであることを検証する。セッション復元の有無に関わらず常に実行する。

検証項目:

| # | 対象 | 検証方法 | 合格基準 |
|---|------|---------|---------|
| 1 | reviewDecision | `gh pr view --json reviewDecision` | APPROVED |
| 2 | Review threads（inline） | GraphQL `reviewThreads` の `isResolved` | 未 resolve がゼロ |
| 3 | PR コメント（全体フィードバック） | `gh api issues/{pr_number}/comments` で claude[bot] のコメントを取得し、指摘事項を含むコメントに返信があるか確認 | 指摘を含む未返信コメントがゼロ |

```bash
# 1. reviewDecision
gh pr view --json reviewDecision --jq '.reviewDecision'

# 2. 未 resolve の review threads
gh api graphql -f query='
query {
  repository(owner: "{owner}", name: "{repo}") {
    pullRequest(number: {pr_number}) {
      reviewThreads(first: 100) {
        nodes { id isResolved }
      }
    }
  }
}'

# 3. claude[bot] の PR コメントと全コメントを取得し、返信の有無を確認
gh api "repos/{owner}/{repo}/issues/{pr_number}/comments" \
  --jq '[.[] | select(.user.login == "claude[bot]")]'
```

検証 #3 の判定手順:

1. claude[bot] の PR コメントを取得する
2. 各コメントからメタデータを抽出する:
   ```bash
   echo "$COMMENT_BODY" | grep -Pzo '(?<=<!-- review-metadata\n)[\s\S]*?(?=\n-->)' || echo ""
   ```
3. メタデータが存在する場合（機械的判定）:
   - 指摘件数（severity-critical, severity-high, severity-medium, severity-low）の合計を計算する
   - 合計がゼロでなければ「指摘を含むコメント」として扱う
4. メタデータが存在しない場合（後方互換性）:
   - 従来通り、コメントの内容を読んで具体的な指摘事項を含むか判定する（サマリーのみのコメントは対象外）
5. 指摘を含むコメントについて、そのコメント URL を参照する返信コメントが存在するか確認する
6. 指摘を含む未返信のコメントがあれば、ユーザーに提示してマージを止める

判定ロジックの疑似コード:

```python
for comment in claude_bot_comments:
    metadata = extract_metadata(comment.body)
    if metadata:
        # メタデータがある場合（機械的判定）
        total_issues = (
            metadata["severity-critical"] +
            metadata["severity-high"] +
            metadata["severity-medium"] +
            metadata["severity-low"]
        )
        has_issues = total_issues > 0
    else:
        # メタデータがない場合（従来の解釈）
        has_issues = contains_specific_feedback(comment.body)

    if has_issues:
        # 返信の有無を確認
        if not has_reply(comment):
            # 未返信の指摘があるのでマージを止める
            abort_merge(comment)
```

| 検証結果 | 対応 |
|---------|------|
| 全て合格 | 全チェックの pass 確認へ |
| 未対応あり | ユーザーに未対応の指摘を提示し、Step 4 に戻る |

改善の経緯: [セッション復元時のレビューコメント検証省略](../../../process/improvements/2026-02/2026-02-13_1148_セッション復元時のレビューコメント検証省略.md)

#### 全チェックの pass 確認

```bash
gh pr checks
```

| 状態 | 対応 |
|------|------|
| 全て pass | ユーザーに最終確認を求めてマージへ |
| pending あり | `gh pr checks --watch` で待機 |
| failure あり | 原因を特定し修正する。原因が特定できたら修正可能か判断する（「自分の変更とは無関係」は修正しない理由にならない） |

改善の経緯:
- [force-push後の自動レビュー待機漏れ](../../../process/improvements/2026-02/2026-02-04_1650_force-push後の自動レビュー待機漏れ.md)
- [CI失敗を無視してマージを強行しようとした](../../../process/improvements/2026-02/2026-02-09_2122_CI失敗を無視してマージを強行しようとした.md)

#### マージ実行

```bash
gh pr merge --squash
just clean-branches
```

マージ完了後、PR の URL を表示して終了する。

**禁止フラグ:** `--auto`、`--admin` は使用しない。

- `--auto`: 全チェック通過時に即座にマージするため、その後にプッシュしたコミットがマージに含まれない
- `--admin`: branch protection をバイパスするため、品質ゲートが無効になる

#### `gh pr merge` 失敗時の対応

`gh pr merge --squash` が失敗した場合、`--auto` にフォールバックせず以下の手順で対応する:

1. エラーメッセージから原因を特定する
2. 原因を解消する（下表参照）
3. 解消後、`gh pr merge --squash` を再試行する

| 原因 | 対応 |
|------|------|
| CI チェックが未完了 | `gh pr checks --watch` で待機してから再試行 |
| レビュー未承認 | Step 2 に戻り、レビュー完了を待つ |
| 未 resolve のスレッドあり | スレッドを resolve してから再試行 |
| base branch と差分あり | rebase + push → CI + Review 完了を待ってから再試行 |
| その他 | 原因をユーザーに報告し、対応を相談する |

改善の経緯: [auto-merge による後続コミット漏れ](../../../process/improvements/2026-02/2026-02-11_2234_auto-mergeによる後続コミット漏れ.md)
