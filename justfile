# RingiFlow 開発タスク

# デフォルト: レシピ一覧を表示
default:
    @just --list

# =============================================================================
# 初回セットアップ
# =============================================================================

# 初回セットアップ（全体）
setup: check-tools setup-env setup-deps dev-deps setup-db
    @echo ""
    @echo "✓ セットアップ完了"
    @echo "  - just dev-api  : バックエンド起動"
    @echo "  - just dev-web  : フロントエンド起動"

# 開発ツールのインストール確認
check-tools:
    @echo "開発ツールを確認中..."
    @which rustc > /dev/null || (echo "ERROR: Rust がインストールされていません" && exit 1)
    @which cargo > /dev/null || (echo "ERROR: Cargo がインストールされていません" && exit 1)
    @which node > /dev/null || (echo "ERROR: Node.js がインストールされていません" && exit 1)
    @which pnpm > /dev/null || (echo "ERROR: pnpm がインストールされていません" && exit 1)
    @which elm > /dev/null || (echo "ERROR: Elm がインストールされていません" && exit 1)
    @which docker > /dev/null || (echo "ERROR: Docker がインストールされていません" && exit 1)
    @which sqlx > /dev/null || (echo "ERROR: sqlx-cli がインストールされていません" && exit 1)
    @echo "✓ 全ツール確認済み"

# .env ファイルを作成（既存の場合はスキップ）
setup-env:
    @echo "環境変数ファイルを確認中..."
    @test -f .env || (cp .env.template .env && echo "  作成: .env")
    @test -f .env && echo "  確認: .env"
    @test -f apps/api/.env || (cp apps/api/.env.template apps/api/.env && echo "  作成: apps/api/.env")
    @test -f apps/api/.env && echo "  確認: apps/api/.env"
    @test -f apps/web/.env || (cp apps/web/.env.template apps/web/.env && echo "  作成: apps/web/.env")
    @test -f apps/web/.env && echo "  確認: apps/web/.env"
    @echo "✓ 環境変数ファイル準備完了"

# 依存関係をインストール
setup-deps:
    @echo "依存関係をインストール中..."
    @echo "  Rust..."
    @cargo build
    @echo "  Elm/Vite..."
    @cd apps/web && pnpm install
    @echo "✓ 依存関係インストール完了"

# データベースをセットアップ
setup-db:
    @echo "データベースをセットアップ中..."
    @sleep 3
    @cd apps/api && sqlx migrate run
    @echo "✓ マイグレーション完了"

# =============================================================================
# 開発サーバー
# =============================================================================

# Docker で依存サービス（PostgreSQL, Redis）を起動
dev-deps:
    docker compose -f infra/docker/docker-compose.yml up -d
    @echo "PostgreSQL: localhost:${POSTGRES_PORT:-15432}"
    @echo "Redis: localhost:${REDIS_PORT:-16379}"

# バックエンド開発サーバーを起動
dev-api:
    cd apps/api && cargo run --bin bff

# フロントエンド開発サーバーを起動
dev-web:
    cd apps/web && pnpm run dev

# =============================================================================
# フォーマット
# =============================================================================

# 全体フォーマット
fmt: fmt-rust fmt-elm

# Rust フォーマット
fmt-rust:
    cargo +nightly fmt --all

# Elm フォーマット
fmt-elm:
    cd apps/web && pnpm run fmt

# =============================================================================
# フォーマットチェック
# =============================================================================

# 全体フォーマットチェック
fmt-check: fmt-check-rust fmt-check-elm

# Rust フォーマットチェック
fmt-check-rust:
    cargo +nightly fmt --all -- --check

# Elm フォーマットチェック
fmt-check-elm:
    cd apps/web && pnpm run format:check

# =============================================================================
# リント
# =============================================================================

# 全体リント
lint: lint-rust lint-elm

# Rust リント（clippy）
lint-rust:
    cargo clippy --all-targets --all-features -- -D warnings

# Elm リント（elm-format チェック）
lint-elm:
    cd apps/web && pnpm run format:check

# =============================================================================
# テスト
# =============================================================================

# 全テスト
test: test-rust test-elm

# Rust テスト
test-rust:
    cargo test --all-features

# Elm テスト
test-elm:
    cd apps/web && pnpm run test

# =============================================================================
# 全チェック
# =============================================================================

# プッシュ前の全チェック（フォーマット、リント、テスト）
check-all: fmt-check lint test

# =============================================================================
# クリーンアップ
# =============================================================================

# ビルド成果物とコンテナを削除
clean:
    docker compose -f infra/docker/docker-compose.yml down -v
    cd apps/api && cargo clean
    cd apps/web && rm -rf node_modules elm-stuff dist
