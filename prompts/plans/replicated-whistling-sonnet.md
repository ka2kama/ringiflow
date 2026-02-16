# `/next` スキル改修計画

## Context

`/next` スキルは現在シングルワーク前提で設計されている。進行中の作業があると Step 1 で停止し、並行開発の候補を提示しない。また worktree の存在を認識しないため、別 worktree で作業中の Issue を候補に出してしまう。

並行開発環境（ADR-021）が整備済みのプロジェクトにおいて、`/next` がその恩恵を活かせていない状態を改善する。

## 対象

- `.claude/skills/next/SKILL.md`（書き換え）

対象外:
- `/restore` スキル（連携先だが変更なし）
- worktree スクリプト群（既存のまま利用）

## 改修内容

### 変更点サマリー

| 観点 | 現行 | 改修後 |
|------|------|--------|
| worktree 確認 | なし | `git worktree list --porcelain` で全 worktree をチェック |
| 進行中で停止 | する（Step 1 で終了） | しない（コンテキスト表示後、候補も提示） |
| Epic/idea 除外 | なし | `type:epic`, `idea` ラベルで除外 |
| 出力形式 | フラットリスト | 3 セクション（進行中 / 推奨 / 他候補） |
| 並行開発 | 考慮なし | カテゴリベースのグルーピング |
| 着手案内 | `git checkout -b` | worktree 作成（`just worktree-issue`）を優先案内 |

### 改修後のアルゴリズム（7 Step）

#### Step 1: 進行中の作業を収集

全 worktree と全 open PR から進行中の Issue 番号を収集する。

```bash
# worktree のブランチ一覧
git worktree list --porcelain

# 自分の open PR 一覧
gh pr list --author @me --state open --json number,title,isDraft,headRefName,url
```

ブランチ名 `feature/{番号}-*` / `fix/{番号}-*` から Issue 番号を抽出。main ブランチは除外。PR のブランチと worktree のブランチで重複する場合があるため、番号で重複排除。

抽出結果 = `IN_PROGRESS_ISSUES`

#### Step 2: オープンな Issue を取得

```bash
gh issue list --state open --json number,title,labels,body --limit 50
```

limit を 30 → 50 に拡大（並行開発で消化ペースが上がるため）。

#### Step 3: 候補をフィルタリング

| 除外条件 | 理由 |
|---------|------|
| `type:epic` ラベル | Epic はコンテナ、直接作業しない |
| `idea` ラベル | 将来検討用 |
| `IN_PROGRESS_ISSUES` に含まれる | 既に着手中 |
| `Blocked by #N` で N がオープン | ブロックされている |

#### Step 4: 候補を優先度ソート

優先度ラベルで降順ソート（現行と同じ）:

| 優先度 | ラベル |
|--------|--------|
| 高 | `priority:high`, `priority:critical` |
| 中 | `priority:medium` |
| 低 | `priority:low`, ラベルなし |

同一優先度のタイブレイク:
1. `Blocks` で他 Issue をブロック → 優先
2. `type:story` ラベルあり → 優先（具体的な作業単位）
3. Issue 番号が小さい（古い順）

#### Step 5: 並行開発グルーピング

進行中の作業のカテゴリラベルを収集（Step 2 の結果を再利用）。

カテゴリラベル: `backend`, `frontend`, `infra`, `docs`, `process`

候補 Issue を以下で分類:

| 分類 | 条件 |
|------|------|
| 並行推奨 | カテゴリが進行中作業と重複しない AND 候補間で依存関係なし |
| 着手可能 | 上記以外 |

カテゴリ未分類の Issue: 並行可能として扱うが、未分類同士は 1 件のみ推奨（競合の可能性がある）。

進行中の作業がない場合: グルーピングせず、従来通り優先度順で提示。

#### Step 6: 結果を提示

```
## 🎯 作業状況と次の推奨

### 📌 進行中の作業 (N件)

| Issue | ブランチ | PR | カテゴリ |
|-------|---------|-----|---------|
| #528 タイトル | feature/528-xxx | #580 | backend |

---

### 🟢 並行着手が可能な候補

進行中の作業とカテゴリが異なり、並行して着手できます。

1. **#566 タイトル** (priority:medium, process)
   - 理由: 進行中の backend と競合なし

### ⚪ 他の候補

1. #537 タイトル (backend, priority:medium)
2. #531 タイトル (backend, priority:medium)
3. ...

---
着手する場合: `just worktree-issue <Issue番号>` で worktree を作成
現在の作業を続ける場合: `/restore` を実行
```

進行中の作業がない場合は「📌 進行中の作業」セクションを省略し、従来の `/next` と同様のフラット表示にする。

#### Step 7: ユーザーの選択を確認

1. 対応する worktree が既にあるか確認（`git worktree list`）
2. あれば `cd` パスを案内
3. なければ `just worktree-issue {番号}` を提案
4. main で作業する場合は `git checkout -b` を提案

### 設計判断

1. 進行中で停止しない: 並行開発環境があるのに1つの作業しか見せないのはもったいない。コンテキストとして表示しつつ、候補も提示する
2. カテゴリベースのグルーピング: 依存関係だけでは並行可能性を判断しきれない。カテゴリ（backend/frontend 等）が異なれば変更が衝突しにくい
3. worktree 優先案内: 並行開発には worktree が必要。`git checkout -b` だと現ブランチを離れることになるため、worktree 作成を第一選択にする
4. Epic/idea の除外: 現行では除外していなかったが、Epic は作業単位ではなくコンテナ、idea は将来検討用なので候補に含めるべきでない

## 検証方法

1. `/next` を実行し、進行中の worktree（#528）が「進行中の作業」セクションに表示されることを確認
2. #528 が候補リストに含まれないことを確認
3. `type:epic`（#467, #406, #405, #404）と `idea`（#445, #389, #302, #136）が候補に含まれないことを確認
4. #566（process）が「並行着手が可能な候補」として表示されることを確認（進行中 #528 は backend のため）
5. #537, #531, #530（backend）は「他の候補」に表示されることを確認
