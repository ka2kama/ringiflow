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
# FIXME: SC2046 - trap 内のコマンド置換はクォートすると kill に複数 PID を渡せない
# shellcheck disable=SC2046
trap 'kill $(jobs -p) 2>/dev/null' EXIT

# API テスト環境変数でバックエンドサービスを起動（バックグラウンド）
cd "$PROJECT_ROOT/backend"

# cargo-watch 検知: 同一 workspace で実行中だとパッケージキャッシュのロック競合が発生するため
for pid in $(pgrep -x cargo-watch 2>/dev/null); do
    cwd="$(readlink /proc/"$pid"/cwd 2>/dev/null)"
    if [[ "$cwd" == "$PROJECT_ROOT" || "$cwd" == "$PROJECT_ROOT"/* ]]; then
        echo "エラー: cargo-watch が実行中のため、Cargo パッケージキャッシュのロック競合が発生します。" >&2
        echo "開発サーバーを停止してから再実行してください（just dev-down または mprocs を終了）。" >&2
        exit 1
    fi
done

# .env.api-test から環境変数を読み込み（空行とコメント行を除外）
env_vars=$(grep -Ev '^\s*$|^\s*#' .env.api-test | xargs)

# ビルドフェーズ: コンパイルを事前に完了させ、起動タイムアウトを防ぐ
echo "バックエンドサービスをビルド中..."
cargo build -p ringiflow-bff -p ringiflow-core-service -p ringiflow-auth-service

# 起動フェーズ: ビルド済みバイナリを使うため即座に起動する
echo "バックエンドサービスを起動中..."
# FIXME: SC2086 - env_vars はスペース区切りで複数の KEY=VALUE を含むため、意図的に分割展開する
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
