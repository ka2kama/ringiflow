#!/usr/bin/env bash
# =============================================================================
# worktree 用の .env ファイルを生成する
#
# 使い方:
#   ./scripts/generate-env.sh [PORT_OFFSET]
#
# 引数:
#   PORT_OFFSET: ポートオフセット（0-9）。省略時は 0（メインworktree用）
#
# 例:
#   ./scripts/generate-env.sh      # メインworktree用（オフセット 0）
#   ./scripts/generate-env.sh 1    # worktree 1 用（オフセット +100）
#   ./scripts/generate-env.sh 2    # worktree 2 用（オフセット +200）
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

PORT_OFFSET="${1:-0}"

# オフセットの検証（0-9）
if ! [[ "$PORT_OFFSET" =~ ^[0-9]$ ]]; then
    echo "エラー: PORT_OFFSET は 0-9 の数字である必要があります" >&2
    exit 1
fi

# 基準ポート（メインworktree用）
BASE_POSTGRES_PORT=15432
BASE_REDIS_PORT=16379
BASE_BFF_PORT=13000
BASE_CORE_PORT=13001
BASE_AUTH_PORT=13002
BASE_VITE_PORT=15173

# オフセット計算（100 単位）
OFFSET=$((PORT_OFFSET * 100))

POSTGRES_PORT=$((BASE_POSTGRES_PORT + OFFSET))
REDIS_PORT=$((BASE_REDIS_PORT + OFFSET))
BFF_PORT=$((BASE_BFF_PORT + OFFSET))
CORE_PORT=$((BASE_CORE_PORT + OFFSET))
AUTH_PORT=$((BASE_AUTH_PORT + OFFSET))
VITE_PORT=$((BASE_VITE_PORT + OFFSET))

# ルート .env を生成
cat > "$PROJECT_ROOT/.env" << EOF
# =============================================================================
# RingiFlow ローカル開発環境設定（共通）
# =============================================================================
# このファイルは scripts/generate-env.sh により自動生成されました
# ポートオフセット: $PORT_OFFSET（+${OFFSET}）

# -----------------------------------------------------------------------------
# Docker Compose ポート設定
# -----------------------------------------------------------------------------
POSTGRES_PORT=$POSTGRES_PORT
REDIS_PORT=$REDIS_PORT

# -----------------------------------------------------------------------------
# 開発サーバーポート設定
# -----------------------------------------------------------------------------
BFF_PORT=$BFF_PORT
VITE_PORT=$VITE_PORT
EOF

# backend/.env を生成
cat > "$PROJECT_ROOT/backend/.env" << EOF
# =============================================================================
# RingiFlow バックエンド設定
# =============================================================================
# このファイルは scripts/generate-env.sh により自動生成されました
# ポートオフセット: $PORT_OFFSET（+${OFFSET}）

# -----------------------------------------------------------------------------
# データベース接続
# -----------------------------------------------------------------------------
DATABASE_URL=postgres://ringiflow:ringiflow@localhost:$POSTGRES_PORT/ringiflow_dev

# -----------------------------------------------------------------------------
# Redis 接続
# -----------------------------------------------------------------------------
REDIS_URL=redis://localhost:$REDIS_PORT

# -----------------------------------------------------------------------------
# BFF サーバー設定
# -----------------------------------------------------------------------------
BFF_HOST=0.0.0.0

# -----------------------------------------------------------------------------
# Core Service サーバー設定
# -----------------------------------------------------------------------------
CORE_HOST=0.0.0.0
CORE_PORT=$CORE_PORT

# -----------------------------------------------------------------------------
# Auth Service サーバー設定
# -----------------------------------------------------------------------------
AUTH_HOST=0.0.0.0
AUTH_PORT=$AUTH_PORT

# -----------------------------------------------------------------------------
# BFF から Core Service への接続
# -----------------------------------------------------------------------------
CORE_URL=http://localhost:$CORE_PORT

# -----------------------------------------------------------------------------
# BFF から Auth Service への接続
# -----------------------------------------------------------------------------
AUTH_URL=http://localhost:$AUTH_PORT

# -----------------------------------------------------------------------------
# ログ・環境設定
# -----------------------------------------------------------------------------
RUST_LOG=info,ringiflow=debug
ENVIRONMENT=development
EOF

echo "✓ .env ファイルを生成しました（ポートオフセット: $PORT_OFFSET）"
echo "  PostgreSQL:     $POSTGRES_PORT"
echo "  Redis:          $REDIS_PORT"
echo "  BFF:            $BFF_PORT"
echo "  Core Service:   $CORE_PORT"
echo "  Auth Service:   $AUTH_PORT"
echo "  Vite:           $VITE_PORT"
