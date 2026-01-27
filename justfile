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
# 順序: ツール確認 → 環境変数 → Git フック → Docker 起動 → DB マイグレーション → 依存関係ビルド
# ※ sqlx の query! マクロはコンパイル時に DB スキーマを検証するため、
#    マイグレーション完了後に cargo build を実行する必要がある
setup: check-tools setup-env setup-hooks dev-deps setup-db setup-deps
    @echo ""
    @echo "✓ セットアップ完了"
    @echo "  - just dev-bff         : BFF 起動"
    @echo "  - just dev-core-service: Core Service 起動"
    @echo "  - just dev-auth-service: Auth Service 起動"
    @echo "  - just dev-web         : フロントエンド起動"

# 開発ツールのインストール確認
check-tools:
    @echo "開発ツールを確認中..."
    @which rustc > /dev/null || (echo "ERROR: Rust がインストールされていません" && exit 1)
    @which cargo > /dev/null || (echo "ERROR: Cargo がインストールされていません" && exit 1)
    @which node > /dev/null || (echo "ERROR: Node.js がインストールされていません" && exit 1)
    @which pnpm > /dev/null || (echo "ERROR: pnpm がインストールされていません" && exit 1)
    @which elm > /dev/null || (echo "ERROR: Elm がインストールされていません" && exit 1)
    @which elm-format > /dev/null || (echo "ERROR: elm-format がインストールされていません" && exit 1)
    @which docker > /dev/null || (echo "ERROR: Docker がインストールされていません" && exit 1)
    @which sqlx > /dev/null || (echo "ERROR: sqlx-cli がインストールされていません" && exit 1)
    @which lefthook > /dev/null || (echo "ERROR: lefthook がインストールされていません" && exit 1)
    @which shellcheck > /dev/null || (echo "ERROR: shellcheck がインストールされていません" && exit 1)
    @which hurl > /dev/null || (echo "ERROR: hurl がインストールされていません" && exit 1)
    @which actionlint > /dev/null || (echo "ERROR: actionlint がインストールされていません" && exit 1)
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

# Git フックをセットアップ
setup-hooks:
    @echo "Git フックをセットアップ中..."
    @lefthook install
    @echo "✓ Git フックセットアップ完了"

# データベースをセットアップ（マイグレーション適用）
setup-db:
    @echo "データベースをセットアップ中..."
    @cd backend && sqlx migrate run 2>/dev/null || echo "  マイグレーションファイルなし（Phase 1 で作成予定）"
    @echo "✓ データベースセットアップ完了"

# データベースをリセット（drop → create → migrate）
reset-db:
    @echo "データベースをリセット中..."
    cd backend && sqlx database reset -y
    @echo "✓ データベースリセット完了"

# =============================================================================
# GitHub 設定
# =============================================================================

# GitHub ラベルを一括作成（冪等: 既存ラベルはスキップ）
# → 詳細: docs/04_手順書/02_プロジェクト構築/03_GitHub設定.md#7-labels
setup-labels:
    @echo "GitHub ラベルをセットアップ中..."
    @# Issue タイプ
    @gh label create "type:epic" --description "複数の Story をまとめる大きな機能" --color "7B68EE" 2>/dev/null || echo "  スキップ: type:epic（既存）"
    @gh label create "type:story" --description "ユーザー価値の単位（1〜数日で完了）" --color "1E90FF" 2>/dev/null || echo "  スキップ: type:story（既存）"
    @gh label create "idea" --description "後で検討するアイデア・メモ" --color "FBCA04" 2>/dev/null || echo "  スキップ: idea（既存）"
    @# カテゴリ
    @gh label create "backend" --description "Rust / API 関連" --color "0366d6" 2>/dev/null || echo "  スキップ: backend（既存）"
    @gh label create "frontend" --description "Elm / UI 関連" --color "28a745" 2>/dev/null || echo "  スキップ: frontend（既存）"
    @gh label create "infra" --description "Docker / Terraform / AWS" --color "6f42c1" 2>/dev/null || echo "  スキップ: infra（既存）"
    @gh label create "docs" --description "ドキュメント" --color "0075ca" 2>/dev/null || echo "  スキップ: docs（既存）"
    @# 優先度
    @gh label create "priority:high" --description "優先度: 高" --color "d73a4a" 2>/dev/null || echo "  スキップ: priority:high（既存）"
    @gh label create "priority:medium" --description "優先度: 中" --color "fbca04" 2>/dev/null || echo "  スキップ: priority:medium（既存）"
    @gh label create "priority:low" --description "優先度: 低" --color "0e8a16" 2>/dev/null || echo "  スキップ: priority:low（既存）"
    @echo "✓ GitHub ラベルセットアップ完了"

