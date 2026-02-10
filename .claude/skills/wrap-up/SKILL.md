---
name: wrap-up
description: セッションの区切りでドキュメントを整備する。作業内容を振り返り、必要なドキュメント（セッションログ、ADR、ナレッジベース等）を特定・作成する。
argument-hint: <省略可。ドキュメント種類を指定: log, adr, knowledge, impl, recipe, improvement>
user-invocable: true
---

# ドキュメント整備（Wrap-up）

セッションや作業の区切りで、CLAUDE.md の「ドキュメント自動作成ルール」に基づき、必要なドキュメントを漏れなく作成する。

## 引数

$ARGUMENTS

引数でドキュメント種類を指定した場合は、そのドキュメントのみ作成する。
引数がない場合は、全種類をチェックして必要なものを提案する。

| 引数 | ドキュメント種類 |
|------|-----------------|
| `log` | セッションログ |
| `adr` | ADR |
| `knowledge` | ナレッジベース |
| `impl` | 実装解説 |
| `recipe` | 操作レシピ |
| `improvement` | 改善記録 |

## 手順

### Step 1: 作業内容の振り返り

現在のブランチの変更内容を確認する:

```bash
# ブランチ上のコミット一覧
git log --oneline --reverse HEAD --not main
# 変更ファイルの統計
git diff --stat main...HEAD
```

会話の文脈と合わせて、以下の観点で作業を分類する:

- コード変更があったか
- 技術選定・設計判断があったか
- 新しいツール・パターンを導入したか
- 非自明な問題解決があったか
- AI 運用上の問題があったか
- 判断ログに記録すべき内容があったか

### Step 2: 必要なドキュメントの特定

以下のチェックリストに基づき判定結果をユーザーに提示する。各項目に 🟢（作成推奨）/ ⚪（該当なし）を付け、推奨理由を添える。

| ドキュメント | 作成条件 | 配置先 |
|-------------|---------|--------|
| セッションログ | コード変更または設計判断があった | `prompts/runs/YYYY-MM/` |
| ADR | 技術選定、実装方針の選択・見送りがあった | `docs/05_ADR/` |
| ナレッジベース | 新しいツール・パターン導入、技術解説が必要 | `docs/06_ナレッジベース/` |
| 実装解説 | Phase 完了、設計判断を伴う実装があった | `docs/07_実装解説/` |
| 操作レシピ | 非自明な操作で問題解決、再利用可能なパターンを発見した | `prompts/recipes/` |
| 改善記録 | AI 運用上の問題と対策があった | `prompts/improvements/YYYY-MM/` |

注意: Git 管理外の情報源（auto memory `~/.claude/projects/.../memory/`、会話コンテキスト等）は永続的なドキュメントではない。これらに設計解説や技術知識が含まれていても、ドキュメント作成を省略する理由にならない。判定基準は「リポジトリ（Git 管理下）に形式知として残っているか」である。

ユーザーの確認を得てから Step 3 に進む。

### Step 3: ドキュメント作成

選択されたドキュメントを順番に作成する。各ドキュメントの規約は以下を参照する。

#### セッションログ

規約: `prompts/runs/README.md`

1. 初回コミット時刻を取得してファイル名を組み立てる:
   ```bash
   git log --format="%ci" --reverse HEAD --not main | head -1
   ```
2. ファイル名を命名規則と照合する（`docs.md` の照合プロセス）
3. セクションを作成する: 概要 → 実施内容 → 判断ログ → 成果物 → （該当時のみ）議論の経緯
4. サニタイズ規則に従って校正

#### ADR

テンプレート: `docs/05_ADR/template.md`

1. 次の ADR 番号を確認:
   ```bash
   ls docs/05_ADR/*.md | sort | tail -3
   ```
2. テンプレートに従って作成
3. セッション中に合意済みならステータスは「承認済み」

#### ナレッジベース

規約: `docs/06_ナレッジベース/README.md`

1. 既存カテゴリとファイルを確認し、追記か新規作成か判断
2. 構成: 概要 → 主な機能 → 使い方 → プロジェクトでの使用箇所 → 関連リソース

#### 実装解説

規約: `docs/07_実装解説/README.md`

1. 既存の実装解説ディレクトリを確認
2. 関連する機能ディレクトリに Phase ファイルを追加、または新規ディレクトリを作成
3. 構成: 対応 Issue → 概要 → アーキテクチャ → Phase 表 → 関連ドキュメント

#### 操作レシピ

規約: `prompts/recipes/README.md`

1. 既存レシピを確認し、重複がないか確認
2. 構成: いつ使うか → 手順（Claude Code / 手動） → なぜこの方法か

#### 改善記録

規約: `prompts/improvements/README.md`

1. 初回コミット時刻を取得してファイル名を組み立てる
2. 構成: 事象 → 原因分析 → 対策 → 次のアクション → 分類
3. 恒久対策がある場合は GitHub Issue を作成し、記録に Issue 番号を記載

### Step 4: 計画ファイルのリネーム

セッション中に plan mode を使用した場合、計画ファイルをランダム名から命名規則に従ってリネームする。

→ 命名規則: [`prompts/plans/README.md`](../../../prompts/plans/README.md)

```bash
# ランダム名の計画ファイルがあるか確認
ls prompts/plans/ | grep -v '^[0-9]' | grep -v README
```

ランダム名のファイルがあれば `git mv` でリネームする:

```bash
git mv prompts/plans/clever-napping-panda.md prompts/plans/288_dev-auth-feature-flag.md
```

### Step 5: 確認と仕上げ

1. 作成したドキュメントの一覧を提示
2. コミットするかユーザーに確認
3. コミットメッセージの例:
   - `Add session log for <トピック>`
   - `Add ADR-NNN: <タイトル>`
   - `Add knowledge base entry for <技術名>`
   - `Add implementation notes for <機能名>`
   - `Add recipe: <レシピ名>`
   - `Add improvement record: <トピック>`
   - `Rename plan file for #<Issue番号>`
