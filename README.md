# RingiFlow

エンタープライズ向けワークフロー管理システム（SaaS）

承認フロー・タスク管理・ドキュメント管理を一元化し、企業の業務効率化を支援する。

## 技術スタック

| レイヤー | 技術 |
|---------|------|
| バックエンド | Rust + axum |
| フロントエンド | Elm |
| インフラ | AWS（ECS Fargate, Aurora PostgreSQL, ElastiCache Redis） |
| IaC | Terraform |

## はじめに

初めての方は [docs/01_要件定義書/00_はじめに.md](docs/01_要件定義書/00_はじめに.md) から読み始めてください。

開発環境の構築は [docs/04_手順書/01_開発参画/01_開発環境構築.md](docs/04_手順書/01_開発参画/01_開発環境構築.md) を参照。

## 開発

```bash
# 利用可能なコマンドを表示
just --list

# ローカル開発環境の起動（PostgreSQL, Redis）
docker compose -f infra/docker/docker-compose.yml up -d
```

## 現在のステータス

Phase 1（MVP 開発中）

- **Phase 0**: ✅ 完了（開発基盤構築）
- **Phase 1**: 🚧 開発中（最小限の動作するワークフローシステム）

詳細は [docs/03_詳細設計書/00_実装ロードマップ.md](docs/03_詳細設計書/00_実装ロードマップ.md) を参照。

## ライセンス

このプロジェクトは学習・実験目的で開発されている。
