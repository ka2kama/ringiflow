#!/usr/bin/env bash
# =============================================================================
# E2E テスト実行スクリプト
#
# バックエンドサービスと Vite 開発サーバーを起動し、
# Playwright で E2E テストを実行する。
# テスト終了後、バックグラウンドプロセスを自動で停止する。
#
# 使い方:
#   ./scripts/test/run-e2e.sh
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
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

# バックグラウンドプロセスを終了するトラップ
# kill 後に wait で終了を待ち、関数の stderr を抑制して ELIFECYCLE ノイズを除去する
cleanup() {
    local pids
    # shellcheck disable=SC2046
    pids=$(jobs -p 2>/dev/null) || true
    if [ -n "$pids" ]; then
        # shellcheck disable=SC2086
        kill $pids 2>/dev/null || true
        # shellcheck disable=SC2086
        wait $pids 2>/dev/null || true
    fi
} 2>/dev/null
trap cleanup EXIT

# API テスト環境変数を読み込み
cd "$PROJECT_ROOT/backend"
set -a
# shellcheck disable=SC1091
source .env.api-test
set +a

# cargo-watch 検知: 同一 workspace で実行中だとパッケージキャッシュのロック競合が発生するため
for pid in $(pgrep -x cargo-watch 2>/dev/null); do
    cwd="$(readlink /proc/"$pid"/cwd 2>/dev/null)"
    if [[ "$cwd" == "$PROJECT_ROOT" || "$cwd" == "$PROJECT_ROOT"/* ]]; then
        echo "エラー: cargo-watch が実行中のため、Cargo パッケージキャッシュのロック競合が発生します。" >&2
        echo "開発サーバーを停止してから再実行してください（just dev-down または mprocs を終了）。" >&2
        exit 1
    fi
done

# ビルドフェーズ: コンパイルを事前に完了させ、起動タイムアウトを防ぐ
echo "バックエンドサービスをビルド中..."
cargo build -p ringiflow-bff -p ringiflow-core-service -p ringiflow-auth-service

# 起動フェーズ: ビルド済みバイナリを使うため即座に起動する
echo "バックエンドサービスを起動中..."
./target/debug/ringiflow-bff &
./target/debug/ringiflow-core-service &
./target/debug/ringiflow-auth-service &

# ヘルスチェックを待機
echo "バックエンドサービス起動を待機中..."
cd "$PROJECT_ROOT"

for i in {1..60}; do
    if curl -sf "http://localhost:$BFF_PORT/health" > /dev/null 2>&1 && \
       curl -sf "http://localhost:$CORE_PORT/health" > /dev/null 2>&1 && \
       curl -sf "http://localhost:$AUTH_PORT/health" > /dev/null 2>&1; then
        echo "✓ バックエンドサービス起動完了"
        break
    fi
    if [ "$i" -eq 60 ]; then
        echo "エラー: バックエンドサービス起動タイムアウト" >&2
        exit 1
    fi
    sleep 1
done

# Vite 開発サーバーを起動（BFF_PORT でプロキシ先を API テスト BFF に向ける）
echo "Vite 開発サーバーを起動中..."
cd "$PROJECT_ROOT/frontend"
VITE_PORT=$E2E_VITE_PORT pnpm run dev &

# Vite 開発サーバーの起動を待機
cd "$PROJECT_ROOT"
for i in {1..30}; do
    if curl -sf "http://localhost:$E2E_VITE_PORT" > /dev/null 2>&1; then
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
E2E_BASE_URL="http://localhost:$E2E_VITE_PORT" npx playwright test
