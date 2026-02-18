#!/usr/bin/env bash
# =============================================================================
# API テスト用の DB をリセットする
#
# dotenv-load は just 起動時に1回のみ読み込むため、
# .env が起動後に生成された場合にも対応するため直接 source する。
#
# 使い方:
#   ./scripts/api-test-reset-db.sh
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

set -a
# shellcheck disable=SC1091
source .env
set +a

echo "API テスト用データベースをリセット中..."
cd backend && DATABASE_URL="postgres://ringiflow:ringiflow@localhost:${API_TEST_POSTGRES_PORT}/ringiflow" sqlx database reset -y
echo "✓ API テスト用データベースリセット完了"
