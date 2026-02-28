# RingiFlow 開発タスク

# .env ファイルを自動読み込み
set dotenv-load := true

# quiet モード（true で警告・エラーのみ表示、false で全出力）
# 使い方: just quiet=false check
quiet := "true"
_cargo_q := if quiet == "true" { "--quiet" } else { "" }

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
setup: setup-mise check-tools setup-env setup-hooks dev-deps setup-db setup-root-deps setup-deps
    @echo ""
    @echo "✓ セットアップ完了"
    @echo "  - just dev-all         : 全サーバー一括起動（推奨）"
    @echo "  - just dev-bff         : BFF 起動"
    @echo "  - just dev-core-service: Core Service 起動"
    @echo "  - just dev-auth-service: Auth Service 起動"
    @echo "  - just dev-web         : フロントエンド起動"

# mise の設定ファイルを信頼済みにする（mise がインストール済みの場合のみ）
setup-mise:
    @which mise > /dev/null 2>&1 && mise trust || true

# 開発ツールのインストール確認
check-tools:
    @echo "開発ツールを確認中..."
    @which rustc > /dev/null || (echo "ERROR: Rust がインストールされていません" && exit 1)
    @which cargo > /dev/null || (echo "ERROR: Cargo がインストールされていません" && exit 1)
    @rustfmt +nightly --version > /dev/null 2>&1 || (echo "ERROR: rustfmt-nightly がインストールされていません" && exit 1)
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
    @which cargo-outdated > /dev/null || (echo "ERROR: cargo-outdated がインストールされていません" && exit 1)
    @which cargo-llvm-cov > /dev/null || (echo "ERROR: cargo-llvm-cov がインストールされていません" && exit 1)
    @which cargo-machete > /dev/null || (echo "ERROR: cargo-machete がインストールされていません" && exit 1)
    @which sccache > /dev/null || (echo "ERROR: sccache がインストールされていません" && exit 1)
    @which rust-script > /dev/null || (echo "ERROR: rust-script がインストールされていません" && exit 1)
    @which mprocs > /dev/null || (echo "ERROR: mprocs がインストールされていません" && exit 1)
    @which gh > /dev/null || (echo "ERROR: GitHub CLI (gh) がインストールされていません" && exit 1)
    @which psql > /dev/null || (echo "ERROR: psql がインストールされていません" && exit 1)
    @which pg_dump > /dev/null || (echo "ERROR: pg_dump がインストールされていません" && exit 1)
    @which redis-cli > /dev/null || (echo "ERROR: redis-cli がインストールされていません" && exit 1)
    @cd tests/e2e && npx playwright --version > /dev/null 2>&1 || echo "  ⚠ Playwright: 未インストール（E2E テスト用: cd tests/e2e && pnpm install && npx playwright install chromium）"
    @echo "✓ 全ツール確認済み"

# .env ファイルを作成（既存の場合はスキップ）
# worktree の場合は空きポートオフセットを自動割り当て
setup-env:
    ./scripts/env/setup.sh

# ルート開発ツールをインストール（@redocly/cli, jscpd）
setup-root-deps:
    @echo "ルート開発ツールをインストール中..."
    @pnpm install
    @echo "✓ ルート開発ツールインストール完了"

# 依存関係をインストール
setup-deps:
    @echo "依存関係をインストール中..."
    @echo "  Rust..."
    @cd backend && cargo build
    @echo "  Elm/Vite..."
    @cd frontend && pnpm install
    @echo "  E2E テスト..."
    @cd tests/e2e && pnpm install
    @echo "✓ 依存関係インストール完了"

# Git フックをセットアップ
setup-hooks:
    @echo "Git フックをセットアップ中..."
    @lefthook install
    @echo "✓ Git フックセットアップ完了"

# データベースをセットアップ（マイグレーション適用 + スキーマスナップショット更新）
setup-db:
    @echo "データベースをセットアップ中..."
    @cd backend && sqlx migrate run
    @just db-dump-schema
    @echo "✓ データベースセットアップ完了"

# worktree 用セットアップ（Docker 起動 → DB マイグレーション → 依存関係インストール）
setup-worktree: dev-deps setup-db setup-root-deps setup-deps
    @echo ""
    @echo "✓ worktree セットアップ完了"

# データベースをリセット（drop → create → migrate + スキーマスナップショット更新）
reset-db:
    @echo "データベースをリセット中..."
    cd backend && sqlx database reset -y
    just db-dump-schema
    @echo "✓ データベースリセット完了"

# =============================================================================
# GitHub 設定
# =============================================================================

# GitHub ラベルを一括作成（冪等: 既存ラベルはスキップ）
# → 詳細: docs/60_手順書/02_プロジェクト構築/03_GitHub設定.md#7-labels
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

