---
name: next
description: 次の推奨作業を特定する。worktree・PR から進行中の作業を把握し、GitHub Issues から優先度・依存関係・カテゴリを考慮して並行開発の候補も含めて選定する。
user-invocable: true
---

# 次の推奨作業を特定

セッション開始時や作業の区切りで「次に何をすべきか」を特定する。並行開発環境（worktree）を考慮し、進行中の作業と競合しない候補も提示する。

## 手順

### Step 1: 進行中の作業を収集

全 worktree と全 open PR から進行中の Issue 番号を収集する。

```bash
# worktree のブランチ一覧
git worktree list --porcelain

# 自分の open PR 一覧
gh pr list --author @me --state open --json number,title,isDraft,headRefName,url
```

ブランチ名 `feature/{番号}-*` / `fix/{番号}-*` から Issue 番号を抽出する。main ブランチは除外。PR のブランチと worktree のブランチで重複する場合があるため、番号で重複排除する。

抽出結果 = `IN_PROGRESS_ISSUES`

### Step 2: オープンな Issue を取得

```bash
gh issue list --state open --json number,title,labels,body --limit 50
```

### Step 3: 候補をフィルタリング

取得した Issue から以下を除外する:

| 除外条件 | 理由 |
|---------|------|
| `type:epic` ラベル | Epic はコンテナ、直接作業しない |
| `idea` ラベル | 将来検討用 |
| `IN_PROGRESS_ISSUES` に含まれる | 既に着手中 |
| `Blocked by #N` で N がオープン | ブロックされている |

### Step 4: 候補を優先度ソート

優先度ラベルで降順ソート:

| 優先度 | ラベル |
|--------|--------|
| 高 | `priority:high`, `priority:critical` |
| 中 | `priority:medium` |
| 低 | `priority:low`, ラベルなし |

同一優先度のタイブレイク:
1. `Blocks` で他 Issue をブロック → 優先
2. `type:story` ラベルあり → 優先（具体的な作業単位）
3. Issue 番号が小さい（古い順）

### Step 5: 並行開発グルーピング

進行中の作業がある場合のみ実施する。進行中の作業がない場合はスキップし、Step 6 でフラット表示する。

進行中の作業のカテゴリラベルを収集する（Step 2 の結果を再利用）。

カテゴリラベル: `backend`, `frontend`, `infra`, `docs`, `process`

候補 Issue を以下で分類:

| 分類 | 条件 |
|------|------|
| 並行推奨 | カテゴリが進行中作業と重複しない AND 候補間で依存関係なし |
| 着手可能 | 上記以外 |

カテゴリ未分類の Issue: 並行可能として扱うが、未分類同士は 1 件のみ推奨（競合の可能性がある）。

### Step 6: 結果を提示

進行中の作業がある場合:

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
着手する場合: `just worktree-issue <Issue番号> <スロット番号>` でスロットのブランチを切り替え
現在の作業を続ける場合: `/start` を実行
```

進行中の作業がない場合は「📌 進行中の作業」セクションと「🟢 並行着手が可能な候補」セクションを省略し、フラット表示する:

```
## 🎯 次の推奨作業

### 推奨: #<Issue番号> <タイトル>
- **優先度**: <high/medium/low>
- **理由**: <なぜこの Issue を推奨するか>
- **ラベル**: <ラベル一覧>

### 他の候補
1. #<番号> <タイトル> (優先度: <high/medium/low>)
2. #<番号> <タイトル> (優先度: <high/medium/low>)
3. #<番号> <タイトル> (優先度: <high/medium/low>)

---
着手する場合: `just worktree-issue <Issue番号> <スロット番号>` でスロットのブランチを切り替え
```

### Step 7: ユーザーの選択を確認

ユーザーが Issue を選択したら:

1. 利用可能なスロットを確認（`git worktree list` + `.worktree-slot` マーカー）
2. 対応するブランチが既にスロット内にあれば `cd` パスを案内
3. 空きスロット（detached HEAD）があれば `just worktree-issue {番号} {スロット番号}` を提案
4. スロットがない場合は `just worktree-create N` でスロット作成を提案
5. main で作業する場合は `git checkout -b` を提案

## 補足

- このスキルは「何をすべきか」を特定するまでが役割
- 実際の作業開始は `/start`、worktree のブランチ切り替え、または手動でブランチを作成して行う
- 優先度ラベルがない場合は、Issue の作成日時（古い順）を参考にする
- worktree での並行開発については ADR-021 を参照
