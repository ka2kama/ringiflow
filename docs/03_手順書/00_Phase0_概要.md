# Phase 0 実装準備 概要

## 目的

本ドキュメントは、RingiFlow の実装を開始する前に完了すべき準備作業（Phase 0）の全体像を定義する。
Phase 0 完了後、Phase 1（MVP 実装）に進むことができる。

## 前提条件

Phase 0 を開始する前に、以下が完了していること:

- [`01_開発環境構築.md`](01_開発環境構築.md) に従い、必須ツールがインストール済み
- Git リポジトリがクローン済み
- GitHub アカウントでリポジトリへのプッシュ権限がある

### 必須ツールのバージョン確認

以下のコマンドを実行し、すべてが成功することを確認する:

```bash
# Rust
rustc --version   # 出力例: rustc 1.92.0 (xxx)
cargo --version   # 出力例: cargo 1.92.0 (xxx)

# Rust コンポーネント
rustfmt --version # 出力例: rustfmt 1.9.0-stable (xxx)
cargo clippy --version # 出力例: clippy 0.1.92 (xxx)

# Node.js / pnpm
node --version    # 出力例: v22.x.x
pnpm --version    # 出力例: 10.x.x

# Elm
elm --version     # 出力例: 0.19.1
elm-format --help | head -1  # 出力例: elm-format 0.8.7

# Docker
docker --version           # 出力例: Docker version 27.x.x
docker compose version     # 出力例: Docker Compose version v2.x.x

# just
just --version    # 出力例: just 1.x.x

# SQLx CLI
cargo sqlx --version  # 出力例: sqlx-cli x.x.x
```

いずれかが失敗する場合、[`01_開発環境構築.md`](01_開発環境構築.md) を再確認する。

---

## Phase 0 作業一覧

| 順序 | 作業 | 手順書 | 備考 |
|:----:|------|--------|------|
| 1 | プロジェクトセットアップ | [`02_プロジェクトセットアップ.md`](02_プロジェクトセットアップ.md) | `just setup` で自動化 |
| 2 | CI/CD 確認 | [`03_CICD構築.md`](03_CICD構築.md) | 構築済み、変更時のみ参照 |

**注記**:
- Terraform 基盤構築（[`04_Terraform基盤構築.md`](04_Terraform基盤構築.md)）は Phase 1 で AWS デプロイが必要になった時点で実施する
- GitHub 設定（[`05_GitHub設定.md`](05_GitHub設定.md)）は必要に応じて参照する

---

## Phase 0 完了条件

以下のすべてを満たした時点で Phase 0 完了とする:

### 1. セットアップ成功

```bash
just setup
# 終了コード 0（エラーなし）
```

### 2. ヘルスチェックエンドポイント応答

```bash
# BFF を起動した状態で
just dev-api &
sleep 3
curl -s http://localhost:3000/health | jq .
# 出力例:
# {
#   "status": "healthy",
#   "version": "0.1.0"
# }
```

### 3. GitHub Actions CI 成功

- `main` ブランチへのプッシュ後、GitHub Actions の CI ワークフローが緑（成功）になること
- 確認 URL: `https://github.com/<org>/ringiflow/actions`

---

## トラブルシューティング

各手順書に個別のトラブルシューティングセクションがある。
共通的な問題は以下:

### 権限エラー

```bash
# Docker ソケットへのアクセス権限がない場合
sudo usermod -aG docker $USER
# 再ログインが必要
```

### ポート競合

```bash
# ポート 5432 が使用中の場合
lsof -i :5432
# 競合するプロセスを停止するか、docker-compose.yml のポートを変更
```

### Cargo のビルドが遅い

```bash
# sccache を使用
cargo install sccache
echo 'export RUSTC_WRAPPER=sccache' >> ~/.bashrc
source ~/.bashrc
```

---

## 次のステップ

Phase 0 完了後:

1. [`docs/02_設計書/00_実装ロードマップ.md`](../02_設計書/00_実装ロードマップ.md) の Phase 1 を参照
2. Phase 1 MVP 実装を開始

---

## 変更履歴

| 日付 | 変更内容 | 担当 |
|------|---------|------|
| 2026-01-14 | Terraform 基盤構築を Phase 1 に延期 | - |
| 2026-01-13 | 初版作成 | - |
