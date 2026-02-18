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
echo "サービスをビルド中..."
cargo build -p ringiflow-bff -p ringiflow-core-service -p ringiflow-auth-service

# 起動フェーズ: ビルド済みバイナリを使うため即座に起動する
echo "サービスを起動中..."
./target/debug/ringiflow-bff &
./target/debug/ringiflow-core-service &
./target/debug/ringiflow-auth-service &

# ヘルスチェックを待機
echo "サービス起動を待機中..."
cd "$PROJECT_ROOT"

for i in {1..30}; do
    if curl -sf "http://localhost:$BFF_PORT/health" > /dev/null 2>&1 && \
       curl -sf "http://localhost:$CORE_PORT/health" > /dev/null 2>&1 && \
       curl -sf "http://localhost:$AUTH_PORT/health" > /dev/null 2>&1; then
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
hurl --test --jobs 1 \
    --variable "bff_url=http://localhost:$BFF_PORT" \
    --variables-file tests/api/hurl/vars.env \
    tests/api/hurl/**/*.hurl
