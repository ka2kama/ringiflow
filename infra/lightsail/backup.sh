#!/bin/bash
# RingiFlow バックアップスクリプト
#
# このスクリプトは Lightsail インスタンス上で実行する。
# PostgreSQL のダンプと Redis の RDB スナップショットを取得。
#
# 使い方:
#   ./backup.sh              # バックアップ実行
#   ./backup.sh --restore    # 最新バックアップからリストア
#
# cron 設定例（毎日 AM 3:00）:
#   0 3 * * * /home/ubuntu/ringiflow/backup.sh >> /home/ubuntu/ringiflow/logs/backup.log 2>&1

set -euo pipefail

# 設定
RINGIFLOW_DIR="${RINGIFLOW_DIR:-$HOME/ringiflow}"
BACKUP_DIR="$RINGIFLOW_DIR/backup"
BACKUP_RETENTION_DAYS=7

# 日時
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# 色付き出力（cron 実行時は無効）
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    RED='\033[0;31m'
    NC='\033[0m'
else
    GREEN=''
    YELLOW=''
    RED=''
    NC=''
fi

info() {
    echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] ${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] ${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] ${RED}[ERROR]${NC} $1"
    exit 1
}

# .env を読み込み
if [ -f "$RINGIFLOW_DIR/.env" ]; then
    # shellcheck source=/dev/null
    source "$RINGIFLOW_DIR/.env"
fi

: "${POSTGRES_USER:=ringiflow}"
: "${POSTGRES_DB:=ringiflow_prod}"

# ==========================================================
# バックアップ処理
# ==========================================================
backup() {
    info "バックアップ開始: $TIMESTAMP"

    mkdir -p "$BACKUP_DIR"

    # PostgreSQL バックアップ
    info "PostgreSQL をバックアップ中..."
    POSTGRES_BACKUP="$BACKUP_DIR/postgres_${TIMESTAMP}.sql.gz"
    docker exec ringiflow-postgres pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB" | gzip > "$POSTGRES_BACKUP"
    info "PostgreSQL バックアップ完了: $POSTGRES_BACKUP ($(du -h "$POSTGRES_BACKUP" | cut -f1))"

    # Redis バックアップ（BGSAVE を実行して RDB を取得）
    info "Redis をバックアップ中..."
    docker exec ringiflow-redis redis-cli -a "${REDIS_PASSWORD:-}" BGSAVE 2>/dev/null || true
    sleep 2  # BGSAVE の完了を待つ

    REDIS_BACKUP="$BACKUP_DIR/redis_${TIMESTAMP}.rdb"
    docker cp ringiflow-redis:/data/dump.rdb "$REDIS_BACKUP"
    info "Redis バックアップ完了: $REDIS_BACKUP ($(du -h "$REDIS_BACKUP" | cut -f1))"

    # 古いバックアップを削除
    info "古いバックアップを削除中（${BACKUP_RETENTION_DAYS}日以上前）..."
    find "$BACKUP_DIR" -name "postgres_*.sql.gz" -mtime +$BACKUP_RETENTION_DAYS -delete
    find "$BACKUP_DIR" -name "redis_*.rdb" -mtime +$BACKUP_RETENTION_DAYS -delete

    # バックアップ一覧
    info "現在のバックアップ:"
    ls -lh "$BACKUP_DIR"

    info "バックアップ完了"
}

# ==========================================================
# リストア処理
# ==========================================================
restore() {
    info "リストア開始"

    # 最新のバックアップを検索（ファイル名のタイムスタンプでソート）
    LATEST_POSTGRES=$(find "$BACKUP_DIR" -name "postgres_*.sql.gz" 2>/dev/null | sort -r | head -1)
    LATEST_REDIS=$(find "$BACKUP_DIR" -name "redis_*.rdb" 2>/dev/null | sort -r | head -1)

    if [ -z "$LATEST_POSTGRES" ]; then
        error "PostgreSQL バックアップが見つかりません"
    fi

    if [ -z "$LATEST_REDIS" ]; then
        warn "Redis バックアップが見つかりません（スキップ）"
    fi

    echo ""
    warn "以下のバックアップからリストアします:"
    echo "  PostgreSQL: $LATEST_POSTGRES"
    echo "  Redis: ${LATEST_REDIS:-なし}"
    echo ""
    read -rp "続行しますか？ (yes/no): " CONFIRM
    if [ "$CONFIRM" != "yes" ]; then
        error "リストアをキャンセルしました"
    fi

    # PostgreSQL リストア
    info "PostgreSQL をリストア中..."
    # 既存のデータベースを削除して再作成
    docker exec -i ringiflow-postgres psql -U "$POSTGRES_USER" -c "DROP DATABASE IF EXISTS ${POSTGRES_DB};"
    docker exec -i ringiflow-postgres psql -U "$POSTGRES_USER" -c "CREATE DATABASE ${POSTGRES_DB};"
    gunzip -c "$LATEST_POSTGRES" | docker exec -i ringiflow-postgres psql -U "$POSTGRES_USER" "$POSTGRES_DB"
    info "PostgreSQL リストア完了"

    # Redis リストア
    if [ -n "$LATEST_REDIS" ]; then
        info "Redis をリストア中..."
        docker exec ringiflow-redis redis-cli -a "${REDIS_PASSWORD:-}" SHUTDOWN NOSAVE 2>/dev/null || true
        sleep 2
        docker cp "$LATEST_REDIS" ringiflow-redis:/data/dump.rdb
        docker start ringiflow-redis
        info "Redis リストア完了"
    fi

    info "リストア完了"
}

# ==========================================================
# メイン処理
# ==========================================================
case "${1:-}" in
    --restore)
        restore
        ;;
    *)
        backup
        ;;
esac