# Docker で依存サービス（PostgreSQL, Redis, DynamoDB）を起動
# --wait: healthcheck が通るまで待機（setup 時の競合を防止）
# プロジェクト名はディレクトリ名から自動取得（worktree対応）
dev-deps:
    #!/usr/bin/env bash
    PROJECT_NAME=$(basename "$(pwd)")
    docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml up -d --wait
    echo "PostgreSQL: localhost:${POSTGRES_PORT}"
    echo "Redis: localhost:${REDIS_PORT}"
    echo "DynamoDB: localhost:${DYNAMODB_PORT}"
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

# バックエンド全体を監視・ビルド（dev-all で使用）
dev-build:
    cd backend && cargo watch -x 'build --workspace'

# ビルド済みバイナリを監視して起動（dev-all で使用）
dev-run service:
    ./scripts/dev/run-service.sh {{service}}

# 全開発サーバーを一括起動（依存サービス + mprocs）
dev-all: dev-deps
    mprocs

# Ghostty + Wayland でキー入力がフリーズする場合の回避策（X11 で別ウィンドウ起動）
dev-all-x11: dev-deps
    GDK_BACKEND=x11 ghostty -e just dev-all

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

_psql_url := "postgres://ringiflow:ringiflow@localhost:" + env_var_or_default("POSTGRES_PORT", "15432") + "/ringiflow"

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

# PostgreSQL: 現在のスキーマスナップショットを出力
db-dump-schema:
    ./scripts/tools/dump-schema.sh "{{ _psql_url }}" > backend/schema.sql
    @echo "✓ backend/schema.sql を更新しました"

# データベースマイグレーション実行 + スキーマスナップショット更新
db-migrate:
    @echo "マイグレーション実行中..."
    cd backend && sqlx migrate run
    just db-dump-schema

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
    cd backend && cargo clippy {{ _cargo_q }} --all-targets --all-features

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
    pnpm exec redocly lint --config openapi/redocly.yaml

# =============================================================================
# テスト
# =============================================================================

# 全テスト（単体テストのみ）
test: test-rust test-elm test-rust-scripts

# Rust 単体テスト + doctest
test-rust:
    cd backend && cargo test {{ _cargo_q }} --all-features --lib --bins
    cd backend && cargo test {{ _cargo_q }} --all-features --doc

# Rust 統合テスト（DB 接続が必要）
test-rust-integration:
    cd backend && cargo test {{ _cargo_q }} --all-features --test '*'

# Elm テスト
test-elm:
    cd frontend && pnpm run test

# rust-script ユニットテスト（Cargo workspace 外の独立スクリプト）
test-rust-scripts:
    rust-script --test ./scripts/check/instrumentation.rs
    rust-script --test ./scripts/check/improvement-records.rs
    rust-script --test ./scripts/check/impl-docs.rs
    rust-script --test ./scripts/issue/sync-epic.rs

# Elm ビルドチェック（コンパイルエラー検出）
# lint-elm や test-elm ではコンパイルエラーを検出できないため、
# 実際にビルドしてコンパイルエラーがないことを確認する
build-elm:
    cd frontend && pnpm run build

# =============================================================================
# API テスト
# =============================================================================

# API テスト用の DB/Redis/DynamoDB を起動（開発環境とは独立）
# プロジェクト名はディレクトリ名から自動取得（worktree 対応）
api-test-deps: setup-env
    #!/usr/bin/env bash
    PROJECT_NAME="$(basename "$(pwd)")-api-test"
    docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.api-test.yaml up -d --wait
    echo "API テスト環境:"
    echo "  PostgreSQL: localhost:${API_TEST_POSTGRES_PORT}"
    echo "  Redis: localhost:${API_TEST_REDIS_PORT}"
    echo "  DynamoDB: localhost:${API_TEST_DYNAMODB_PORT}"
    echo "  プロジェクト名: $PROJECT_NAME"

# API テスト用の DB をリセット
api-test-reset-db:
    ./scripts/test/reset-db.sh

# API テスト用の DB/Redis を停止
api-test-stop:
    #!/usr/bin/env bash
    PROJECT_NAME="$(basename "$(pwd)")-api-test"
    docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.api-test.yaml down

# API テスト用の DB/Redis を削除（データ含む）
api-test-clean:
    #!/usr/bin/env bash
    PROJECT_NAME="$(basename "$(pwd)")-api-test"
    docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.api-test.yaml down -v

# API テスト実行（hurl）
# サービスを起動してテストを実行し、終了後にサービスを停止する
test-api: api-test-deps api-test-reset-db
    ./scripts/test/run-api.sh

