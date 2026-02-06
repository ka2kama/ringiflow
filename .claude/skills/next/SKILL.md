---
name: next
description: 次の推奨作業を特定する。進行中の作業があればそれを優先、なければ GitHub Issues から優先度・依存関係を考慮して選定する。
user-invocable: true
---

# 次の推奨作業を特定

セッション開始時や作業の区切りで「次に何をすべきか」を特定する。

## 判定の優先順位

1. 進行中の作業: 未完了のブランチ/PR があればそれを継続推奨
2. GitHub Issues: オープンな Issue から優先度・依存関係を考慮して選定

## 手順

### Step 1: 進行中の作業を確認

現在のブランチと未マージの PR を確認する:

```bash
# 現在のブランチ
git branch --show-current

# 自分の未マージ PR 一覧
gh pr list --author @me --state open --json number,title,isDraft,headRefName,url
```

| 状態 | 対応 |
|------|------|
| main 以外のブランチにいる | 継続推奨として提示、`/resume` を案内 |
| Draft PR が存在する | その PR の完了を推奨 |
| 両方なし | Step 2 へ |

進行中の作業がある場合の出力形式:

```
## 🔄 進行中の作業があります

### 継続推奨: #<Issue番号> <タイトル>
- **ブランチ**: <ブランチ名>
- **PR**: <PR URL（あれば）>
- **状態**: Draft PR / 作業中

---
続きを再開する場合: `/resume` を実行してください
```

進行中の作業がなければ Step 2 へ進む。

### Step 2: オープンな Issue を取得

```bash
gh issue list --state open --json number,title,labels,body --limit 30
```

### Step 3: 優先度でフィルタリング・ソート

取得した Issue を以下の基準で評価する:

**優先度ラベル（降順）:**

| 優先度 | ラベル例 |
|--------|---------|
| 高 | `priority:high`, `priority:critical`, `urgent` |
| 中 | `priority:medium` |
| 低 | `priority:low`, ラベルなし |

**依存関係の確認:**

Issue 本文から依存関係を抽出する:

- `Blocked by #123` → その Issue が完了するまで着手不可
- `Blocks #456` → 他の Issue をブロックしている（優先度アップ）

依存関係の判定:
1. `Blocked by` で指定された Issue がオープンなら、その Issue は候補から除外
2. `Blocks` で他の Issue をブロックしているなら、優先度を上げる

### Step 4: 推奨を提示

以下の形式で出力する:

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
着手する場合: `git checkout -b feature/<Issue番号>-<機能名>`
```

推奨理由の例:
- 「優先度 high、他の Issue をブロックしていない」
- 「#123 をブロックしているため、先に完了が必要」
- 「優先度ラベルなしだが、最も古い Issue」

### Step 5: ユーザーの選択を確認

ユーザーが Issue を選択したら:

1. 対応するブランチが存在するか確認
2. 存在すれば切り替え、なければ新規作成を提案
3. Issue の内容を確認し、作業開始の準備をする

```bash
# 対応するブランチが存在するか確認
git branch --list "feature/<Issue番号>-*" "fix/<Issue番号>-*"

# ブランチが存在すれば切り替え
git checkout <ブランチ名>

# 存在しなければ新規作成
git checkout -b feature/<Issue番号>-<機能名>
```

## 補足

- このスキルは「何をすべきか」を特定するまでが役割
- 実際の作業開始は `/resume` または手動でブランチを作成して行う
- 優先度ラベルがない場合は、Issue の作成日時（古い順）を参考にする
