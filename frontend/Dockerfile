# RingiFlow Frontend Dockerfile
#
# Elm + Vite アプリケーションをビルドして静的ファイルを出力。
# 出力された dist/ は Nginx コンテナにマウントして配信。
#
# ビルド例:
#   docker build -t ringiflow-frontend .
#
# 静的ファイルの取り出し例:
#   docker create --name frontend-build ringiflow-frontend
#   docker cp frontend-build:/app/dist ./dist
#   docker rm frontend-build

# ============================================================
# Stage 1: Dependencies（依存関係のインストール）
# ============================================================
FROM node:22-slim AS deps

# pnpm を有効化
RUN corepack enable pnpm

WORKDIR /app

# 依存関係ファイルをコピー
COPY package.json pnpm-lock.yaml ./

# 依存関係をインストール（キャッシュ対象）
RUN pnpm install --frozen-lockfile

# ============================================================
# Stage 2: Builder（アプリケーションのビルド）
# ============================================================
FROM node:22-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN corepack enable pnpm

# Elm コンパイラをグローバルインストール
# ローカル開発環境と同様にグローバルインストールする（ADR-002 参照）
RUN npm install -g elm

WORKDIR /app

# 依存関係をコピー
COPY --from=deps /app/node_modules ./node_modules

# ソースコードをコピー
COPY . .

# デモ環境では --build-arg VITE_DEV_AUTH=true で DevAuth Cookie を有効化
# Vite がビルド時に import.meta.env.VITE_DEV_AUTH をリテラルに置換する
ARG VITE_DEV_AUTH=false
ENV VITE_DEV_AUTH=${VITE_DEV_AUTH}

# 本番用ビルド
RUN pnpm run build

# ============================================================
# Stage 3: Output（成果物のみを出力）
# ============================================================
FROM busybox:stable AS output

WORKDIR /app

# ビルド成果物のみをコピー
COPY --from=builder /app/dist ./dist

# このイメージ自体は実行しない（成果物取り出し用）
CMD ["ls", "-la", "/app/dist"]
