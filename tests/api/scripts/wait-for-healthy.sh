#!/usr/bin/env bash
# サービスのヘルスチェックが通るまで待機するスクリプト
#
# 使用例:
#   ./wait-for-healthy.sh http://localhost:13000/health
#   ./wait-for-healthy.sh http://localhost:13001/health 60  # タイムアウト60秒

set -euo pipefail

URL="${1:-http://localhost:13000/health}"
TIMEOUT="${2:-30}"
INTERVAL=1

echo "ヘルスチェック待機: $URL"
echo "タイムアウト: ${TIMEOUT}秒"

start_time=$(date +%s)

while true; do
    current_time=$(date +%s)
    elapsed=$((current_time - start_time))

    if [ "$elapsed" -ge "$TIMEOUT" ]; then
        echo "エラー: タイムアウト（${TIMEOUT}秒）"
        exit 1
    fi

    if curl -sf "$URL" > /dev/null 2>&1; then
        echo "ヘルスチェック成功（${elapsed}秒）"
        exit 0
    fi

    echo "待機中... (${elapsed}/${TIMEOUT}秒)"
    sleep "$INTERVAL"
done
