# ADR-018: Dev Containers 導入

## ステータス

承認済み

## コンテキスト

RingiFlow の開発環境構築には、以下のツールが必要である:

- Rust 1.92+ / rustfmt / clippy
- Node.js 22 LTS / pnpm
- Elm 0.19.1 / elm-format / elm-test
- Docker / Docker Compose
- just / sqlx-cli / lefthook / shellcheck

現状の課題:

1. **セットアップの複雑さ**: 10 種類以上のツールを個別にインストールする必要があり、手順書に沿っても 30 分〜1 時間かかる
2. **環境差異**: OS やツールのバージョン違いにより「自分の環境では動かない」問題が発生しうる
3. **再現性**: 新規参画者が同一の開発環境を再現することが困難

Dev Containers（旧称: VS Code Remote - Containers）を導入することで、これらの課題を解決できる可能性がある。

## 検討した選択肢

### 選択肢 1: Dev Containers

VS Code / JetBrains Gateway が公式サポートするコンテナベースの開発環境。`devcontainer.json` でコンテナ設定を定義し、IDE から直接コンテナ内で開発する。

評価:
- 利点:
  - 「Clone → Reopen in Container」で即座に開発開始可能
  - 全開発者が同一の環境を使用（環境差異ゼロ）
  - VS Code と JetBrains 両方をサポート
  - GitHub Codespaces との互換性
- 欠点:
  - Docker が必須（Docker Desktop のライセンス問題）
  - コンテナビルド時間（初回のみ）
  - ファイル I/O のオーバーヘッド（named volume で軽減可能）

### 選択肢 2: Nix / devbox

宣言的なパッケージマネージャーによる環境構築。`flake.nix` または `devbox.json` で開発環境を定義する。

評価:
- 利点:
  - コンテナ不要で軽量
  - 宣言的な環境定義
  - macOS / Linux でネイティブ動作
- 欠点:
  - Nix の学習コストが高い
  - Windows サポートが弱い（WSL 必須）
  - IDE との統合が限定的

### 選択肢 3: 手動セットアップのみ（現状維持）

手順書に従って各ツールを個別にインストールする。

評価:
- 利点:
  - 追加の依存関係なし
  - 環境を完全に理解できる
- 欠点:
  - セットアップに時間がかかる
  - 環境差異が発生しうる
  - 新規参画者の障壁

### 比較表

| 観点 | Dev Containers | Nix / devbox | 手動セットアップ |
|------|---------------|--------------|----------------|
| セットアップ時間 | ◎ 5分（初回ビルド後） | ○ 10分 | △ 30分〜1時間 |
| 環境の再現性 | ◎ 完全 | ◎ 完全 | △ 差異が発生しうる |
| 学習コスト | ○ 低い | △ 高い | ◎ なし |
| Windows サポート | ◎ 良好 | △ WSL 必須 | ○ ツール依存 |
| IDE 統合 | ◎ VS Code / JetBrains | △ 限定的 | ◎ ネイティブ |

## 決定

**選択肢 1: Dev Containers を採用する。**

理由:

1. **最小の学習コストで最大の効果**: Docker さえあれば、既存の開発者も新規参画者も同じ手順で開発環境を構築できる
2. **VS Code / JetBrains 両対応**: プロジェクトでは両 IDE を使用する開発者がいるため、両方をサポートできることが重要
3. **GitHub Codespaces 互換**: 将来的にブラウザベースの開発環境を提供できる

ただし、Dev Containers は**オプション**として提供し、従来の手動セットアップも引き続きサポートする。

### docker-compose の構成

PostgreSQL / Redis の定義は `infra/docker/docker-compose.yml` に一元化し、Dev Containers 用の `.devcontainer/docker-compose.yml` は `include` で参照する:

```yaml
# .devcontainer/docker-compose.yml
include:
  - path: ../infra/docker/docker-compose.yml

services:
  app:
    # Dev Container 固有の設定のみ
```

この設計により:
- PostgreSQL / Redis の設定変更は 1 箇所で済む（DRY）
- ローカル開発（`just dev-deps`）と Dev Containers で同じ定義を使用
- 設定の乖離によるバグを防止

## 帰結

### 肯定的な影響

- 新規参画者が 5 分で開発環境を構築可能（初回コンテナビルド後）
- 全開発者が同一の環境を使用し、環境差異による問題を排除
- VS Code / JetBrains の両方で利用可能

### 否定的な影響・トレードオフ

- Docker Desktop のライセンス（企業利用時は要確認）
- コンテナのビルド・起動にリソースを消費
- ファイル I/O のオーバーヘッド（named volume で軽減済み）

### 関連ドキュメント

- 実装: [`.devcontainer/`](../../.devcontainer/)
- 手順書: [`docs/04_手順書/01_開発参画/01_開発環境構築.md`](../04_手順書/01_開発参画/01_開発環境構築.md)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-18 | 初版作成 |
