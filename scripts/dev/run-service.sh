#!/usr/bin/env bash
set -euo pipefail

# ビルド済みバイナリを監視して起動する（dev-all で使用）
# Usage: run-service.sh <service-name>
#   例: run-service.sh bff

SERVICE="${1:?Usage: run-service.sh <service-name>}"
BINARY="target/debug/ringiflow-${SERVICE}"

cd backend

while [ ! -f "$BINARY" ]; do
    echo "Waiting for $BINARY to be built..."
    sleep 2
done

exec cargo watch --no-vcs-ignores -w "$BINARY" -s "./$BINARY"
