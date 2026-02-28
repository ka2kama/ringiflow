# mprocs 導入: 開発サーバー一括起動 (dev-all)

## 概要

mprocs（Rust 製 TUI プロセスマネージャー）を導入し、`just dev-all` 1コマンドで全開発サーバーを一括起動できるようにする。

## Issue 駆動開発フロー

1. GitHub Issue 作成
2. ブランチ作成: `feature/{N}-mprocs-dev-all`
3. Draft PR 作成
4. 実装（以下の Step 1〜5）
5. Ready for Review → マージ

## 実装ステップ

### Step 1: `mprocs.yaml` を新規作成（プロジェクトルート）

```yaml
procs:
  bff:
    shell: "just dev-bff"
  core-service:
    shell: "just dev-core-service"
  auth-service:
    shell: "just dev-auth-service"
  web:
    shell: "just dev-web"
```

設計判断:
- `just dev-xxx` 経由で起動（直接 `cargo run` ではなく）→ dotenv-load で .env が確実に読み込まれる
- dev-deps は mprocs に含めない → justfile 側で先に実行して完了を待つ

### Step 2: `justfile` を修正（3箇所）

**2-1. `dev-all` タスクを追加**（`dev-web` の後、行 128 付近）

```just
# 全開発サーバーを一括起動（依存サービス + mprocs）
dev-all: dev-deps
    mprocs
```

- `dev-all: dev-deps` で PostgreSQL/Redis が起動完了してから mprocs を開始

**2-2. `check-tools` に mprocs を追加**（行 41、`gh` の前）

```just
@which mprocs > /dev/null || (echo "ERROR: mprocs がインストールされていません" && exit 1)
```

**2-3. `setup` 完了メッセージに `dev-all` を追加**（行 20、先頭に追加）

```just
@echo "  - just dev-all         : 全サーバー一括起動（推奨）"
```

### Step 3: `CLAUDE.md` の開発サーバー起動セクションを更新

`just dev-all` を推奨として追加し、個別起動はサブセクションにする。

### Step 4: ADR-026 を新規作成

ファイル: `docs/70_ADR/026_開発サーバー一括起動ツールの選定.md`

- ADR-014（lefthook 導入）のフォーマットに準拠
- 検討した選択肢: mprocs / tmux / concurrently / foreman
- 決定: mprocs を採用（TUI・YAML 設定・Rust 製シングルバイナリ・目的に対して必要十分）

### Step 5: 開発環境構築ドキュメントを更新

ファイル: `docs/60_手順書/01_開発参画/01_開発環境構築.md`

- 概要テーブルに mprocs 行を追加
- セクション 6 として mprocs インストール手順を追加（既存の 6〜12 を 7〜13 に繰り下げ）
- セクション 13（旧12）「全ツールの確認」に mprocs を追加
- 変更履歴に追記

## 対象ファイル一覧

| ファイル | 操作 |
|---------|------|
| `mprocs.yaml` | 新規作成 |
| `justfile` | 修正（3箇所） |
| `CLAUDE.md` | 修正（開発サーバー起動セクション） |
| `docs/70_ADR/026_開発サーバー一括起動ツールの選定.md` | 新規作成 |
| `docs/60_手順書/01_開発参画/01_開発環境構築.md` | 修正（セクション追加・番号繰り下げ） |

## 検証方法

```bash
just check-tools        # mprocs のインストール確認が含まれることを検証
just dev-all            # 全サーバーが mprocs TUI 内で起動することを確認
just check-all          # lint + test がパスすることを確認
```