# E2E テスト実行（Playwright）
# バックエンド + Vite を起動してブラウザテストを実行し、終了後にサービスを停止する
test-e2e: api-test-deps api-test-reset-db
    ./scripts/test/run-e2e.sh

# =============================================================================
# セキュリティチェック
# =============================================================================

# 依存関係の脆弱性・ライセンスチェック（cargo-deny）
audit:
    cd backend && cargo deny check

# =============================================================================
# 依存関係鮮度チェック
# =============================================================================

# 依存関係の更新状況を確認（cargo-outdated + pnpm outdated）
outdated:
    cd backend && cargo outdated --root-deps-only
    -cd frontend && pnpm outdated

# =============================================================================
# 構造品質チェック
# =============================================================================

# ソースファイルの行数閾値チェック（500 行超で警告）
check-file-size:
    ./scripts/check/file-size.sh

# 関数の行数閾値チェック（50 行超で警告）
check-fn-size:
    ./scripts/check/fn-size.sh {{ _cargo_q }}

# コード重複（コピー＆ペースト）を検出（jscpd）
# 警告のみ（exit 0）: CI をブロックしない。重複の可視化が目的。
# 選定理由: docs/70_ADR/042_コピペ検出ツールの選定.md
# ベースライン: .config/baselines.env（ラチェット方式、閾値超過で exit 1）
check-duplicates:
    ./scripts/check/check-duplicates.sh

# 改善記録の標準フォーマット準拠チェック（カテゴリ・失敗タイプの値検証）
# ベースライン: .config/baselines.env（ラチェット方式、閾値超過で exit 1）
lint-improvements:
    #!/usr/bin/env bash
    set -euo pipefail
    source .config/baselines.env
    rust-script ./scripts/check/improvement-records.rs \
        --max-missing-nature "$IMPROVEMENT_RECORDS_MAX_MISSING_NATURE"

# 改善記録のサニタイズ違反検出（ユーザー発言の直接引用）
lint-improvements-sanitize:
    ./scripts/check/sanitize-improvements.sh

# .claude/rules/ 内のルールファイルが CLAUDE.md または paths: で参照されているかチェック
lint-rules:
    ./scripts/check/rule-files.sh

# ドキュメント内の相対パスリンク切れをチェック
# 警告のみ（exit 0）: 既存のリンク切れが多数あるため、ブロックしない
check-doc-links:
    ./scripts/check/doc-links.sh

# クローズ済み Issue を参照する TODO/FIXME を検出
check-stale-annotations:
    ./scripts/check/stale-annotations.sh

# 実装解説のファイル命名規則をチェック
# 警告のみ（exit 0）: 既存の違反がある場合にブロックしない
check-impl-docs:
    rust-script ./scripts/check/impl-docs.rs

# 計画ファイルの確認事項チェック漏れを検出
lint-plans:
    ./scripts/check/plan-confirmations.sh

# 計画ファイルのテスト層網羅確認
lint-plan-test-layers:
    ./scripts/check/plan-test-layers.sh

# 計画ファイルの命名規則違反検出
lint-plan-filenames:
    ./scripts/check/check-plan-filenames.sh

# OpenAPI ハンドラ登録照合（#[utoipa::path] と openapi.rs の paths() の一致確認）
lint-openapi-handlers:
    ./scripts/check/openapi-handler-registration.sh

# 計装（tracing::instrument）の漏れを検出
check-instrumentation:
    rust-script ./scripts/check/instrumentation.rs

# =============================================================================
# 未使用依存チェック
# =============================================================================

# Cargo.toml の未使用依存を検出（cargo-machete）
# 選定理由: docs/70_ADR/038_未使用依存検出ツールの選定.md
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

# pre-push フックで使用するチェック（DB 不要のサブセット）
# lint + unit test + ビルド + 構造品質チェック。統合テスト・SQLx・スキーマチェックを除外。
check-pre-push:
    ./scripts/check/parallel.sh --skip-db

# 実装中の軽量チェック（リント、テスト、統合テスト、ビルド、SQLx キャッシュ同期、OpenAPI 同期、構造品質）
# Rust レーンと Non-Rust レーンを並列実行して高速化
check:
    ./scripts/check/parallel.sh

# プッシュ前の全チェック（軽量チェック + API テスト + E2E テスト + セキュリティ）
# audit を最後に配置: cargo deny が取得するパッケージキャッシュのロックが
# 後続の cargo build に影響し、サービス起動タイムアウトを引き起こすため (#596)
check-all: check test-api test-e2e audit

# OpenAPI 仕様書を utoipa から生成して openapi/openapi.yaml に出力
openapi-generate:
    cd backend && cargo run --bin generate-openapi -p ringiflow-bff 2>/dev/null > ../openapi/openapi.yaml
    @echo "✓ openapi/openapi.yaml を生成しました"

