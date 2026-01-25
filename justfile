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
# 開発サーバー
# =============================================================================

# Docker で依存サービス（PostgreSQL, Redis）を起動
# --wait: healthcheck が通るまで待機（setup 時の競合を防止）
# プロジェクト名はディレクトリ名から自動取得（worktree対応）
dev-deps:
    #!/usr/bin/env bash
    PROJECT_NAME=$(basename "$(pwd)")
    docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.yml up -d --wait
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
lint: lint-rust lint-elm lint-shell

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

# E2E テスト（hurl）
# 事前に dev-deps, reset-db を実行し、BFF/Core/Auth の各サービスを起動しておくこと
test-e2e:
    hurl --test --variables-file tests/e2e/hurl/vars.env tests/e2e/hurl/**/*.hurl

# =============================================================================
# 全チェック
# =============================================================================

# プッシュ前の全チェック（リント、テスト、SQLx キャッシュ同期）
check-all: lint test sqlx-check

# SQLx オフラインキャッシュの同期チェック（DB 接続が必要）
# --all-targets: 統合テスト内の sqlx::query! マクロも含めてチェック
sqlx-check:
    cd backend && cargo sqlx prepare --check --workspace -- --all-targets

# =============================================================================
# クリーンアップ
# =============================================================================

# ビルド成果物とコンテナを削除
# プロジェクト名はディレクトリ名から自動取得（worktree対応）
clean:
    #!/usr/bin/env bash
    PROJECT_NAME=$(basename "$(pwd)")
    docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.yml down -v
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
    #!/usr/bin/env bash
    set -euo pipefail
    PARENT_DIR=$(dirname "$(pwd)")
    WORKTREE_PATH="${PARENT_DIR}/ringiflow-{{name}}"

    # 使用中のオフセットを収集（各 worktree の .env から POSTGRES_PORT を読み取り）
    used_offsets=()
    while IFS= read -r wt_path; do
        env_file="$wt_path/.env"
        if [[ -f "$env_file" ]]; then
            port=$(grep -E '^POSTGRES_PORT=' "$env_file" 2>/dev/null | cut -d= -f2)
            if [[ -n "$port" ]]; then
                # ベースポート 15432 からのオフセットを計算（100単位）
                offset=$(( (port - 15432) / 100 ))
                used_offsets+=("$offset")
            fi
        fi
    done < <(git worktree list --porcelain | grep '^worktree ' | cut -d' ' -f2-)

    # 空きオフセットを探す（1-9、0 はメイン用）
    port_offset=""
    for i in {1..9}; do
        found=false
        # 配列が空でない場合のみチェック
        if [[ ${#used_offsets[@]} -gt 0 ]]; then
            for used in "${used_offsets[@]}"; do
                if [[ "$used" == "$i" ]]; then
                    found=true
                    break
                fi
            done
        fi
        if [[ "$found" == false ]]; then
            port_offset="$i"
            break
        fi
    done

    if [[ -z "$port_offset" ]]; then
        echo "エラー: 空きポートオフセットがありません（最大9個まで）" >&2
        exit 1
    fi

    echo "worktree を作成中: {{name}}"
    echo "  パス: $WORKTREE_PATH"
    echo "  ブランチ: {{branch}}"
    echo "  ポートオフセット: $port_offset（自動割り当て）"

    # worktree を追加（ブランチがなければ作成）
    if git rev-parse --verify "{{branch}}" >/dev/null 2>&1; then
        git worktree add "$WORKTREE_PATH" "{{branch}}"
    else
        git worktree add -b "{{branch}}" "$WORKTREE_PATH"
    fi

    # .env を生成
    cd "$WORKTREE_PATH"
    ./scripts/generate-env.sh "$port_offset"

    echo ""
    echo "✓ worktree を作成しました"
    echo "  cd $WORKTREE_PATH"
    echo "  just dev-deps  # 依存サービスを起動"

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
    containers=$(docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.yml ps -q 2>/dev/null || true)
    if [[ -n "$containers" ]]; then
        echo "  Docker コンテナを停止中..."
        docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.yml down -v
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
