#!/usr/bin/env bash
# =============================================================================
# API テスト実行スクリプト
#
# バックエンドサービスを起動し、hurl で API テストを実行する。
# テスト終了後、バックグラウンドプロセスを自動で停止する。
#
# 使い方:
#   ./scripts/run-api-tests.sh
#
# 前提条件:
#   - API テスト用の DB/Redis が起動済み（just api-test-deps）
#   - マイグレーションが適用済み（just api-test-reset-db）
#   - backend/.env.api-test が存在すること
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# バックグラウンドプロセスを終了するトラップ
# shellcheck disable=SC2046
trap 'kill $(jobs -p) 2>/dev/null' EXIT

echo "サービスを起動中..."

# API テスト環境変数でサービスを起動（バックグラウンド）
cd "$PROJECT_ROOT/backend"

# .env.api-test から環境変数を読み込み（空行とコメント行を除外）
env_vars=$(grep -Ev '^\s*$|^\s*#' .env.api-test | xargs)

# shellcheck disable=SC2086
env $env_vars cargo run -p ringiflow-bff &
# shellcheck disable=SC2086
env $env_vars cargo run -p ringiflow-core-service &
# shellcheck disable=SC2086
env $env_vars cargo run -p ringiflow-auth-service &

# ヘルスチェックを待機（API テスト用ポート: 14000-14002）
echo "サービス起動を待機中..."
cd "$PROJECT_ROOT"

for i in {1..30}; do
    if curl -sf http://localhost:14000/health > /dev/null 2>&1 && \
       curl -sf http://localhost:14001/health > /dev/null 2>&1 && \
       curl -sf http://localhost:14002/health > /dev/null 2>&1; then
        echo "✓ 全サービス起動完了"
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "エラー: サービス起動タイムアウト" >&2
        exit 1
    fi
    sleep 1
done

# API テスト実行
echo "API テストを実行中..."
hurl --test --jobs 1 --variables-file tests/api/hurl/vars.env tests/api/hurl/**/*.hurl
