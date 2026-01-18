# RingiFlow Dev Container
#
# ベースイメージ: MS 公式 devcontainers/rust:1-bookworm
# - Rust 1.92+ および rust-analyzer が事前設定済み
# - Node.js、Elm は Feature および手動インストールで追加
#
# 詳細: docs/05_ADR/018_DevContainers導入.md

# =============================================================================
# ベースイメージ
# =============================================================================
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# =============================================================================
# システムパッケージ
# =============================================================================
RUN apt-get update && apt-get install -y --no-install-recommends \
    # ShellCheck（シェルスクリプト静的解析）
    shellcheck \
    # PostgreSQL クライアント（psql）
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# =============================================================================
# Node.js 22 LTS インストール（NodeSource）
# =============================================================================
# devcontainers Feature よりも明示的なバージョン管理を優先
ARG NODE_MAJOR=22
RUN curl -fsSL https://deb.nodesource.com/setup_${NODE_MAJOR}.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# =============================================================================
# pnpm インストール
# =============================================================================
# corepack は Node.js 22 に同梱済み
RUN corepack enable && corepack prepare pnpm@latest --activate

# =============================================================================
# Elm ツールチェーン インストール
# =============================================================================
# グローバルインストール（pnpm ではネイティブバイナリが正しく動作しないため）
# 詳細: docs/05_ADR/002_フロントエンドツールチェーンの選定.md
RUN npm install -g elm@0.19.1 elm-format@0.8.7 elm-test@0.19.1-revision17

# =============================================================================
# Rust 追加ツール インストール
# =============================================================================
USER vscode

# nightly ツールチェーン（rustfmt の一部機能に必要）
RUN rustup toolchain install nightly --component rustfmt

# sqlx-cli（マイグレーション管理）
RUN cargo install sqlx-cli --no-default-features --features rustls,postgres

# just（タスクランナー）
RUN cargo install just

# lefthook（Git フック管理）
RUN cargo install lefthook --locked

# cargo-watch（ファイル変更監視）
RUN cargo install cargo-watch

USER root

# =============================================================================
# 環境変数
# =============================================================================
ENV CARGO_HOME=/home/vscode/.cargo
ENV RUSTUP_HOME=/home/vscode/.rustup
ENV PATH="${CARGO_HOME}/bin:${PATH}"

# =============================================================================
# 作業ディレクトリ
# =============================================================================
WORKDIR /workspace
