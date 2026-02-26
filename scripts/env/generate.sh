#!/usr/bin/env bash
# =============================================================================
# worktree 用の .env ファイルを生成する
#
# 使い方:
#   ./scripts/env/generate.sh [PORT_OFFSET]
#
# 引数:
#   PORT_OFFSET: ポートオフセット（0-9）。省略時は 0（メインworktree用）
#
# 例:
#   ./scripts/env/generate.sh      # メインworktree用（オフセット 0）
#   ./scripts/env/generate.sh 1    # worktree 1 用（オフセット +100）
#   ./scripts/env/generate.sh 2    # worktree 2 用（オフセット +200）
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

PORT_OFFSET="${1:-0}"

# オフセットの検証（0-9）
if ! [[ "$PORT_OFFSET" =~ ^[0-9]$ ]]; then
    echo "エラー: PORT_OFFSET は 0-9 の数字である必要があります" >&2
    exit 1
fi

# 基準ポート（メインworktree用）— 開発環境
BASE_POSTGRES_PORT=15432
BASE_REDIS_PORT=16379
BASE_DYNAMODB_PORT=18000
BASE_BFF_PORT=13000
BASE_CORE_PORT=13001
BASE_AUTH_PORT=13002
BASE_VITE_PORT=15173
BASE_MAILPIT_SMTP_PORT=11025
BASE_MAILPIT_UI_PORT=18025

# 基準ポート（メインworktree用）— API テスト環境
BASE_API_TEST_POSTGRES_PORT=15433
BASE_API_TEST_REDIS_PORT=16380
BASE_API_TEST_DYNAMODB_PORT=18001
BASE_API_TEST_BFF_PORT=14000
BASE_API_TEST_CORE_PORT=14001
BASE_API_TEST_AUTH_PORT=14002
BASE_API_TEST_VITE_PORT=15174
BASE_API_TEST_MAILPIT_SMTP_PORT=11026
BASE_API_TEST_MAILPIT_UI_PORT=18026

# オフセット計算（100 単位）
OFFSET=$((PORT_OFFSET * 100))

# 開発環境ポート
POSTGRES_PORT=$((BASE_POSTGRES_PORT + OFFSET))
REDIS_PORT=$((BASE_REDIS_PORT + OFFSET))
DYNAMODB_PORT=$((BASE_DYNAMODB_PORT + OFFSET))
BFF_PORT=$((BASE_BFF_PORT + OFFSET))
CORE_PORT=$((BASE_CORE_PORT + OFFSET))
AUTH_PORT=$((BASE_AUTH_PORT + OFFSET))
VITE_PORT=$((BASE_VITE_PORT + OFFSET))
MAILPIT_SMTP_PORT=$((BASE_MAILPIT_SMTP_PORT + OFFSET))
MAILPIT_UI_PORT=$((BASE_MAILPIT_UI_PORT + OFFSET))

# API テスト環境ポート
API_TEST_POSTGRES_PORT=$((BASE_API_TEST_POSTGRES_PORT + OFFSET))
API_TEST_REDIS_PORT=$((BASE_API_TEST_REDIS_PORT + OFFSET))
API_TEST_DYNAMODB_PORT=$((BASE_API_TEST_DYNAMODB_PORT + OFFSET))
API_TEST_BFF_PORT=$((BASE_API_TEST_BFF_PORT + OFFSET))
API_TEST_CORE_PORT=$((BASE_API_TEST_CORE_PORT + OFFSET))
API_TEST_AUTH_PORT=$((BASE_API_TEST_AUTH_PORT + OFFSET))
API_TEST_VITE_PORT=$((BASE_API_TEST_VITE_PORT + OFFSET))
API_TEST_MAILPIT_SMTP_PORT=$((BASE_API_TEST_MAILPIT_SMTP_PORT + OFFSET))
API_TEST_MAILPIT_UI_PORT=$((BASE_API_TEST_MAILPIT_UI_PORT + OFFSET))

# ルート .env を生成
cat > "$PROJECT_ROOT/.env" << EOF
# =============================================================================
# RingiFlow ローカル開発環境設定（共通）
# =============================================================================
# このファイルは scripts/env/generate.sh により自動生成されました
# ポートオフセット: $PORT_OFFSET（+${OFFSET}）

# -----------------------------------------------------------------------------
# Docker Compose ポート設定
# -----------------------------------------------------------------------------
POSTGRES_PORT=$POSTGRES_PORT
REDIS_PORT=$REDIS_PORT
DYNAMODB_PORT=$DYNAMODB_PORT
MAILPIT_SMTP_PORT=$MAILPIT_SMTP_PORT
MAILPIT_UI_PORT=$MAILPIT_UI_PORT

# -----------------------------------------------------------------------------
# Docker Compose ポート設定（API テスト用）
# -----------------------------------------------------------------------------
API_TEST_POSTGRES_PORT=$API_TEST_POSTGRES_PORT
API_TEST_REDIS_PORT=$API_TEST_REDIS_PORT
API_TEST_DYNAMODB_PORT=$API_TEST_DYNAMODB_PORT
API_TEST_MAILPIT_SMTP_PORT=$API_TEST_MAILPIT_SMTP_PORT
API_TEST_MAILPIT_UI_PORT=$API_TEST_MAILPIT_UI_PORT

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
# このファイルは scripts/env/generate.sh により自動生成されました
# ポートオフセット: $PORT_OFFSET（+${OFFSET}）

