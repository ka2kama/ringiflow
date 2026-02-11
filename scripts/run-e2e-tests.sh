#!/usr/bin/env bash
# =============================================================================
# E2E テスト実行スクリプト
#
# バックエンドサービスと Vite 開発サーバーを起動し、
# Playwright で E2E テストを実行する。
# テスト終了後、バックグラウンドプロセスを自動で停止する。
#
# 使い方:
#   ./scripts/run-e2e-tests.sh
#
# 前提条件:
#   - API テスト用の DB/Redis が起動済み（just api-test-deps）
#   - マイグレーションが適用済み（just api-test-reset-db）
#   - backend/.env.api-test が存在すること
#   - フロントエンド依存がインストール済み（cd frontend && pnpm install）
#   - Playwright がインストール済み（cd tests/e2e && pnpm install && npx playwright install chromium）
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# バックグラウンドプロセスを終了するトラップ
# shellcheck disable=SC2046
trap 'kill $(jobs -p) 2>/dev/null' EXIT

echo "バックエンドサービスを起動中..."

# API テスト環境変数でバックエンドサービスを起動（バックグラウンド）
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
echo "バックエンドサービス起動を待機中..."
cd "$PROJECT_ROOT"

for i in {1..60}; do
    if curl -sf http://localhost:14000/health > /dev/null 2>&1 && \
       curl -sf http://localhost:14001/health > /dev/null 2>&1 && \
       curl -sf http://localhost:14002/health > /dev/null 2>&1; then
        echo "✓ バックエンドサービス起動完了"
        break
    fi
    if [ "$i" -eq 60 ]; then
        echo "エラー: バックエンドサービス起動タイムアウト" >&2
        exit 1
    fi
    sleep 1
done

# Vite 開発サーバーを起動（BFF_PORT=14000 でプロキシ先を API テスト BFF に向ける）
echo "Vite 開発サーバーを起動中..."
cd "$PROJECT_ROOT/frontend"
VITE_PORT=15173 BFF_PORT=14000 pnpm run dev &

# Vite 開発サーバーの起動を待機
cd "$PROJECT_ROOT"
for i in {1..30}; do
    if curl -sf http://localhost:15173 > /dev/null 2>&1; then
        echo "✓ Vite 開発サーバー起動完了"
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "エラー: Vite 開発サーバー起動タイムアウト" >&2
        exit 1
    fi
    sleep 1
done

# E2E テスト実行
echo "E2E テストを実行中..."
cd "$PROJECT_ROOT/tests/e2e"
E2E_BASE_URL=http://localhost:15173 npx playwright test
