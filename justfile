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
    @echo "  - just dev-all         : 全サーバー一括起動（推奨）"
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
    @which cargo-watch > /dev/null || (echo "ERROR: cargo-watch がインストールされていません" && exit 1)
    @which cargo-deny > /dev/null || (echo "ERROR: cargo-deny がインストールされていません" && exit 1)
    @which cargo-llvm-cov > /dev/null || (echo "ERROR: cargo-llvm-cov がインストールされていません" && exit 1)
    @which cargo-machete > /dev/null || (echo "ERROR: cargo-machete がインストールされていません" && exit 1)
    @which sccache > /dev/null || (echo "ERROR: sccache がインストールされていません" && exit 1)
    @which mprocs > /dev/null || (echo "ERROR: mprocs がインストールされていません" && exit 1)
    @which gh > /dev/null || (echo "ERROR: GitHub CLI (gh) がインストールされていません" && exit 1)
    @which psql > /dev/null || (echo "ERROR: psql がインストールされていません" && exit 1)
    @which redis-cli > /dev/null || (echo "ERROR: redis-cli がインストールされていません" && exit 1)
    @echo "✓ 全ツール確認済み"

# .env ファイルを作成（既存の場合はスキップ）
# worktree の場合は空きポートオフセットを自動割り当て
setup-env:
    ./scripts/setup-env.sh

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

# worktree 用セットアップ（Docker 起動 → DB マイグレーション → 依存関係インストール）
setup-worktree: dev-deps setup-db setup-deps
    @echo ""
    @echo "✓ worktree セットアップ完了"

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
    docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml up -d --wait
    echo "PostgreSQL: localhost:${POSTGRES_PORT}"
    echo "Redis: localhost:${REDIS_PORT}"
    echo "プロジェクト名: $PROJECT_NAME"

# BFF 開発サーバーを起動（ポート: $BFF_PORT、ファイル変更で自動リビルド）
dev-bff:
    cd backend && cargo watch -x 'run -p ringiflow-bff'

# Core Service 開発サーバーを起動（ポート: $CORE_PORT、ファイル変更で自動リビルド）
dev-core-service:
    cd backend && cargo watch -x 'run -p ringiflow-core-service'

# Auth Service 開発サーバーを起動（ポート: $AUTH_PORT、ファイル変更で自動リビルド）
dev-auth-service:
    cd backend && cargo watch -x 'run -p ringiflow-auth-service'

# フロントエンド開発サーバーを起動
dev-web:
    cd frontend && pnpm run dev

# 全開発サーバーを一括起動（依存サービス + mprocs）
dev-all: dev-deps
    mprocs

# 依存サービス（PostgreSQL, Redis）を停止
dev-down:
    #!/usr/bin/env bash
    PROJECT_NAME=$(basename "$(pwd)")
    docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml down

# 開発サーバープロセスを一括終了（mprocs がハングしたとき用）
dev-kill:
    #!/usr/bin/env bash
    echo "開発サーバープロセスを終了中..."
    pkill -f "mprocs" 2>/dev/null && echo "  mprocs を終了しました" || true
    pkill -f "ringiflow-bff" 2>/dev/null && echo "  BFF を終了しました" || true
    pkill -f "ringiflow-core-service" 2>/dev/null && echo "  Core Service を終了しました" || true
    pkill -f "ringiflow-auth-service" 2>/dev/null && echo "  Auth Service を終了しました" || true
    pkill -f "pnpm.*dev.*ringiflow" 2>/dev/null || pkill -f "vite.*ringiflow" 2>/dev/null && echo "  Web を終了しました" || true
    echo "✓ 完了"

# =============================================================================
# データストア操作（開発用）
# =============================================================================

_psql_url := "postgres://ringiflow:ringiflow@localhost:" + env_var_or_default("POSTGRES_PORT", "15432") + "/ringiflow_dev"

# PostgreSQL: テーブル一覧を表示
db-tables:
    @psql "{{ _psql_url }}" -c "\dt public.*" --pset="footer=off"

# PostgreSQL: 指定テーブルのカラム定義を表示
db-schema table:
    @psql "{{ _psql_url }}" -c "\d {{table}}"

# PostgreSQL: 任意の SQL を実行
db-query sql:
    @psql "{{ _psql_url }}" -c "{{sql}}"

# Redis: キー一覧を表示（パターンで絞り込み可能、デフォルト: *）
redis-keys pattern='*':
    @redis-cli -p "${REDIS_PORT}" keys "{{pattern}}"

