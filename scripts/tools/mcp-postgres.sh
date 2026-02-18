#!/usr/bin/env bash
# MCP PostgreSQL サーバー起動スクリプト
#
# Volta + Node.js v24 環境では、npx --package で一時インストールしたパッケージを
# ESM の import で解決できない問題がある（NODE_PATH は ESM で無視される）。
# 回避策として、決定論的なディレクトリにインストールし、そこを cwd にして起動する。
#
# 詳細: docs/05_ADR/028_MCPサーバー導入（PostgreSQL）.md
set -euo pipefail

# プロジェクトルートの .env からポート設定を読み込む（worktree 対応）
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
if [ -f "$PROJECT_ROOT/.env" ]; then
    # shellcheck source=/dev/null
    source "$PROJECT_ROOT/.env"
fi

export DATABASE_URL="${DATABASE_URL:-postgresql://ringiflow:ringiflow@localhost:${POSTGRES_PORT:-15432}/ringiflow}"

PACKAGE="@zeddotdev/postgres-context-server"
INSTALL_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/ringiflow-mcp-postgres"

# 初回のみインストール
if [ ! -d "$INSTALL_DIR/node_modules/$PACKAGE" ]; then
    mkdir -p "$INSTALL_DIR"
    npm install --prefix "$INSTALL_DIR" "$PACKAGE" >&2
fi

cd "$INSTALL_DIR"
exec node --input-type=module -e "import '$PACKAGE'"
