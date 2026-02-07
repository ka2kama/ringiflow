# MCP PostgreSQL サーバー導入 + justfile ヘルパーコマンド

## 概要

Claude Code が開発中に PostgreSQL スキーマ・データと Redis データを効率的に参照できるようにする。
MCP サーバーによるリアルタイムアクセスと、justfile コマンドによる手軽な操作の両方を提供する。

## MCP パッケージ選定

公式 `@modelcontextprotocol/server-postgres` は 2025年7月に非推奨。
代替として **`@zeddotdev/postgres-context-server`** を採用する。

| 観点 | 公式（非推奨） | Zed fork（採用） |
|------|---------------|-----------------|
| メンテナンス | 非推奨（2025/07） | アクティブ（3日前に更新、v0.1.7） |
| セキュリティ | SQL injection 脆弱性あり | パッチ済み |
| インストール | npx | npx |
| 機能 | list_tables, describe_table, query | スキーマ introspection + query |

選定理由:
- 「最新ベストプラクティス採用方針」に準拠（非推奨パッケージを避ける）
- Zed Industries による継続的メンテナンス
- SQL injection 脆弱性がパッチ済み
- npm ベースでセットアップが簡単（KISS）

## 変更ファイル一覧

| ファイル | 操作 | 内容 |
|---------|------|------|
| `.mcp.json` | 新規作成 | PostgreSQL MCP サーバー設定 |
| `justfile` | 編集 | データストア操作セクション追加（5コマンド） |
| `.claude/settings.json` | 編集 | 新 just コマンドの許可追加 |
| `docs/05_ADR/028_MCPサーバー導入（PostgreSQL）.md` | 新規作成 | 技術選定の記録 |
| `docs/05_ADR/009_MCPサーバー導入の見送り.md` | 編集 | ステータスを「廃止」に更新 |
| `CLAUDE.md` | 編集 | データストア操作セクション追加 |

## Step 1: `.mcp.json` を作成

```json
{
  "mcpServers": {
    "postgres": {
      "command": "npx",
      "args": ["-y", "@zeddotdev/postgres-context-server"],
      "env": {
        "DATABASE_URL": "postgresql://ringiflow:ringiflow@localhost:15432/ringiflow_dev"
      }
    }
  }
}
```

- `DATABASE_URL` 環境変数で接続先を指定（Zed fork のインターフェース）
- `npx -y` で自動インストール
- Redis MCP は導入しない（Python 依存を避ける、セッション/CSRF のみの用途）

## Step 2: justfile にデータストア操作セクション追加

挿入位置: `dev-down` レシピ（141行目）の後、フォーマットセクション（142行目）の前

```just
# =============================================================================
# データストア操作（開発用）
# =============================================================================

# PostgreSQL: テーブル一覧を表示
db-tables:
    @psql "postgres://ringiflow:ringiflow@localhost:${POSTGRES_PORT}/ringiflow_dev" \
        -c "\dt public.*" --pset="footer=off"

# PostgreSQL: 指定テーブルのカラム定義を表示
db-schema table:
    @psql "postgres://ringiflow:ringiflow@localhost:${POSTGRES_PORT}/ringiflow_dev" \
        -c "\d {{table}}"

# PostgreSQL: 任意の SQL を実行
db-query sql:
    @psql "postgres://ringiflow:ringiflow@localhost:${POSTGRES_PORT}/ringiflow_dev" \
        -c "{{sql}}"

# Redis: キー一覧を表示（パターンで絞り込み可能、デフォルト: *）
redis-keys pattern='*':
    @redis-cli -p "${REDIS_PORT}" keys "{{pattern}}"

# Redis: 指定キーの値を取得
redis-get key:
    @redis-cli -p "${REDIS_PORT}" get "{{key}}"
```

- `$POSTGRES_PORT` / `$REDIS_PORT` で worktree 対応（`.env` から自動読み込み）
- `@` プレフィックスでコマンド自体の出力を抑制（結果のみ表示）

## Step 3: `.claude/settings.json` に許可追加

`permissions.allow` 配列の末尾（既存の just コマンド群の後）に追加:

```json
"Bash(just db-tables)",
"Bash(just db-schema:*)",
"Bash(just db-query:*)",
"Bash(just redis-keys:*)",
"Bash(just redis-get:*)"
```

## Step 4: ADR-028 を作成

ファイル: `docs/05_ADR/028_MCPサーバー導入（PostgreSQL）.md`

- ADR テンプレート（`template.md`）に準拠
- ADR-009 を置換する決定を記録
- 選択肢:
  1. `@zeddotdev/postgres-context-server` を導入する（採用）
  2. `@modelcontextprotocol/server-postgres`（公式・非推奨）を導入する
  3. MCP なし、justfile コマンドのみで対応する

## Step 5: ADR-009 を廃止

ステータスを「廃止（2026-01-31） → ADR-028」に変更。変更履歴に追記。

## Step 6: CLAUDE.md にデータストア操作セクション追加

「開発コマンド」セクション内（`just check-all` の説明後あたり）に追記:

```markdown
### データストア操作

PostgreSQL スキーマやデータの確認に使用する。MCP（PostgreSQL）も利用可能。

\```bash
just db-tables              # テーブル一覧
just db-schema テーブル名    # カラム定義
just db-query "SELECT ..."  # SQL 実行
just redis-keys             # Redis キー一覧
just redis-keys "session:*" # パターン指定
just redis-get キー名        # Redis 値取得
\```
```

## 検証手順

1. `just dev-deps` で PostgreSQL/Redis を起動
2. justfile コマンドの動作確認:
   - `just db-tables` → テーブル一覧が表示される
   - `just db-schema users` → users テーブルのカラム定義が表示される
   - `just db-query "SELECT count(*) FROM users"` → SQL 実行結果が表示される
   - `just redis-keys` → Redis キーが表示される（データがあれば）
3. Claude Code を再起動し、MCP ツール（postgres）が認識されることを確認
