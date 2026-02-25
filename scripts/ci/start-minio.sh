#!/usr/bin/env bash
set -euo pipefail

# CI 環境で MinIO コンテナを起動する。
#
# GitHub Actions の services: は docker create を使用するため、
# CMD 引数（server /data）を渡せない。docker run で直接起動する。
#
# 環境変数:
#   MINIO_PORT  - ホスト側ポート（デフォルト: 19000）
#   S3_BUCKET_NAME - 作成するバケット名（デフォルト: ringiflow-dev-documents）

MINIO_PORT="${MINIO_PORT:-19000}"
BUCKET_NAME="${S3_BUCKET_NAME:-ringiflow-dev-documents}"

echo "Starting MinIO on port ${MINIO_PORT}..."
docker run -d --name minio \
  -p "${MINIO_PORT}:9000" \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data

echo "Waiting for MinIO to be ready..."
for _ in $(seq 1 30); do
  if curl -sf "http://localhost:${MINIO_PORT}/minio/health/live" > /dev/null 2>&1; then
    echo "MinIO is ready."
    break
  fi
  sleep 1
done

# ヘルスチェック最終確認（起動失敗時に明示的にエラー終了）
if ! curl -sf "http://localhost:${MINIO_PORT}/minio/health/live" > /dev/null 2>&1; then
  echo "::error::MinIO failed to start within 30 seconds"
  docker logs minio || true
  exit 1
fi

echo "Creating bucket: ${BUCKET_NAME}"
curl -sSL https://dl.min.io/client/mc/release/linux-amd64/mc -o /usr/local/bin/mc
chmod +x /usr/local/bin/mc
mc alias set local "http://localhost:${MINIO_PORT}" minioadmin minioadmin
mc mb --ignore-existing "local/${BUCKET_NAME}"
echo "MinIO setup complete."
