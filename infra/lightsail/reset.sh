#!/bin/bash
# RingiFlow デモ環境リセットスクリプト
#
# 全データストアを初期シード状態にリセットする。
# このスクリプトは Lightsail インスタンス上で実行する。
#
# 対象:
#   - PostgreSQL: drop → create → 全マイグレーション再適用（シードデータ含む）
#   - Redis: 全データ削除（セッション、CSRF トークン等）
#   - DynamoDB: コンテナ再起動（-inMemory のため自動リセット）
#
# 使い方:
#   ./reset.sh              # 確認プロンプトあり
#   ./reset.sh --yes        # 確認なしで実行（CI 用）
#
# 前提:
#   - ~/ringiflow/.env が設定済み
#   - ~/ringiflow/migrations/ にマイグレーションファイルが配置済み
#   - Docker コンテナが起動中

set -euo pipefail

RINGIFLOW_DIR="${RINGIFLOW_DIR:-$HOME/ringiflow}"

# 色付き出力
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    RED='\033[0;31m'
    CYAN='\033[0;36m'
    NC='\033[0m'
else
    GREEN=''
    YELLOW=''
    RED=''
    CYAN=''
    NC=''
fi

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
    echo -e "${CYAN}--- $1 ---${NC}"
}

# .env を読み込み
if [ -f "$RINGIFLOW_DIR/.env" ]; then
    # shellcheck source=/dev/null
    source "$RINGIFLOW_DIR/.env"
fi

: "${POSTGRES_USER:=ringiflow}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD が設定されていません}"
: "${POSTGRES_DB:=ringiflow}"
: "${REDIS_PASSWORD:?REDIS_PASSWORD が設定されていません}"

# ==========================================================
# 確認プロンプト
# ==========================================================
AUTO_YES=false
for arg in "$@"; do
    case $arg in
        --yes|-y)
            AUTO_YES=true
            ;;
    esac
done

if [ "$AUTO_YES" = false ]; then
    echo ""
    warn "全データストアを初期シード状態にリセットします。"
    warn "現在のデータはすべて削除されます。"
    echo ""
    echo "対象:"
    echo "  - PostgreSQL: 全テーブルを削除し、マイグレーション（シードデータ含む）を再適用"
    echo "  - Redis: 全データを削除（セッション等）"
    echo "  - DynamoDB: コンテナ再起動（監査ログのリセット）"
    echo ""
    read -rp "続行しますか？ (yes/no): " CONFIRM
    if [ "$CONFIRM" != "yes" ]; then
        error "リセットをキャンセルしました"
    fi
fi

# ==========================================================
# コンテナ状態の確認
# ==========================================================
step "1. コンテナ状態を確認"

cd "$RINGIFLOW_DIR"

for container in ringiflow-postgres ringiflow-redis ringiflow-dynamodb; do
    if ! docker inspect --format='{{.State.Running}}' "$container" 2>/dev/null | grep -q true; then
        error "コンテナ $container が起動していません。docker compose up -d を実行してください。"
    fi
done
info "全コンテナが起動中"

# ==========================================================
# マイグレーションファイルの確認
# ==========================================================
step "2. マイグレーションファイルを確認"

MIGRATIONS_DIR="$RINGIFLOW_DIR/migrations"
if [ ! -d "$MIGRATIONS_DIR" ] || [ -z "$(ls -A "$MIGRATIONS_DIR"/*.sql 2>/dev/null)" ]; then
    error "マイグレーションファイルが見つかりません: $MIGRATIONS_DIR"
fi

MIGRATION_COUNT=$(find "$MIGRATIONS_DIR" -name '*.sql' | wc -l)
info "マイグレーションファイル: ${MIGRATION_COUNT} 個"

# ==========================================================
# アプリケーションサービスを停止（DB 接続を切断）
# ==========================================================
step "3. アプリケーションサービスを停止"

info "BFF, Core Service, Auth Service を停止中..."
docker compose stop bff core-service auth-service 2>/dev/null || \
    docker stop ringiflow-bff ringiflow-core-service ringiflow-auth-service 2>/dev/null || true
info "アプリケーションサービス停止完了"

# ==========================================================
# PostgreSQL リセット
# ==========================================================
step "4. PostgreSQL をリセット"

# 残存接続を強制切断
info "残存接続を切断中..."
docker exec ringiflow-postgres psql -U "$POSTGRES_USER" -d postgres \
    -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '${POSTGRES_DB}' AND pid <> pg_backend_pid();" \
    > /dev/null 2>&1 || true

info "データベースを削除中..."
docker exec ringiflow-postgres psql -U "$POSTGRES_USER" -d postgres \
    -c "DROP DATABASE IF EXISTS ${POSTGRES_DB};"

info "データベースを再作成中..."
docker exec ringiflow-postgres psql -U "$POSTGRES_USER" -d postgres \
    -c "CREATE DATABASE ${POSTGRES_DB};"

info "拡張機能をインストール中..."
docker exec ringiflow-postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
    -f /docker-entrypoint-initdb.d/01_extensions.sql