# OpenAPI 仕様書の同期チェック（utoipa 生成結果と openapi/openapi.yaml を比較）
openapi-check:
    #!/usr/bin/env bash
    set -euo pipefail
    temp=$(mktemp)
    trap 'rm -f "$temp"' EXIT
    cd backend && cargo run --bin generate-openapi -p ringiflow-bff > "$temp" 2>/dev/null
    cd ..
    if ! diff -q openapi/openapi.yaml "$temp" > /dev/null 2>&1; then
        echo "ERROR: openapi/openapi.yaml が utoipa の定義と同期していません"
        echo "  'just openapi-generate' を実行して更新してください"
        diff --unified openapi/openapi.yaml "$temp" || true
        exit 1
    fi
    echo "✓ openapi/openapi.yaml は utoipa の定義と同期しています"

# スキーマスナップショットの同期チェック（pg_dump 出力と backend/schema.sql を比較）
schema-check:
    #!/usr/bin/env bash
    set -euo pipefail
    temp=$(mktemp)
    trap 'rm -f "$temp"' EXIT
    ./scripts/tools/dump-schema.sh "{{ _psql_url }}" > "$temp"
    if ! diff -q backend/schema.sql "$temp" > /dev/null 2>&1; then
        echo "ERROR: backend/schema.sql が現在の DB スキーマと同期していません"
        echo "  'just db-dump-schema' を実行して更新してください"
        diff --unified backend/schema.sql "$temp" || true
        exit 1
    fi
    echo "✓ backend/schema.sql は現在の DB スキーマと同期しています"

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
    ./scripts/worktree/cleanup.sh {{flags}}

# マージ済みローカルブランチを削除（cleanup に統合済み。後方互換のため残す）
clean-branches:
    just cleanup

# =============================================================================
# Worktree 管理（永続スロット方式）
# =============================================================================

# 永続 worktree スロットを作成（初回のみ）
# 使い方: just worktree-create N
# 例: just worktree-create 1
# スロット番号がポートオフセットとして使用される
worktree-create n:
    ./scripts/worktree/create.sh {{n}}

# worktree スロット内のブランチを切り替え
# 使い方: just worktree-switch N BRANCH
# 例: just worktree-switch 1 feature/625-persistent-slots
# DB マイグレーションと依存関係の差分更新を自動で行う
worktree-switch n branch:
    ./scripts/worktree/switch.sh {{n}} {{branch}}

# Issue 番号からブランチを作成してスロットに切り替え
# 使い方: just worktree-issue NUMBER [SLOT]
# 例: just worktree-issue 321 1
# SLOT を省略した場合、現在のスロットを自動検出
worktree-issue number *slot:
    ./scripts/worktree/issue.sh {{number}} {{slot}}

# worktree を削除
# 使い方: just worktree-remove NAME
worktree-remove name:
    #!/usr/bin/env bash
    set -euo pipefail
    PARENT_DIR=$(dirname "$(pwd)")
    WORKTREE_PATH="${PARENT_DIR}/ringiflow-{{name}}"
    PROJECT_NAME="ringiflow-{{name}}"

    echo "worktree を削除中: {{name}}"

    # Docker コンテナ・ボリュームを停止・削除
    # コンテナが停止済みでもボリュームが残っている場合があるため、常に実行する
    echo "  Docker リソースを削除中..."
    docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.yaml down -v 2>/dev/null || true

    # worktree を削除
    git worktree remove "$WORKTREE_PATH" --force

    # Docker がバインドマウントで作成した root 所有ディレクトリが残る場合があるため削除
    if [ -d "$WORKTREE_PATH" ]; then
        echo "  残存ディレクトリを削除中..."
        rm -rf "$WORKTREE_PATH"
    fi

    echo "✓ worktree を削除しました: {{name}}"

# worktree 一覧を表示
worktree-list:
    @echo "=== Worktree 一覧 ==="
    @git worktree list
    @echo ""
    @echo "=== Docker プロジェクト一覧 ==="
    @docker compose ls --filter "name=ringiflow" 2>/dev/null || echo "（実行中のプロジェクトなし）"

# =============================================================================
# Issue/Epic 状態管理
# =============================================================================

# Issue のチェックボックス状態を検証する
# 使い方: just check-issue [ISSUE_NUMBER]
# ISSUE_NUMBER を省略した場合、ブランチ名から自動検出
check-issue *args:
    ./scripts/issue/check-issue-state.sh {{args}}

# Story 完了後に Epic タスクリストを自動更新する
# 使い方: just sync-epic ISSUE_NUMBER
# 例: just sync-epic 749
sync-epic issue_number:
    rust-script ./scripts/issue/sync-epic.rs {{issue_number}}
