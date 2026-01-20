# RingiFlow Backend Dockerfile
#
# マルチステージビルドで BFF と Core API の両方をビルド。
# cargo-chef を使用して依存関係のキャッシュを最適化。
#
# ビルド例:
#   docker build -t ringiflow-backend .
#
# 実行例（BFF）:
#   docker run -e RUST_LOG=info ringiflow-backend ringiflow-bff
#
# 実行例（Core API）:
#   docker run -e RUST_LOG=info ringiflow-backend ringiflow-core-api

# ============================================================
# Stage 1: Chef（依存関係の準備）
# ============================================================
FROM rust:1.92-slim-bookworm AS chef

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked
WORKDIR /app

# ============================================================
# Stage 2: Planner（依存関係の抽出）
# ============================================================
FROM chef AS planner

# ソースコードをコピー
COPY Cargo.toml Cargo.lock ./
COPY apps/ apps/
COPY crates/ crates/

# レシピファイルを生成（依存関係の情報のみ）
RUN cargo chef prepare --recipe-path recipe.json

# ============================================================
# Stage 3: Builder（依存関係のビルド + アプリのビルド）
# ============================================================
FROM chef AS builder

# レシピファイルをコピーして依存関係をビルド（キャッシュ対象）
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# ソースコードをコピー
COPY Cargo.toml Cargo.lock ./
COPY apps/ apps/
COPY crates/ crates/

# SQLx のオフラインモード用データをコピー
COPY .sqlx/ .sqlx/
ENV SQLX_OFFLINE=true

# アプリケーションをビルド
RUN cargo build --release --bin ringiflow-bff --bin ringiflow-core-api

# ============================================================
# Stage 4: Runtime（最小限の実行環境）
# ============================================================
FROM debian:bookworm-slim AS runtime

# 必要なランタイム依存関係のみインストール
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 非 root ユーザーを作成
RUN useradd --create-home --user-group ringiflow
USER ringiflow
WORKDIR /home/ringiflow

# ビルド成果物をコピー
COPY --from=builder --chown=ringiflow:ringiflow /app/target/release/ringiflow-bff .
COPY --from=builder --chown=ringiflow:ringiflow /app/target/release/ringiflow-core-api .

# 環境変数のデフォルト値
ENV RUST_LOG=info

# ヘルスチェック用のポート（実行するバイナリに応じて変更）
EXPOSE 13000 13001

# デフォルトは BFF を起動（docker-compose で上書き可能）
CMD ["./ringiflow-bff"]
