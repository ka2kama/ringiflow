# MCP PostgreSQL サーバー導入

## 概要

Claude Code から PostgreSQL のスキーマやデータをリアルタイムに参照するため、MCP（Model Context Protocol）サーバーを導入した。合わせて justfile にデータストア操作コマンドを追加した。

## 背景と目的

- 開発が進み、DB スキーマの参照頻度が増加していた
- ADR-009 で MCP 導入を見送っていたが、エコシステムの成熟により再検討が必要に
- 公式 `@modelcontextprotocol/server-postgres` が非推奨（2025年7月）となり、代替パッケージの選定が必要だった

## 実施内容

### 1. MCP PostgreSQL サーバーの導入

- パッケージ: `@zeddotdev/postgres-context-server`（Zed Industries フォーク版）
- 公式パッケージの SQL injection 脆弱性を修正済み
- `.mcp.json` と `.claude/settings.json` を設定

### 2. `bin` フィールド欠如への対応

パッケージに `bin` フィールドがないため `npx -y パッケージ名` が使えない問題に対応。`npx --package` + `node --input-type=module -e "import ..."` で ESM モジュールとして直接実行する回避策を実装。

### 3. Volta + Node.js v24 の ESM モジュール解決問題への対応

`npx --package` 方式が Volta + Node.js v24 環境で動作しないことが判明。Node.js の ESM モジュール解決は `NODE_PATH` を無視し、`cwd` からの `node_modules` 探索に依存するため、`npx` が一時キャッシュにインストールしたパッケージを `node` が見つけられなかった。

解決策として起動スクリプト（`scripts/mcp-postgres.sh`）を作成：

- 決定論的なディレクトリ（`$XDG_CACHE_HOME/ringiflow-mcp-postgres`）にパッケージをインストール
- そのディレクトリに `cd` してから `node` を実行

### 4. justfile データストアコマンドの追加

| コマンド | 用途 |
|---------|------|
| `just db-tables` | PostgreSQL テーブル一覧 |
| `just db-schema <テーブル名>` | カラム定義表示 |
| `just db-query "SELECT ..."` | 任意の SQL 実行 |
| `just redis-keys [pattern]` | Redis キー一覧 |
| `just redis-get <キー名>` | Redis 値取得 |

### 5. justfile リファクタリング

psql 接続文字列を justfile 変数 `_psql_url` に抽出し、3箇所の重複を解消。`env_var_or_default` を使用して CI 環境（`.env` なし）でもエラーにならないようにした。

## 設計上の判断

| 判断 | 選択 | 理由 |
|------|------|------|
| MCP パッケージ | `@zeddotdev/postgres-context-server` | SQL injection パッチ済み、アクティブメンテナンス |
| 起動方式 | スクリプトファイル | Volta + Node.js v24 の ESM 問題を回避、可読性・保守性が高い |
| Redis MCP | 導入しない | Python 依存追加、参照頻度低い、justfile で十分 |
| justfile 変数 | `env_var_or_default` | CI 環境（`.env` なし）でのフォールバック |

## 成果物

### コミット

| コミット | 内容 |
|---------|------|
| `c895da5` | MCP PostgreSQL サーバーと justfile データストアコマンドを導入 |
| `f91436e` | psql と redis-cli を check-tools とセットアップドキュメントに追加 |
| `92ce1e3` | bin フィールド欠如の回避策で MCP 起動コマンドを修正 |
| `16c4e96` | Volta + Node.js v24 の ESM モジュール解決問題に対応 |
| `7f58de8` | justfile: psql URL を変数に抽出 |
| `e1523c1` | CI 修正: POSTGRES_PORT に env_var_or_default を使用 |

### 作成・更新ファイル

- 新規: `.mcp.json`, `scripts/mcp-postgres.sh`
- 新規: `docs/70_ADR/028_MCPサーバー導入（PostgreSQL）.md`
- 更新: `.claude/settings.json`, `CLAUDE.md`, `justfile`
- 更新: `docs/60_手順書/01_開発参画/01_開発環境構築.md`
- 更新: `docs/70_ADR/009_MCPサーバー導入の見送り.md`（ステータスを「廃止」に変更）

## 議論の経緯

### MCP サーバー起動失敗の診断

`/mcp` コマンドで MCP サーバーが `failed` ステータスであることを確認。Docker コンテナの停止と、ESM モジュール解決の失敗という2つの原因を特定した。

### 起動方式の選択

ESM モジュール解決問題の回避策として3つの選択肢を検討：

1. スクリプトファイル方式（採用）— 可読性・保守性が高い
2. `bash -c` インライン方式 — 追加ファイル不要だがコマンドが長い
3. devDependencies 追加 — フロントエンドとの依存混在が懸念

ユーザーがスクリプトファイル方式を選択した。

### CI 失敗への対応

justfile の psql URL 変数化（リファクタリング）後、CI が失敗。justfile のトップレベル変数は即座に評価される（遅延評価されない）ため、`.env` がない CI 環境で `env_var("POSTGRES_PORT")` が失敗した。`env_var_or_default` で修正。

## 学んだこと

- Node.js の ESM モジュール解決は `NODE_PATH` を無視し、`cwd` からの `node_modules` 探索に依存する（CJS とは異なる動作）
- justfile のトップレベル変数は即座に評価される。レシピ内のシェル変数（実行時評価）とは異なるため、CI 環境への影響を考慮する必要がある
- justfile の `_` プレフィックス変数は `just --list` に表示されない（プライベート変数の規約）

## 次のステップ

- PR をマージ
- 上流パッケージの `bin` フィールド修正を監視し、修正されたら起動コマンドを簡素化
