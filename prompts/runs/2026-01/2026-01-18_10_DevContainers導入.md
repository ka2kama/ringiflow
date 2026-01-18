# 2026-01-18_10: Dev Containers 導入

## 概要

「Clone → Reopen in Container」で即座に開発を開始できる Dev Containers 環境を構築した。
docker-compose の設計について議論し、既存定義を `include` で参照する DRY な構成に改善した。

## 背景と目的

RingiFlow の開発環境構築には 10 種類以上のツールが必要で、手順書に沿っても 30 分〜1 時間かかる。
Dev Containers を導入することで、Docker さえあれば 5 分で開発環境を構築できるようにする。

## 実施内容

### 1. Dev Containers 環境の構築

以下のファイルを作成:

| ファイル | 目的 |
|----------|------|
| `.devcontainer/Dockerfile` | Rust 1.92 + Node.js 22 + Elm 0.19.1 + 開発ツール |
| `.devcontainer/docker-compose.yml` | 既存 compose を include + app コンテナ追加 |
| `.devcontainer/post-create.sh` | 依存関係インストール・Git フックセットアップ |
| `.devcontainer/devcontainer.json` | VS Code + JetBrains Gateway 両対応設定 |

### 2. docker-compose 設計の議論と改善

当初は PostgreSQL/Redis を `.devcontainer/docker-compose.yml` に重複定義していたが、
議論の結果、既存の `infra/docker/docker-compose.yml` を `include` で参照する構成に改善した。

```yaml
# .devcontainer/docker-compose.yml
include:
  - path: ../infra/docker/docker-compose.yml

services:
  app:
    # Dev Container 固有の設定のみ
```

### 3. JetBrains Gateway 設定の改善

当初は特定の IDE（CLion）を設定で固定していたが、
開発者によって使用する IDE が異なるため、接続時に選択できるよう空設定に変更した。

### 4. ドキュメント作成

- ADR-018 作成
- 開発環境構築手順書に Dev Containers セクション追加

## 成果物

### 作成/更新ファイル

| ファイル | 内容 |
|---------|------|
| `.devcontainer/Dockerfile` | 新規作成 |
| `.devcontainer/docker-compose.yml` | 新規作成 |
| `.devcontainer/post-create.sh` | 新規作成 |
| `.devcontainer/devcontainer.json` | 新規作成 |
| `docs/05_ADR/018_DevContainers導入.md` | 新規作成 |
| `docs/04_手順書/01_開発参画/01_開発環境構築.md` | Dev Containers セクション追加 |

## 設計判断と実装解説

### なぜ Rust ベースの devcontainer イメージを選んだか

MS 公式の `devcontainers/rust:1-bookworm` をベースに採用した理由:

1. **Rust がプロジェクトの中核**: rust-analyzer、cargo、rustfmt が事前設定済み
2. **Node.js は追加が容易**: NodeSource のセットアップスクリプトで追加
3. **Elm もグローバルインストール**: pnpm ではネイティブバイナリが正しく動作しないため

### docker-compose の `include` による一元化

当初は「既存環境と競合しないように別コンテナ」という設計だったが、
「新規プロジェクトと仮定すると？」という問いを受けて再検討した。

結論: **インフラ定義は `infra/` に集約し、Dev Containers はそれを `include` で参照**

```
infra/docker/docker-compose.yml      → PostgreSQL, Redis の唯一の定義
        ↑
        include
        ↑
.devcontainer/docker-compose.yml     → app コンテナのみ追加
```

メリット:
- PostgreSQL/Redis の設定変更は 1 箇所で済む（DRY）
- 設定の乖離によるバグを防止

### named volume によるパフォーマンス最適化

macOS/Windows の Docker はファイル共有に FUSE を使用するため、
大量ファイルを含むディレクトリは named volume に分離すると 10〜50 倍高速化できる。

```yaml
volumes:
  - node_modules:/workspace/frontend/node_modules
  - cargo_target:/workspace/backend/target
  - cargo_registry:/home/vscode/.cargo/registry
```

### JetBrains Gateway 設定の柔軟性

`customizations.jetbrains.backend` を指定すると IDE が固定されるが、
チームメンバーによって CLion、RustRover、IntelliJ IDEA など使用 IDE が異なるため、
空オブジェクト `{}` にして接続時に選択できるようにした。

これは「個人の好みに関わる部分は設定で固定しない」というプラクティス。

## ユーザープロンプト（抜粋）

> devcontainer に詳しくないのだが、なぜ PostgreSQL と Redis の compose が 2 つあるのか？

> これが新規プロジェクトと仮定すると？

> 人によって使用する JetBrains IDE は異なるのでは？

## 学んだこと

1. **docker-compose の `include` 機能**
   - Compose v2.20+ で導入
   - 他の compose ファイルを取り込んで設定を一元化できる

2. **設定の柔軟性と固定のバランス**
   - チーム共通の設定は固定（ツールバージョン等）
   - 個人の好み（IDE 選択等）は固定しない

3. **「新規プロジェクトなら？」という視点**
   - 既存との互換性を考慮しすぎると、最適な設計から外れることがある
   - 新規前提で考えると DRY でシンプルな構成が見えてくる
