#!/bin/bash
# RingiFlow Lightsail 初期セットアップスクリプト（AlmaLinux 9）
#
# このスクリプトは Lightsail インスタンス上で実行する。
# Docker と Docker Compose をインストールし、ディレクトリ構造を作成する。
#
# 使い方（SSH ログイン後に実行）:
#   curl -fsSL https://raw.githubusercontent.com/ka2kama/ringiflow/main/infra/lightsail/setup.sh | bash
#
# 使い方（Lightsail 起動スクリプトとして指定）:
#   起動スクリプトは root で実行されるため、自動的に ec2-user 用にセットアップする。

set -euo pipefail

# 実行ユーザーの検出
# 起動スクリプトとして実行される場合は root なので、ターゲットユーザーを ec2-user に設定
if [ "$(id -u)" -eq 0 ]; then
    TARGET_USER="ec2-user"
    TARGET_HOME="/home/$TARGET_USER"
else
    TARGET_USER="$USER"
    TARGET_HOME="$HOME"
fi

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

# root 実行時は sudo 不要、一般ユーザー実行時は sudo を付ける
if [ "$(id -u)" -eq 0 ]; then
    SUDO=""
else
    SUDO="sudo"
fi

# ==========================================================
# 1. システムアップデート
# ==========================================================
info "システムをアップデート中..."
$SUDO dnf update -y

# ==========================================================
# 2. Docker インストール
# ==========================================================
if command -v docker &> /dev/null; then
    info "Docker は既にインストールされています: $(docker --version)"
else
    info "Docker をインストール中..."

    # 古いバージョンを削除
    $SUDO dnf remove -y docker docker-client docker-client-latest \
        docker-common docker-latest docker-latest-logrotate \
        docker-logrotate docker-engine 2>/dev/null || true

    # dnf-plugins-core をインストール（config-manager コマンドに必要）
    $SUDO dnf install -y dnf-plugins-core

    # Docker 公式リポジトリを追加（AlmaLinux は CentOS リポジトリを使用）
    $SUDO dnf config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo

    # Docker をインストール
    $SUDO dnf install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

    # Docker サービスを起動・有効化
    $SUDO systemctl enable --now docker

    # ターゲットユーザーを docker グループに追加
    $SUDO usermod -aG docker "$TARGET_USER"

    info "Docker インストール完了: $(docker --version)"
fi

# ==========================================================
# 3. ディレクトリ構造を作成
# ==========================================================
info "ディレクトリ構造を作成中..."

RINGIFLOW_DIR="$TARGET_HOME/ringiflow"
mkdir -p "$RINGIFLOW_DIR"/{config,backup,logs}

# 設定ファイル用ディレクトリ
mkdir -p "$RINGIFLOW_DIR/config/nginx/conf.d"

# root 実行時はディレクトリの所有者をターゲットユーザーに変更
if [ "$(id -u)" -eq 0 ]; then
    chown -R "$TARGET_USER:$TARGET_USER" "$RINGIFLOW_DIR"
fi

info "ディレクトリ構造:"
tree "$RINGIFLOW_DIR" 2>/dev/null || ls -laR "$RINGIFLOW_DIR"

# ==========================================================
# 4. .env.example をコピー
# ==========================================================
if [ ! -f "$RINGIFLOW_DIR/.env" ]; then
    info ".env.example をダウンロード中..."
    curl -fsSL https://raw.githubusercontent.com/ka2kama/ringiflow/main/infra/lightsail/.env.example \
        -o "$RINGIFLOW_DIR/.env.example"

    # root 実行時はダウンロードしたファイルの所有者を変更
    if [ "$(id -u)" -eq 0 ]; then
        chown "$TARGET_USER:$TARGET_USER" "$RINGIFLOW_DIR/.env.example"
    fi

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
# 6. ファイアウォール設定（firewalld）
# ==========================================================
if command -v firewall-cmd &> /dev/null; then
    info "firewalld を設定中..."
    $SUDO systemctl enable --now firewalld
    $SUDO firewall-cmd --permanent --add-service=ssh
    $SUDO firewall-cmd --permanent --add-port=80/tcp
    $SUDO firewall-cmd --reload
    $SUDO firewall-cmd --list-all
else
    warn "firewalld が見つかりません。Lightsail コンソールでファイアウォールを設定してください。"
fi

# ==========================================================
# 7. SELinux 状態の確認
# ==========================================================
info "SELinux の状態を確認中..."
if command -v getenforce &> /dev/null; then
    SELINUX_STATUS=$(getenforce)
    info "SELinux: $SELINUX_STATUS"
    if [ "$SELINUX_STATUS" = "Enforcing" ]; then
        info "SELinux は有効です。Docker のバインドマウントには :z/:Z フラグが設定済みです。"
    fi
else
    warn "SELinux コマンドが見つかりません。"
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
echo "  1. $TARGET_USER で SSH ログイン（または再ログイン）して docker グループを有効化"
echo "  2. .env ファイルを設定"
echo "     cd $RINGIFLOW_DIR"
echo "     cp .env.example .env"
echo "     vim .env"
echo "  3. デプロイスクリプトを実行（ローカルマシンから）"
echo "     ./deploy.sh"
echo ""