# =============================================================================
# 開発サーバー
# =============================================================================

# Docker で依存サービス（PostgreSQL, Redis）を起動
# --wait: healthcheck が通るまで待機（setup 時の競合を防止）
# プロジェクト名はディレクトリ名から自動取得（worktree対応）
dev-deps:
    #!/usr/bin/env bash
    PROJECT_NAME=$(basename "$(pwd)")
    docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml up -d --wait
    echo "PostgreSQL: localhost:${POSTGRES_PORT}"
    echo "Redis: localhost:${REDIS_PORT}"
    echo "プロジェクト名: $PROJECT_NAME"

# BFF 開発サーバーを起動（ポート: $BFF_PORT）
dev-bff:
    cd backend && cargo run -p ringiflow-bff

# Core Service 開発サーバーを起動（ポート: $CORE_PORT）
dev-core-service:
    cd backend && cargo run -p ringiflow-core-service

# Auth Service 開発サーバーを起動（ポート: $AUTH_PORT）
dev-auth-service:
    cd backend && cargo run -p ringiflow-auth-service

# フロントエンド開発サーバーを起動
dev-web:
    cd frontend && pnpm run dev

# =============================================================================
# フォーマット
# =============================================================================

# 全体フォーマット
fmt: fmt-rust fmt-elm

# Rust フォーマット（引数なし=全ファイル、引数あり=指定ファイル）
fmt-rust *files:
    #!/usr/bin/env bash
    if [ -z "{{files}}" ]; then
        cd backend && cargo +nightly fmt --all
    else
        rustfmt +nightly --edition 2024 --quiet {{files}}
    fi

# Elm フォーマット（引数なし=全ファイル、引数あり=指定ファイル）
fmt-elm *files:
    #!/usr/bin/env bash
    if [ -z "{{files}}" ]; then
        cd frontend && pnpm run fmt
    else
        elm-format --yes {{files}}
    fi

# =============================================================================
# リント（フォーマットチェック含む）
# =============================================================================

# 全体リント
lint: lint-rust lint-elm lint-shell lint-ci lint-openapi

# Rust リント（rustfmt + clippy）
lint-rust:
    cd backend && cargo +nightly fmt --all -- --check
    cd backend && cargo clippy --all-targets --all-features -- -D warnings

# Elm リント（elm-format + elm-review）
lint-elm:
    cd frontend && pnpm run lint

# シェルスクリプト リント（ShellCheck）
lint-shell:
    #!/usr/bin/env bash
    # git ls-files で .git と .gitignore を除外
    files=$(git ls-files --cached --others --exclude-standard "*.sh")
    if [ -z "$files" ]; then
        echo "No shell scripts found"
    else
        echo "$files" | xargs shellcheck
    fi

# GitHub Actions ワークフロー リント（actionlint）
lint-ci:
    actionlint

# OpenAPI 仕様書 リント（Redocly CLI）
lint-openapi:
    npx --yes @redocly/cli@latest lint --config openapi/redocly.yaml

# =============================================================================
# テスト
# =============================================================================

# 全テスト（単体テストのみ）
test: test-rust test-elm

# Rust 単体テスト
test-rust:
    cd backend && cargo test --all-features --lib --bins

# Rust 統合テスト（DB 接続が必要）
test-rust-integration:
    cd backend && cargo test --all-features --test '*'

# Elm テスト
test-elm:
    cd frontend && pnpm run test

# =============================================================================
# API テスト
# =============================================================================

# API テスト用の DB/Redis を起動（開発環境とは独立）
api-test-deps:
    docker compose -p ringiflow-api-test -f infra/docker/docker-compose.api-test.yaml up -d --wait
    @echo "API テスト環境:"
    @echo "  PostgreSQL: localhost:15433"
    @echo "  Redis: localhost:16380"

