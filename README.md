# RingiFlow (稟議フロー)

[![CI](https://github.com/ka2kama/ringiflow/actions/workflows/ci.yaml/badge.svg)](https://github.com/ka2kama/ringiflow/actions/workflows/ci.yaml)
![Rust](https://img.shields.io/badge/Rust-1.92-orange?logo=rust)
![Elm](https://img.shields.io/badge/Elm-0.19-60B5CC?logo=elm)
![License](https://img.shields.io/badge/License-CC0--1.0-blue)

承認フロー・タスク管理・ドキュメント管理を一元化する **エンタープライズ向けワークフロー管理システム（SaaS）**

> **学習 & 実験プロジェクト**: 商用レベルの品質を目指しながら、AI エージェント（Claude Code）主導で開発する実験。

---

## プロジェクト理念

このプロジェクトは 2 つの理念を軸に進めている。

### 学習効果の最大化

単にコードを書くだけでなく、**設計判断の理由**を常に言語化する。

- なぜその技術・パターンを選んだか
- 他にどんな選択肢があり、なぜ採用しなかったか
- トレードオフは何か

**重点学習テーマ:**
- CQRS + Event Sourcing
- 並行更新と状態整合性（楽観的ロック、競合解決、UI同期）
- マルチテナントアーキテクチャ

これらを ADR や技術ノートに記録し、後から振り返れるようにしている。

### 品質の追求

外部品質（信頼性、パフォーマンス、セキュリティ）と内部品質（可読性、保守性、テスタビリティ）の両方を追求する。

**設計原則:**
- シンプルさを保つ（KISS）
- 責務を明確に分離する
- 変更の影響範囲を局所化する

**型システムの活用:**
- 不正な状態を型で表現不可能にする
- 実行時エラーよりコンパイルエラーを選ぶ

### 共通アプローチ: ベストプラクティス起点

2つの理念を実現するため、あらゆる判断において業界ベストプラクティスを起点とし、プロジェクトの現実に合わせて調整する。

- 起点を高く置く（ベストプラクティスから始めて調整する）
- 全領域に適用（コード設計、UI/UX、セキュリティ、テスト——例外なし）
- 意識的な調整（外れるときは理由を記録する）

---

## 技術スタック

| レイヤー | 技術 | 選定理由 |
|---------|------|----------|
| バックエンド | **Rust** + axum | 型安全性、メモリ安全性、高パフォーマンス |
| フロントエンド | **Elm** | 純粋関数型、ランタイムエラーゼロ、The Elm Architecture |
| データストア | PostgreSQL, Redis | ワークフロー・ユーザー管理、セッション管理 |
| インフラ | AWS Lightsail, Cloudflare | デモ環境（個人開発向け低コスト構成） |

## アーキテクチャ

```mermaid
flowchart LR
    subgraph Client
        Browser["Browser\n(Elm SPA)"]
    end

    subgraph Backend
        BFF["BFF\n(Rust/axum)"]
        Core["Core Service\n(Rust/axum)"]
        Auth["Auth Service\n(Rust/axum)"]
    end

    subgraph Data
        PG["PostgreSQL"]
        Redis["Redis\n(Session)"]
    end

    Browser --> BFF
    BFF --> Core
    BFF --> Auth
    BFF --> Redis
    Core --> PG
    Auth --> PG
```

### 設計パターン

| パターン | 目的 |
|---------|------|
| **BFF (Backend for Frontend)** | セキュリティ強化（トークン秘匿）、フロントエンド最適化 API |
| **マルチテナント (tenant_id)** | アプリケーションレベルのテナントデータ分離 |
| **レイヤードアーキテクチャ** | domain / infra / apps の責務分離 |

## 技術的ハイライト

### ドキュメント体系

すべての知識を文書化し、**暗黙知ゼロ**を目指す。

| 知りたいこと | 参照先 |
|-------------|--------|
| 何を作るか（WHAT） | [要件定義書](docs/01_要件定義書/) |
| どう作るか（HOW） | [基本設計書](docs/02_基本設計書/) / [詳細設計書](docs/03_詳細設計書/) |
| どう操作するか（HOW TO） | [手順書](docs/04_手順書/) |
| なぜその決定か（WHY） | [ADR](docs/05_ADR/)（例: [ID 形式](docs/05_ADR/001_ID形式の選定.md) / [データ削除](docs/05_ADR/007_テナント退会時のデータ削除方針.md) / [Newtype 化](docs/05_ADR/016_プリミティブ型のNewtype化方針.md)） |
| 技術の知識 | [ナレッジベース](docs/06_ナレッジベース/) |
| 実装の詳細解説 | [実装解説](docs/07_実装解説/)（例: [認証機能](docs/07_実装解説/01_認証機能/00_概要.md)） |
| 開発の過程 | [セッションログ](prompts/runs/) |

### CI/CD & コード品質

- **GitHub Actions**: 変更検出による効率的な並列 CI
- **Claude Code Action**: AI による自動 PR レビュー
- **リント**: clippy (Rust), elm-review (Elm)
- **フォーマット**: rustfmt, elm-format

### 開発環境

- **並行開発対応**: git worktree + Docker Compose で、複数タスクを独立した環境で同時進行可能
  - **ポート自動割り当て**: 環境ごとに衝突しないポートを自動設定

## ディレクトリ構成

```
ringiflow/
├── backend/           # Rust バックエンド
│   ├── apps/          # BFF, Core Service, Auth Service
│   └── crates/        # 共有ライブラリ（domain, infra, shared）
├── frontend/          # Elm フロントエンド
├── infra/             # Terraform, Docker
├── openapi/           # OpenAPI 仕様
└── docs/              # ドキュメント
    ├── 01_要件定義書/
    ├── 02_基本設計書/
    ├── 03_詳細設計書/
    ├── 04_手順書/
    ├── 05_ADR/
    ├── 06_ナレッジベース/
    └── 07_実装解説/
```

## 開発フロー

### Issue 駆動開発

GitHub Projects + Issue でタスクを管理。

1. Issue を作成または確認
2. `feature/123-機能名` 形式でブランチ作成
3. 実装 → PR 作成（`Closes #123` で紐付け）
4. CI + AI レビュー → マージ

→ [Project Board](https://github.com/users/ka2kama/projects/1) / [Issues](https://github.com/ka2kama/ringiflow/issues)

### AI 主導の開発

このプロジェクトは AI エージェント（Claude Code）が主導で開発している。

- **オーナー**: 指示・方針決定・レビュー
- **Claude Code**: 実装・設計判断・ドキュメント作成
- **Claude Code Action**: PR の自動レビュー

AI エージェント向けのガイドラインは [CLAUDE.md](CLAUDE.md) を参照。

## Getting Started

開発環境の構築手順: [手順書](docs/04_手順書/01_開発参画/01_開発環境構築.md)

複数タスクを同時に進める場合: [並行開発（Worktree）](docs/04_手順書/04_開発フロー/04_並行開発（Worktree）.md)

```bash
# 初回セットアップ（依存関係インストール、DB 起動、マイグレーション）
just setup

# 開発サーバー起動（BFF, Core Service, Auth Service, Web を一括起動）
just dev-all

# コミット前チェック（lint + test + API test）
just check-all
```

## 開発状況

**Phase 2（機能拡張）計画中** — Phase 1 MVP 完了

| Phase | 状態 | 内容 |
|-------|------|------|
| Phase 0 | ✅ 完了 | 開発基盤構築（CI/CD、プロジェクト構造、ドキュメント体系） |
| Phase 1 | ✅ 完了 | 最小限の動作するワークフローシステム |
| Phase 2 | 📋 計画中 | 機能拡張（マルチテナント、通知、ドキュメント管理） |
| Phase 3 | 📋 計画中 | エンタープライズ機能（SSO/MFA、複雑なフロー） |
| Phase 4 | 📋 計画中 | 高度な機能（CQRS/ES、リアルタイム） |

### Phase 1 の進捗

- [x] 認証（メール/パスワードログイン、ログアウト）
- [x] セッション管理（HTTPOnly Cookie、Redis）
- [x] ワークフロー申請・承認
- [x] タスク一覧・詳細
- [x] ダッシュボード

詳細: [実装ロードマップ](docs/03_詳細設計書/00_実装ロードマップ.md)
