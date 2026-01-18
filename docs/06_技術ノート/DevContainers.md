# Dev Containers

コンテナベースの開発環境を提供する仕組み。VS Code および JetBrains Gateway で利用可能。

公式: https://containers.dev/

## 概要

Dev Containers（旧称: VS Code Remote - Containers）は、Docker コンテナ内で開発環境を構築する仕組み。
`devcontainer.json` で設定を定義し、IDE から「Reopen in Container」で即座に開発環境を構築できる。

主な利点:
- **環境の再現性**: 全開発者が同一の環境を使用
- **セットアップの簡素化**: Clone → Reopen in Container で完了
- **GitHub Codespaces 互換**: ブラウザベースの開発も可能

## ファイル構成

```
.devcontainer/
├── devcontainer.json    # メイン設定
├── Dockerfile           # コンテナイメージ定義
├── docker-compose.yml   # 複数サービス定義（任意）
└── post-create.sh       # 初期化スクリプト（任意）
```

## devcontainer.json の基本

### 最小構成（Dockerfile のみ）

```json
{
  "name": "My Project",
  "build": {
    "dockerfile": "Dockerfile"
  },
  "postCreateCommand": "npm install"
}
```

### docker-compose 連携

```json
{
  "name": "My Project",
  "dockerComposeFile": "docker-compose.yml",
  "service": "app",
  "workspaceFolder": "/workspace",
  "postCreateCommand": "bash .devcontainer/post-create.sh"
}
```

## docker-compose の `include` 機能

Compose v2.20+ で導入された機能。他の compose ファイルを取り込める。

```yaml
# .devcontainer/docker-compose.yml
include:
  - path: ../infra/docker/docker-compose.yml

services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
    depends_on:
      postgres:
        condition: service_healthy
```

メリット:
- インフラ定義を一元管理（DRY）
- 設定の乖離を防止

## パフォーマンス最適化

### named volume による高速化

macOS/Windows の Docker はホストとのファイル共有が遅い。
`node_modules` や `target` を named volume に分離すると 10〜50 倍高速化できる。

```yaml
services:
  app:
    volumes:
      - ..:/workspace:cached
      - node_modules:/workspace/node_modules
      - cargo_target:/workspace/target

volumes:
  node_modules:
  cargo_target:
```

### `:cached` オプション

ホストからコンテナへの同期を非同期にする。
ソースコードのマウントに使用すると読み取りが高速化。

```yaml
volumes:
  - ..:/workspace:cached
```

## VS Code 設定

### 拡張機能の自動インストール

```json
{
  "customizations": {
    "vscode": {
      "extensions": [
        "rust-lang.rust-analyzer",
        "elmtooling.elm-ls-vscode"
      ]
    }
  }
}
```

### エディタ設定

```json
{
  "customizations": {
    "vscode": {
      "settings": {
        "editor.formatOnSave": true,
        "[rust]": {
          "editor.defaultFormatter": "rust-lang.rust-analyzer"
        }
      }
    }
  }
}
```

## JetBrains Gateway 設定

JetBrains Gateway も devcontainer.json を読み取れる。

```json
{
  "customizations": {
    "jetbrains": {
      "backend": "CLion"
    }
  }
}
```

`backend` を空にすると接続時に IDE を選択可能:

```json
{
  "customizations": {
    "jetbrains": {}
  }
}
```

## Features

MS 提供の追加機能を簡単にインストールできる。

```json
{
  "features": {
    "ghcr.io/devcontainers/features/git:1": {},
    "ghcr.io/devcontainers/features/github-cli:1": {},
    "ghcr.io/devcontainers/features/node:1": {
      "version": "22"
    }
  }
}
```

利用可能な Features: https://containers.dev/features

## マウント設定

ホストの設定ファイルをコンテナにマウントして共有できる。

```json
{
  "mounts": [
    "source=${localEnv:HOME}/.gitconfig,target=/home/vscode/.gitconfig,type=bind,consistency=cached",
    "source=${localEnv:HOME}/.ssh,target=/home/vscode/.ssh,type=bind,consistency=cached"
  ]
}
```

## ライフサイクルフック

| フック | タイミング | 用途 |
|--------|-----------|------|
| `initializeCommand` | コンテナ構築前（ホスト側） | 前提条件の確認 |
| `onCreateCommand` | コンテナ作成時（初回のみ） | 永続的な初期化 |
| `postCreateCommand` | コンテナ作成後 | 依存関係インストール |
| `postStartCommand` | コンテナ起動毎 | サービス起動 |
| `postAttachCommand` | IDE 接続毎 | 環境変数設定 |

よく使うのは `postCreateCommand`:

```json
{
  "postCreateCommand": "bash .devcontainer/post-create.sh"
}
```

## RingiFlow での使用

### 構成

```
.devcontainer/
├── devcontainer.json    # メイン設定（VS Code + JetBrains 両対応）
├── Dockerfile           # Rust 1.92 + Node.js 22 + Elm 0.19.1
├── docker-compose.yml   # 既存 compose を include + app 追加
└── post-create.sh       # 依存関係インストール
```

### docker-compose の include

```yaml
include:
  - path: ../infra/docker/docker-compose.yml

services:
  app:
    # Dev Container 固有の設定のみ
```

`infra/docker/docker-compose.yml` の PostgreSQL/Redis 定義を再利用し、
app コンテナのみを追加する DRY な構成。

### 使い方

1. リポジトリをクローン
2. VS Code で「Reopen in Container」または JetBrains Gateway で接続
3. 初回はコンテナビルド + 依存関係インストール（5〜10 分）
4. `just dev-bff` / `just dev-web` で開発サーバー起動

## トラブルシューティング

### コンテナのビルドに失敗する

```bash
# キャッシュをクリアして再ビルド
docker compose -f .devcontainer/docker-compose.yml build --no-cache
```

### ファイルの変更が反映されない

named volume にキャッシュされている可能性:

```bash
# node_modules を再インストール
rm -rf frontend/node_modules
pnpm install
```

### ポートが競合する

```bash
# 既存コンテナを停止
docker compose -f infra/docker/docker-compose.yml down
```

## 参考資料

- [Dev Containers 仕様](https://containers.dev/)
- [VS Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/containers)
- [JetBrains Gateway](https://www.jetbrains.com/remote-development/gateway/)
- [devcontainer Features](https://containers.dev/features)