# API テスト用の DB をリセット
api-test-reset-db:
    @echo "API テスト用データベースをリセット中..."
    cd backend && DATABASE_URL=postgres://ringiflow:ringiflow@localhost:15433/ringiflow_api_test sqlx database reset -y
    @echo "✓ API テスト用データベースリセット完了"

# API テスト用の DB/Redis を停止
api-test-stop:
    docker compose -p ringiflow-api-test -f infra/docker/docker-compose.api-test.yaml down

# API テスト用の DB/Redis を削除（データ含む）
api-test-clean:
    docker compose -p ringiflow-api-test -f infra/docker/docker-compose.api-test.yaml down -v

# API テスト実行（hurl）
# サービスを起動してテストを実行し、終了後にサービスを停止する
test-api: api-test-deps api-test-reset-db
    ./scripts/run-api-tests.sh

# =============================================================================
# 全チェック
# =============================================================================

# プッシュ前の全チェック（リント、テスト、SQLx キャッシュ同期）
check-all: lint test sqlx-check

# SQLx オフラインキャッシュの同期チェック（DB 接続が必要）
# --all-targets: 統合テスト内の sqlx::query! マクロも含めてチェック
sqlx-check:
    cd backend && cargo sqlx prepare --check --workspace -- --all-targets

# SQLx クエリキャッシュを更新（DB 接続が必要）
# 新しい sqlx::query! を追加したら必ず実行する
# --all-targets: 統合テスト内の sqlx::query! マクロも含めてキャッシュ
sqlx-prepare:
    cd backend && cargo sqlx prepare --workspace -- --all-targets
    @echo "✓ SQLx クエリキャッシュを更新しました"
    @echo "  変更された .sqlx/ ファイルをコミットに含めてください"

# コミット前の完全チェック（sqlx-prepare + check-all）
# 新しいリポジトリ実装時やクエリ追加時は必ず実行する
pre-commit: sqlx-prepare check-all
    @echo ""
    @echo "✓ コミット前チェック完了"
    @echo "  git add backend/.sqlx/"
    @echo "  git commit"

# =============================================================================
# クリーンアップ
# =============================================================================

# ビルド成果物とコンテナを削除
# プロジェクト名はディレクトリ名から自動取得（worktree対応）
clean:
    #!/usr/bin/env bash
    PROJECT_NAME=$(basename "$(pwd)")
    docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml down -v
    cd backend && cargo clean
    cd frontend && rm -rf node_modules elm-stuff dist

# マージ済みローカルブランチを削除
clean-branches:
    git switch main
    git pull
    git fetch --prune
    git branch --merged main | grep -v main | xargs -r git branch -d

# =============================================================================
# Worktree 管理（並行開発用）
# =============================================================================

# worktree を追加（並行開発用の独立した作業ディレクトリを作成）
# 使い方: just worktree-add NAME BRANCH
# 例: just worktree-add auth feature/auth
# ポートオフセットは自動で空き番号が割り当てられる
worktree-add name branch:
    ./scripts/worktree-add.sh {{name}} {{branch}}

# worktree を削除
# 使い方: just worktree-remove NAME
worktree-remove name:
    #!/usr/bin/env bash
    set -euo pipefail
    PARENT_DIR=$(dirname "$(pwd)")
    WORKTREE_PATH="${PARENT_DIR}/ringiflow-{{name}}"
    PROJECT_NAME="ringiflow-{{name}}"

    echo "worktree を削除中: {{name}}"

    # Docker コンテナを停止・削除
    containers=$(docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml ps -q 2>/dev/null || true)
    if [[ -n "$containers" ]]; then
        echo "  Docker コンテナを停止中..."
        docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml down -v
    fi

    # worktree を削除
    git worktree remove "$WORKTREE_PATH" --force

    echo "✓ worktree を削除しました: {{name}}"

# worktree 一覧を表示
worktree-list:
    @echo "=== Worktree 一覧 ==="
    @git worktree list
    @echo ""
    @echo "=== Docker プロジェクト一覧 ==="
    @docker compose ls --filter "name=ringiflow" 2>/dev/null || echo "（実行中のプロジェクトなし）"
