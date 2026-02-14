#!/bin/bash
# RingiFlow デプロイスクリプト
#
# ローカルマシンで実行し、以下を行う:
# 1. Docker イメージをローカルでビルド
# 2. イメージを tar にエクスポート
# 3. SCP で Lightsail に転送
# 4. Lightsail 上で docker load + compose up
#
# 使い方:
#   ./deploy.sh              # フルデプロイ
#   ./deploy.sh --skip-build # イメージビルドをスキップ

set -euo pipefail

# プロジェクトルートに移動
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

step() {
    echo ""
    echo -e "${CYAN}=========================================="
    echo -e "$1"
    echo -e "==========================================${NC}"
}

# ==========================================================
# 設定の読み込み
# ==========================================================
ENV_FILE="$SCRIPT_DIR/.env"
if [ ! -f "$ENV_FILE" ]; then
    error ".env ファイルが見つかりません: $ENV_FILE"
fi

# shellcheck source=/dev/null
source "$ENV_FILE"

# 必須環境変数のチェック
: "${LIGHTSAIL_HOST:?LIGHTSAIL_HOST が設定されていません}"
: "${LIGHTSAIL_USER:?LIGHTSAIL_USER が設定されていません}"

# SSH オプション（配列として定義）
# LIGHTSAIL_SSH_KEY が設定されていれば -i で鍵を指定、未設定なら SSH エージェントに委任
SSH_OPTS=(-o StrictHostKeyChecking=accept-new)
if [ -n "${LIGHTSAIL_SSH_KEY:-}" ]; then
    SSH_OPTS+=(-i "${LIGHTSAIL_SSH_KEY/#\~/$HOME}")
fi
SSH_TARGET="$LIGHTSAIL_USER@$LIGHTSAIL_HOST"
REMOTE_DIR="/home/$LIGHTSAIL_USER/ringiflow"

# Docker イメージ名（GHCR と合わせる）
BACKEND_IMAGE="ghcr.io/ka2kama/ringiflow/backend"
FRONTEND_IMAGE="ghcr.io/ka2kama/ringiflow/frontend"

# オプション解析
SKIP_BUILD=false
for arg in "$@"; do
    case $arg in
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
    esac
done

# ==========================================================
# 1. Docker イメージのビルド
# ==========================================================
if [ "$SKIP_BUILD" = false ]; then
    step "Step 1: Docker イメージをビルド"

    info "Backend イメージをビルド中..."
    # CARGO_FEATURES="" で default features（dev-auth 含む）を有効化
    docker build \
        --build-arg CARGO_FEATURES="" \
        -t "$BACKEND_IMAGE:latest" \
        -f backend/Dockerfile backend/

    info "Frontend イメージをビルド中..."
    # VITE_DEV_AUTH=true で DevAuth Cookie 設定を有効化
    docker build \
        --build-arg VITE_DEV_AUTH=true \
        -t "$FRONTEND_IMAGE:latest" \
        -f frontend/Dockerfile frontend/

    # フロントエンドの静的ファイルを取り出す
    info "Frontend 静的ファイルを取り出し中..."
    docker create --name frontend-build "$FRONTEND_IMAGE:latest"
    rm -rf /tmp/ringiflow-frontend-dist
    docker cp frontend-build:/app/dist /tmp/ringiflow-frontend-dist
    docker rm frontend-build
else
    step "Step 1: ビルドをスキップ"
fi

# ==========================================================
# 2. イメージを tar にエクスポート
# ==========================================================
step "Step 2: Docker イメージをエクスポート"

EXPORT_DIR="/tmp/ringiflow-deploy"
rm -rf "$EXPORT_DIR"
mkdir -p "$EXPORT_DIR"

info "Backend イメージを保存中..."
docker save "$BACKEND_IMAGE:latest" | gzip > "$EXPORT_DIR/backend.tar.gz"

info "エクスポート完了:"
ls -lh "$EXPORT_DIR"

# ==========================================================
# 3. ファイルを Lightsail に転送
# ==========================================================
step "Step 3: Lightsail にファイルを転送"

# リモートディレクトリを作成
# shellcheck disable=SC2029
ssh "${SSH_OPTS[@]}" "$SSH_TARGET" "mkdir -p $REMOTE_DIR/{config/nginx/conf.d,images,frontend,config/init}"

info "Docker イメージを転送中..."
scp "${SSH_OPTS[@]}" "$EXPORT_DIR/backend.tar.gz" "$SSH_TARGET:$REMOTE_DIR/images/"

info "設定ファイルを転送中..."
scp "${SSH_OPTS[@]}" infra/lightsail/docker-compose.yaml "$SSH_TARGET:$REMOTE_DIR/docker-compose.yaml"
scp "${SSH_OPTS[@]}" infra/lightsail/nginx/nginx.conf "$SSH_TARGET:$REMOTE_DIR/config/nginx/"
scp "${SSH_OPTS[@]}" infra/lightsail/nginx/conf.d/default.conf "$SSH_TARGET:$REMOTE_DIR/config/nginx/conf.d/"

info "Frontend 静的ファイルを転送中..."
scp "${SSH_OPTS[@]}" -r /tmp/ringiflow-frontend-dist/* "$SSH_TARGET:$REMOTE_DIR/frontend/"

# init スクリプトも転送
scp "${SSH_OPTS[@]}" infra/lightsail/init/01_extensions.sql "$SSH_TARGET:$REMOTE_DIR/config/init/"

# ==========================================================
# 4. Lightsail 上でデプロイ
# ==========================================================
step "Step 4: Lightsail 上でデプロイを実行"

# shellcheck disable=SC2087,SC2029
# SC2029: $SSH_TARGET はクライアント側で展開されるのが意図的な動作（接続先ホストの指定）
ssh "${SSH_OPTS[@]}" "$SSH_TARGET" << 'REMOTE_SCRIPT'
set -euo pipefail
cd ~/ringiflow

echo "[INFO] Docker イメージをロード中..."
docker load < images/backend.tar.gz

echo "[INFO] Frontend ボリュームを更新中..."
# frontend_dist ボリュームにファイルをコピー
docker volume create ringiflow-frontend-dist 2>/dev/null || true
docker run --rm \
    -v ringiflow-frontend-dist:/dist \
    -v ~/ringiflow/frontend:/src:ro,z \
    busybox sh -c "rm -rf /dist/* && cp -r /src/* /dist/"

echo "[INFO] コンテナを起動中..."
docker compose -f docker-compose.yaml down --remove-orphans 2>/dev/null || true
docker compose -f docker-compose.yaml up -d

echo "[INFO] コンテナのステータス:"
docker compose -f docker-compose.yaml ps

echo "[INFO] ヘルスチェック..."
sleep 10
curl -sf http://localhost/health && echo " - Nginx: OK" || echo " - Nginx: NG"
curl -sf http://localhost/api/health && echo " - BFF: OK" || echo " - BFF: NG"
REMOTE_SCRIPT

# ==========================================================
# 5. クリーンアップ
# ==========================================================
step "Step 5: クリーンアップ"

rm -rf "$EXPORT_DIR"
rm -rf /tmp/ringiflow-frontend-dist

# ==========================================================
# 完了
# ==========================================================
echo ""
echo -e "${GREEN}=========================================="
echo -e "デプロイ完了！"
echo -e "==========================================${NC}"
echo ""
echo "確認方法:"
echo "  curl https://your-domain.com/health"
echo "  curl https://your-domain.com/api/health"
echo ""
echo "ログ確認:"
echo "  ssh $SSH_TARGET 'cd $REMOTE_DIR && docker compose logs -f'"
echo ""