info "マイグレーションを実行中..."
# psql でマイグレーションファイルを順番に適用する
# （sqlx-cli は 1GB RAM の Lightsail 環境ではコンパイルできないため psql を使用）
#
# sqlx との互換性:
#   アプリ起動時に sqlx migrate run がチェックサム検証を行うため、
#   _sqlx_migrations テーブルに正しい SHA-384 チェックサムを記録する必要がある

# sqlx 互換の _sqlx_migrations テーブルを作成
docker exec ringiflow-postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "
CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
);
"

APPLIED=0
FAILED=0
for migration_file in $(find "$MIGRATIONS_DIR" -name '*.sql' | sort); do
    filename=$(basename "$migration_file")
    # ファイル名から version と description を抽出（例: 20260115000001_create_tenants.sql）
    version=$(echo "$filename" | grep -oP '^\d+')
    description=$(echo "$filename" | sed -E 's/^[0-9]+_//' | sed 's/\.sql$//')

    # SHA-384 チェックサムを計算（sqlx と同じアルゴリズム）
    checksum_hex=$(sha384sum "$migration_file" | cut -d' ' -f1)

    if docker exec -i ringiflow-postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" < "$migration_file" > /dev/null 2>&1; then
        # _sqlx_migrations に正しいチェックサムで記録
        docker exec ringiflow-postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "
            INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
            VALUES ($version, '$description', true, '\\x${checksum_hex}', 0)
            ON CONFLICT (version) DO NOTHING;
        " > /dev/null 2>&1
        APPLIED=$((APPLIED + 1))
    else
        warn "マイグレーション失敗: $filename"
        FAILED=$((FAILED + 1))
    fi
done

if [ "$FAILED" -gt 0 ]; then
    warn "マイグレーション完了: ${APPLIED} 成功, ${FAILED} 失敗"
else
    info "マイグレーション完了: ${APPLIED} 個適用"
fi

info "PostgreSQL リセット完了"

# ==========================================================
# Redis フラッシュ
# ==========================================================
step "5. Redis をフラッシュ"

docker exec ringiflow-redis redis-cli -a "$REDIS_PASSWORD" FLUSHALL 2>/dev/null
info "Redis フラッシュ完了"

# ==========================================================
# DynamoDB リセット（コンテナ再起動）
# ==========================================================
step "6. DynamoDB をリセット"

docker restart ringiflow-dynamodb
info "DynamoDB リセット完了（-inMemory のためデータはクリア済み）"

# ==========================================================
# 全アプリケーションサービスを再起動
# ==========================================================
step "7. アプリケーションサービスを再起動"

# DynamoDB の healthcheck が通るまで待つ
info "DynamoDB の起動を待機中..."
for i in $(seq 1 30); do
    if docker inspect --format='{{.State.Health.Status}}' ringiflow-dynamodb 2>/dev/null | grep -q healthy; then
        break
    fi
    if [ "$i" -eq 30 ]; then
        warn "DynamoDB の healthcheck がタイムアウトしました（続行します）"
    fi
    sleep 1
done

info "Core Service, Auth Service, BFF を起動中..."
docker compose start core-service auth-service 2>/dev/null || \
    docker start ringiflow-core-service ringiflow-auth-service 2>/dev/null || true

# Core/Auth の healthcheck を待ってから BFF を起動
info "Core Service, Auth Service の起動を待機中..."
for i in $(seq 1 30); do
    CORE_OK=$(docker inspect --format='{{.State.Health.Status}}' ringiflow-core-service 2>/dev/null || echo "unknown")
    AUTH_OK=$(docker inspect --format='{{.State.Health.Status}}' ringiflow-auth-service 2>/dev/null || echo "unknown")
    if [ "$CORE_OK" = "healthy" ] && [ "$AUTH_OK" = "healthy" ]; then
        break
    fi
    if [ "$i" -eq 30 ]; then
        warn "サービスの healthcheck がタイムアウトしました（続行します）"
    fi
    sleep 1
done

docker compose start bff 2>/dev/null || docker start ringiflow-bff 2>/dev/null || true
info "アプリケーションサービス再起動完了"

# ==========================================================
# ヘルスチェック
# ==========================================================
step "8. ヘルスチェック"

info "サービスの起動を待機中..."
sleep 10

if curl -sf http://localhost/health > /dev/null 2>&1; then
    info "Nginx: OK"
else
    warn "Nginx: NG（起動中の可能性あり）"
fi

if curl -sf http://localhost/api/health > /dev/null 2>&1; then
    info "BFF: OK"
else
    warn "BFF: NG（起動中の可能性あり）"
fi

# ==========================================================
# 完了
# ==========================================================
echo ""
echo -e "${GREEN}=========================================="
echo -e "リセット完了！"
echo -e "==========================================${NC}"
echo ""
echo "初期シードデータ:"
echo "  管理者: admin@example.com / password123"
echo "  一般: user@example.com / password123"
echo "  他 8 ユーザー（tanaka, sato, suzuki, takahashi, ito, watanabe, yamamoto, nakamura）"
echo ""
