# RingiFlow 開発タスク

# .env ファイルを自動読み込み
set dotenv-load := true

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
    @echo "  - just dev-bff      : BFF 起動"
    @echo "  - just dev-core-api : Core API 起動"
    @echo "  - just dev-web      : フロントエンド起動"

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
    @test -f backend/.env || (cp backend/.env.template backend/.env && echo "  作成: backend/.env")
    @test -f backend/.env && echo "  確認: backend/.env"
    @echo "✓ 環境変数ファイル準備完了"

# 依存関係をインストール
setup-deps:
    @echo "依存関係をインストール中..."
    @echo "  Rust..."
    @cd backend && cargo build
    @echo "  Elm/Vite..."
    @cd frontend && pnpm install
    @echo "✓ 依存関係インストール完了"

# データベースをセットアップ
setup-db:
    @echo "データベースをセットアップ中..."
    @sleep 3
    @cd backend && sqlx migrate run 2>/dev/null || echo "  マイグレーションファイルなし（Phase 1 で作成予定）"
    @echo "✓ データベースセットアップ完了"

# =============================================================================
# 開発サーバー
# =============================================================================

# Docker で依存サービス（PostgreSQL, Redis）を起動
dev-deps:
    docker compose -f infra/docker/docker-compose.yml up -d
    @echo "PostgreSQL: localhost:${POSTGRES_PORT}"
    @echo "Redis: localhost:${REDIS_PORT}"

# BFF 開発サーバーを起動（ポート: $BFF_PORT）
dev-bff:
    cd backend && cargo run -p ringiflow-bff

# Core API 開発サーバーを起動（ポート: $CORE_API_PORT）
dev-core-api:
    cd backend && cargo run -p ringiflow-core-api

# フロントエンド開発サーバーを起動
dev-web:
    cd frontend && pnpm run dev

# =============================================================================
# フォーマット
# =============================================================================

# 全体フォーマット
fmt: fmt-rust fmt-elm

# Rust フォーマット
fmt-rust:
    cd backend && cargo +nightly fmt --all

# Elm フォーマット
fmt-elm:
    cd frontend && pnpm run fmt

# =============================================================================
# リント（フォーマットチェック含む）
# =============================================================================

# 全体リント
lint: lint-rust lint-elm

# Rust リント（rustfmt + clippy）
lint-rust:
    cd backend && cargo +nightly fmt --all -- --check
    cd backend && cargo clippy --all-targets --all-features -- -D warnings

# Elm リント（elm-format + elm-review）
lint-elm:
    cd frontend && pnpm run lint

# =============================================================================
# テスト
# =============================================================================

# 全テスト
test: test-rust test-elm

# Rust テスト
test-rust:
    cd backend && cargo test --all-features

# Elm テスト
test-elm:
    cd frontend && pnpm run test

# =============================================================================
# 全チェック
# =============================================================================

# プッシュ前の全チェック（リント、テスト）
check-all: lint test

# =============================================================================
# クリーンアップ
# =============================================================================

# ビルド成果物とコンテナを削除
clean:
    docker compose -f infra/docker/docker-compose.yml down -v
    cd backend && cargo clean
    cd frontend && rm -rf node_modules elm-stuff dist