# Redis: 指定キーの値を取得
redis-get key:
    @redis-cli -p "${REDIS_PORT}" get "{{key}}"

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
lint: lint-rust lint-elm lint-shell lint-ci lint-openapi check-unused-deps

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

# Rust 単体テスト + doctest
test-rust:
    cd backend && cargo test --all-features --lib --bins
    cd backend && cargo test --all-features --doc

# Rust 統合テスト（DB 接続が必要）
test-rust-integration:
    cd backend && cargo test --all-features --test '*'

# Elm テスト
test-elm:
    cd frontend && pnpm run test

# Elm ビルドチェック（コンパイルエラー検出）
# lint-elm や test-elm ではコンパイルエラーを検出できないため、
# 実際にビルドしてコンパイルエラーがないことを確認する
build-elm:
    cd frontend && pnpm run build

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
# セキュリティチェック
# =============================================================================

# 依存関係の脆弱性・ライセンスチェック（cargo-deny）
audit:
    cd backend && cargo deny check

# =============================================================================
# 構造品質チェック
# =============================================================================

# ソースファイルの行数閾値チェック（500 行超で警告）
check-file-size:
    ./scripts/check-file-size.sh

# コード重複（コピー＆ペースト）を検出（jscpd）
# 警告のみ（exit 0）: CI をブロックしない。重複の可視化が目的。
# 選定理由: docs/05_ADR/042_コピペ検出ツールの選定.md
# 注: --formats-exts と複数 format の同時指定にバグがあるため、Rust と Elm を分けて実行する
check-duplicates:
    @echo "=== Rust コード重複チェック ==="
    npx --yes jscpd@latest --min-lines 10 --min-tokens 50 --format "rust" --gitignore --exitCode 0 backend/
    @echo ""
    @echo "=== Elm コード重複チェック ==="
    npx --yes jscpd@latest --min-lines 10 --min-tokens 50 --format "haskell" --formats-exts "haskell:elm" --gitignore --exitCode 0 frontend/src/

# =============================================================================
# 未使用依存チェック
# =============================================================================

# Cargo.toml の未使用依存を検出（cargo-machete）
# 選定理由: docs/05_ADR/038_未使用依存検出ツールの選定.md
check-unused-deps:
    cd backend && cargo machete

# =============================================================================
# カバレッジ計測
# =============================================================================

# Rust コードカバレッジを計測し HTML レポートを生成（cargo-llvm-cov）
# レポートは backend/target/llvm-cov/html/index.html に出力される
coverage:
    cd backend && cargo llvm-cov --workspace --html
    @echo ""
    @echo "✓ カバレッジレポート生成完了"
    @echo "  open backend/target/llvm-cov/html/index.html"

# Rust コードカバレッジのサマリーをターミナルに表示
coverage-summary:
    cd backend && cargo llvm-cov --workspace

# =============================================================================
# 全チェック
# =============================================================================

# 実装中の軽量チェック（リント、テスト、統合テスト、ビルド、SQLx キャッシュ同期、セキュリティ、構造品質）
check: lint test test-rust-integration build-elm sqlx-check audit check-file-size check-duplicates

# プッシュ前の全チェック（軽量チェック + API テスト）
check-all: check test-api

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
    docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml down -v
    cd backend && cargo clean
    cd frontend && rm -rf node_modules elm-stuff dist

# 不要なブランチとワークツリーを整理（マージ済み・リモート削除済みを検出）
# 使い方: just cleanup [--dry-run]
cleanup *flags:
    ./scripts/cleanup.sh {{flags}}

# マージ済みローカルブランチを削除（cleanup に統合済み。後方互換のため残す）
clean-branches:
    just cleanup

# =============================================================================
# Worktree 管理（並行開発用）
# =============================================================================

# Issue 番号から worktree を作成
# 使い方: just worktree-issue NUMBER
# 例: just worktree-issue 321
# Issue タイトルからブランチ名を自動生成する
worktree-issue number:
    ./scripts/worktree-issue.sh {{number}}

# worktree を追加（並行開発用の独立した作業ディレクトリを作成）
# 使い方: just worktree-add NAME BRANCH [--no-setup]
# 例: just worktree-add auth feature/auth
# ポートオフセットは自動で空き番号が割り当てられる
worktree-add name branch *flags:
    ./scripts/worktree-add.sh {{flags}} {{name}} {{branch}}

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
    containers=$(docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml ps -q 2>/dev/null || true)
    if [[ -n "$containers" ]]; then
        echo "  Docker コンテナを停止中..."
        docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml down -v
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
