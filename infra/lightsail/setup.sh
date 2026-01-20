#!/bin/bash
# RingiFlow Lightsail 初期セットアップスクリプト
#
# このスクリプトは Lightsail インスタンス上で実行する。
# Docker と Docker Compose をインストールし、ディレクトリ構造を作成する。
#
# 使い方:
#   ssh ubuntu@your-lightsail-instance
#   curl -fsSL https://raw.githubusercontent.com/ka2kama/ringiflow/main/infra/lightsail/setup.sh | bash
#   または
#   scp setup.sh ubuntu@your-lightsail-instance:~
#   ssh ubuntu@your-lightsail-instance 'bash setup.sh'

set -euo pipefail

echo "=========================================="
echo "RingiFlow Lightsail セットアップ開始"
echo "=========================================="

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

# ==========================================================
# 1. システムアップデート
# ==========================================================
info "システムをアップデート中..."
sudo apt-get update
sudo apt-get upgrade -y

# ==========================================================
# 2. Docker インストール
# ==========================================================
if command -v docker &> /dev/null; then
    info "Docker は既にインストールされています: $(docker --version)"
else
    info "Docker をインストール中..."

    # 古いバージョンを削除
    sudo apt-get remove -y docker docker-engine docker.io containerd runc 2>/dev/null || true

    # 必要なパッケージをインストール
    sudo apt-get install -y \
        apt-transport-https \
        ca-certificates \
        curl \
        gnupg \
        lsb-release

    # Docker の公式 GPG キーを追加
    sudo install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    sudo chmod a+r /etc/apt/keyrings/docker.gpg

    # Docker リポジトリを追加
    # shellcheck disable=SC1091
    echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
        $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
        sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

    # Docker をインストール
    sudo apt-get update
    sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

    # 現在のユーザーを docker グループに追加
    sudo usermod -aG docker "$USER"

    info "Docker インストール完了: $(docker --version)"
fi

# ==========================================================
# 3. ディレクトリ構造を作成
# ==========================================================
info "ディレクトリ構造を作成中..."

RINGIFLOW_DIR="$HOME/ringiflow"
mkdir -p "$RINGIFLOW_DIR"/{config,backup,logs}

# 設定ファイル用ディレクトリ
mkdir -p "$RINGIFLOW_DIR/config/nginx/conf.d"

info "ディレクトリ構造:"
tree "$RINGIFLOW_DIR" 2>/dev/null || ls -laR "$RINGIFLOW_DIR"

# ==========================================================
# 4. .env.example をコピー
# ==========================================================
if [ ! -f "$RINGIFLOW_DIR/.env" ]; then
    info ".env.example をダウンロード中..."
    curl -fsSL https://raw.githubusercontent.com/ka2kama/ringiflow/main/infra/lightsail/.env.example \
        -o "$RINGIFLOW_DIR/.env.example"

    warn ".env.example を .env にコピーして設定してください:"
    warn "  cd $RINGIFLOW_DIR"
    warn "  cp .env.example .env"
    warn "  vim .env"
fi

# ==========================================================
# 5. バックアップ用 cron 設定（オプション）
# ==========================================================
info "バックアップ用 cron のセットアップはスキップ（手動で設定してください）"
echo "バックアップを有効にするには、以下のコマンドを実行:"
echo "  crontab -e"
echo "  # 毎日 AM 3:00 にバックアップ"
echo "  0 3 * * * $RINGIFLOW_DIR/backup.sh >> $RINGIFLOW_DIR/logs/backup.log 2>&1"

# ==========================================================
# 6. ファイアウォール設定
# ==========================================================
if command -v ufw &> /dev/null; then
    info "UFW ファイアウォールを設定中..."
    sudo ufw allow 22/tcp   # SSH
    sudo ufw allow 80/tcp   # HTTP（Cloudflare からのアクセス）
    sudo ufw --force enable
    sudo ufw status
else
    warn "UFW が見つかりません。Lightsail コンソールでファイアウォールを設定してください。"
fi

# ==========================================================
# 完了
# ==========================================================
echo ""
echo "=========================================="
echo -e "${GREEN}セットアップ完了！${NC}"
echo "=========================================="
echo ""
echo "次のステップ:"
echo "  1. 新しいシェルセッションを開く（または再ログイン）して docker グループを有効化"
echo "  2. .env ファイルを設定"
echo "     cd $RINGIFLOW_DIR"
echo "     cp .env.example .env"
echo "     vim .env"
echo "  3. デプロイスクリプトを実行（ローカルマシンから）"
echo "     ./deploy.sh"
echo ""