# -----------------------------------------------------------------------------
# データベース接続
# -----------------------------------------------------------------------------
DATABASE_URL=postgres://ringiflow:ringiflow@localhost:$POSTGRES_PORT/ringiflow

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
# DynamoDB 接続（監査ログ）
# -----------------------------------------------------------------------------
DYNAMODB_ENDPOINT=http://localhost:$DYNAMODB_PORT

# -----------------------------------------------------------------------------
# ログ・環境設定
# -----------------------------------------------------------------------------
RUST_LOG=info,ringiflow=debug
ENVIRONMENT=development

# -----------------------------------------------------------------------------
# 開発用認証バイパス（DevAuth）
# ログイン画面なしで認証済み状態を実現する
# 詳細: docs/06_ナレッジベース/security/DevAuth.md
# -----------------------------------------------------------------------------
DEV_AUTH_ENABLED=true

# -----------------------------------------------------------------------------
# 通知設定
# 詳細: docs/03_詳細設計書/16_通知機能設計.md
# -----------------------------------------------------------------------------
NOTIFICATION_BACKEND=smtp
SMTP_HOST=localhost
SMTP_PORT=$MAILPIT_SMTP_PORT
NOTIFICATION_FROM_ADDRESS=noreply@ringiflow.example.com
NOTIFICATION_BASE_URL=http://localhost:$VITE_PORT
EOF

# backend/.env.api-test を生成
cat > "$PROJECT_ROOT/backend/.env.api-test" << EOF
# =============================================================================
# RingiFlow API テスト環境設定
# =============================================================================
# このファイルは scripts/env/generate.sh により自動生成されました
# ポートオフセット: $PORT_OFFSET（+${OFFSET}）
# 開発環境とは独立した DB/Redis/ポートを使用する。
# just test-api / just test-e2e で自動的にこの設定が読み込まれる。

# -----------------------------------------------------------------------------
# データベース接続（API テスト専用）
# -----------------------------------------------------------------------------
DATABASE_URL=postgres://ringiflow:ringiflow@localhost:$API_TEST_POSTGRES_PORT/ringiflow

# -----------------------------------------------------------------------------
# Redis 接続（API テスト専用）
# -----------------------------------------------------------------------------
REDIS_URL=redis://localhost:$API_TEST_REDIS_PORT

# -----------------------------------------------------------------------------
# BFF サーバー設定
# -----------------------------------------------------------------------------
BFF_HOST=0.0.0.0
BFF_PORT=$API_TEST_BFF_PORT

# -----------------------------------------------------------------------------
# Core Service サーバー設定
# -----------------------------------------------------------------------------
CORE_HOST=0.0.0.0
CORE_PORT=$API_TEST_CORE_PORT

# -----------------------------------------------------------------------------
# Auth Service サーバー設定
# -----------------------------------------------------------------------------
AUTH_HOST=0.0.0.0
AUTH_PORT=$API_TEST_AUTH_PORT

# -----------------------------------------------------------------------------
# BFF から Core Service への接続
# -----------------------------------------------------------------------------
CORE_URL=http://localhost:$API_TEST_CORE_PORT

# -----------------------------------------------------------------------------
# BFF から Auth Service への接続
# -----------------------------------------------------------------------------
AUTH_URL=http://localhost:$API_TEST_AUTH_PORT

# -----------------------------------------------------------------------------
# DynamoDB 接続（監査ログ、API テスト専用）
# -----------------------------------------------------------------------------
DYNAMODB_ENDPOINT=http://localhost:$API_TEST_DYNAMODB_PORT

# -----------------------------------------------------------------------------
# E2E テスト用 Vite ポート
# -----------------------------------------------------------------------------
E2E_VITE_PORT=$API_TEST_VITE_PORT

# -----------------------------------------------------------------------------
# ログ・環境設定
# -----------------------------------------------------------------------------
RUST_LOG=warn,ringiflow=info
ENVIRONMENT=test

# -----------------------------------------------------------------------------
# 通知設定（API テスト用）
# -----------------------------------------------------------------------------
NOTIFICATION_BACKEND=smtp
SMTP_HOST=localhost
SMTP_PORT=$API_TEST_MAILPIT_SMTP_PORT
NOTIFICATION_FROM_ADDRESS=noreply@ringiflow.example.com
NOTIFICATION_BASE_URL=http://localhost:$API_TEST_VITE_PORT
EOF

echo "✓ .env ファイルを生成しました（ポートオフセット: $PORT_OFFSET）"
echo "  [開発環境]"
echo "    PostgreSQL:     $POSTGRES_PORT"
echo "    Redis:          $REDIS_PORT"
echo "    DynamoDB:       $DYNAMODB_PORT"
echo "    BFF:            $BFF_PORT"
echo "    Core Service:   $CORE_PORT"
echo "    Auth Service:   $AUTH_PORT"
echo "    Vite:           $VITE_PORT"
echo "    Mailpit SMTP:  $MAILPIT_SMTP_PORT"
echo "    Mailpit UI:    $MAILPIT_UI_PORT"
echo "  [API テスト環境]"
echo "    PostgreSQL:     $API_TEST_POSTGRES_PORT"
echo "    Redis:          $API_TEST_REDIS_PORT"
echo "    DynamoDB:       $API_TEST_DYNAMODB_PORT"
echo "    BFF:            $API_TEST_BFF_PORT"
echo "    Core Service:   $API_TEST_CORE_PORT"
echo "    Auth Service:   $API_TEST_AUTH_PORT"
echo "    E2E Vite:       $API_TEST_VITE_PORT"
echo "    Mailpit SMTP:  $API_TEST_MAILPIT_SMTP_PORT"
echo "    Mailpit UI:    $API_TEST_MAILPIT_UI_PORT"
