# #858 コンテキストクリア後のワークフローステップ喪失への対策

## Context

`/start` → 計画作成 → コンテキストクリア → 実装、という流れで作業すると、`/start` が提示した「次のステップ」（ブランチ作成、Draft PR 作成）がコンテキストクリアで失われる。計画ファイルのコミット漏れ（#583）と同じ「コンテキスト境界でのステップ喪失」パターン。

改善記録: `process/improvements/2026-02/2026-02-24_2106_コンテキストクリア後のDraft-PR作成漏れ.md`

## 対策方針

方向性 2（クリア前に完結）+ 方向性 3（復元時に自動検知）の組み合わせ。

`/start` を「情報提示スキル」から「ワークフロー基盤構築スキル」に拡張する。ブランチと Draft PR は Issue 駆動開発の前提条件であり、`/start` 内で作成を完結させる。

## 変更対象

`.claude/skills/start/SKILL.md` のみ。手順書や他スキルの変更は不要。

## 設計

### ワークフロー状態の検出

Step 2（Issue 特定）の後に、ブランチと PR の状態を検出して 4 つの状態に分類する。

| 状態 | ブランチ | PR | 検出条件 |
|------|---------|-----|---------|
| A: 新規着手 | なし | なし | main / detached HEAD + Issue 用ブランチなし |
| B: PR 欠落 | あり | なし | feature ブランチ上 + `gh pr view` が失敗 |
| C: 通常再開 | あり | あり（Draft） | feature ブランチ + PR が Draft |
| D: レビュー中 | あり | あり（Ready） | feature ブランチ + PR が非 Draft |

検出コマンド:

```bash
# ブランチ確認
git branch --show-current
git branch --list "feature/<Issue番号>-*" "fix/<Issue番号>-*"

# PR 確認
gh pr view --json number,state,isDraft,title,url 2>/dev/null
```

### 改修後の Step 構造

```
Step 1: main を最新化（変更なし）
Step 2: Issue を特定（変更なし）
Step 3: ワークフロー状態を検出（新規）
Step 4: 状態別フロー（新規）
  状態 A → Issue 精査 → ブランチ作成 → Draft PR 作成
  状態 B → Draft PR 作成（復旧）
  状態 C → 従来通り（Issue 状態確認、直近の作業確認）
  状態 D → レビュー状態を表示、/review-and-merge を案内
Step 5: セッションログを参照（旧 Step 5、変更なし）
Step 6: コンテキストを提示して作業開始（旧 Step 6、状態別の出力）
```

### 状態 A: 新規着手フロー

1. Issue 精査（Issue 駆動開発 セクション 1 に従う）
   - 精査結果「破棄」→ ブランチ・PR を作成せず終了
   - 精査結果「続行」→ 次のステップへ

2. ブランチ作成
   - worktree 判定: `.worktree-slot` ファイルの存在で判定
   - worktree: `git checkout -b feature/<Issue番号>-<slug> origin/main`
   - 通常: `git checkout main && git pull origin main && git checkout -b feature/<Issue番号>-<slug>`
   - slug 生成: Issue タイトルから英数字+ハイフンのスラッグを生成（`scripts/worktree/issue.sh` と同じロジック）

3. Draft PR 作成（Issue 駆動開発 セクション 3 に従う）
   - 空コミット → push → `gh pr create --draft`
   - PR 本文: テンプレートの Issue セクションに `Closes #<Issue番号>` を設定

### 状態 B: PR 欠落の復旧

1. 検出メッセージを提示
2. コミットの有無・push 状態を確認
   - コミットなし → 空コミット → push → PR 作成
   - コミットあり + 未 push → push → PR 作成
   - コミットあり + push 済み → PR 作成のみ
3. Draft PR 作成

### 状態 C: 通常再開

従来の Step 3-5 相当。Issue 状態確認、直近の作業確認。変更なし。

### 状態 D: レビュー中

PR 状態とレビュー結果を表示し、`/review-and-merge` の実行を案内。

### 冪等性

| 操作 | 保証方法 |
|------|---------|
| ブランチ作成 | 状態検出で分岐（既存ブランチがあれば作成しない） |
| Draft PR 作成 | `gh pr view` で既存 PR を検出（あれば作成しない） |
| Issue 精査コメント | 重複コメントは害がない（既存コメントの有無は確認しない） |

## 検証方法

1. SKILL.md の変更後、`just check-all` でプロジェクト全体のテストが通ることを確認
2. 実際の利用シナリオで動作確認:
   - 新規 Issue に対して `/start` を実行 → ブランチ + Draft PR が作成される
   - ブランチのみの状態で `/start` を実行 → Draft PR が復旧される
   - 通常の再開で `/start` を実行 → 従来通りのコンテキスト表示

## 参照ファイル

- `.claude/skills/start/SKILL.md` — 変更対象
- `docs/60_手順書/04_開発フロー/01_Issue駆動開発.md` — Issue 精査、ブランチ作成、Draft PR 作成の手順
- `.github/pull_request_template.md` — PR テンプレート
- `prompts/recipes/worktree環境でのブランチ作成.md` — worktree でのブランチ作成パターン
- `scripts/worktree/issue.sh` — Issue タイトルからスラッグ生成のロジック

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | worktree 環境（detached HEAD）の状態検出が未定義 | 不完全なパス | 状態 A の検出条件に「detached HEAD + Issue 用ブランチなし」を追加 |
| 1回目 | 精査結果が「破棄」の場合のパスが未定義 | 不完全なパス | 状態 A に精査結果による分岐を追加 |
| 2回目 | 状態 B でコミットの有無による分岐が必要 | エッジケース | push 状態に応じた 3 パターンの分岐を追加 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 4 つのワークフロー状態すべてに対応。worktree / 通常環境の両方を考慮 |
| 2 | 曖昧さ排除 | OK | 各状態の検出条件とアクションが具体的に定義されている |
| 3 | 設計判断の完結性 | OK | Issue 精査の位置（ブランチ作成前）、worktree 判定方法を理由付きで記載 |
| 4 | スコープ境界 | OK | 対象: SKILL.md のみ。対象外: 手順書、他スキル |
| 5 | 技術的前提 | OK | worktree 検出、`gh pr view` の失敗時挙動を確認済み |
| 6 | 既存ドキュメント整合 | OK | Issue 駆動開発フローの Step 1-3 と整合 |
