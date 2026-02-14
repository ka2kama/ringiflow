#!/bin/bash
# Lightsail デモ環境へのデプロイ
#
# GitHub Actions の deploy-demo ワークフローから呼び出される。
# 設定ファイルの転送、イメージの pull、サービスの再起動を行う。
#
# 必要な環境変数:
#   SSH_KEY         - SSH 秘密鍵
#   DEPLOY_HOST     - Lightsail のホスト名/IP
#   DEPLOY_USER     - SSH ユーザー名
#   GH_TOKEN        - GHCR 認証用の GitHub トークン
#   BACKEND_IMAGE   - バックエンド GHCR イメージ名
#   FRONTEND_IMAGE  - フロントエンド GHCR イメージ名

set -euo pipefail

: "${SSH_KEY:?SSH_KEY is required}"
: "${DEPLOY_HOST:?DEPLOY_HOST is required}"
: "${DEPLOY_USER:?DEPLOY_USER is required}"
: "${GH_TOKEN:?GH_TOKEN is required}"
: "${BACKEND_IMAGE:?BACKEND_IMAGE is required}"
: "${FRONTEND_IMAGE:?FRONTEND_IMAGE is required}"

# SSH 鍵をセットアップ
mkdir -p ~/.ssh
echo "$SSH_KEY" > ~/.ssh/deploy_key
chmod 600 ~/.ssh/deploy_key
SSH_OPTS="-i ~/.ssh/deploy_key -o StrictHostKeyChecking=accept-new"

# 設定ファイルを転送（tar over SSH）
STAGING=$(mktemp -d)
trap 'rm -rf "$STAGING"' EXIT

cp infra/lightsail/docker-compose.yaml "$STAGING/"
mkdir -p "$STAGING/config/nginx/conf.d" "$STAGING/config/init"
cp infra/lightsail/nginx/nginx.conf "$STAGING/config/nginx/"
cp infra/lightsail/nginx/conf.d/default.conf "$STAGING/config/nginx/conf.d/"
cp infra/lightsail/init/01_extensions.sql "$STAGING/config/init/"

# shellcheck disable=SC2086
tar -czf - -C "$STAGING" . \
  | ssh $SSH_OPTS "${DEPLOY_USER}@${DEPLOY_HOST}" \
      "mkdir -p ~/ringiflow && cd ~/ringiflow && tar -xzf -"

# Lightsail 上でデプロイ実行
# shellcheck disable=SC2029,SC2086,SC2087
ssh $SSH_OPTS "${DEPLOY_USER}@${DEPLOY_HOST}" << DEPLOY_EOF
trap 'docker logout ghcr.io 2>/dev/null' EXIT
set -euo pipefail
cd ~/ringiflow

echo "[INFO] GHCR にログイン..."
echo "${GH_TOKEN}" | docker login ghcr.io --username github-actions --password-stdin

echo "[INFO] イメージを pull..."
docker pull ${BACKEND_IMAGE}:latest
docker pull ${FRONTEND_IMAGE}:latest

echo "[INFO] フロントエンド静的ファイルを更新..."
docker volume create ringiflow-frontend-dist 2>/dev/null || true
docker run --rm \
  -v ringiflow-frontend-dist:/dist \
  ${FRONTEND_IMAGE}:latest \
  sh -c "rm -rf /dist/* && cp -r /app/dist/* /dist/"

echo "[INFO] サービスを再起動..."
docker compose up -d --remove-orphans

echo "[INFO] ヘルスチェック..."
sleep 15
curl -sf http://localhost/health && echo " - Nginx: OK" || echo " - Nginx: NG"
curl -sf http://localhost/api/health && echo " - BFF: OK" || echo " - BFF: NG"
DEPLOY_EOF
