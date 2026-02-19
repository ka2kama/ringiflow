# Observability CI チェック（#696）

## 概要

Epic #648 の Observability 基盤で導入した `#[tracing::instrument]` パターンを構造的に担保するため、AI エージェント用ルールファイル、CI チェックスクリプト、CI ワークフロー統合の 3 層を実装した。

## 実施内容

### Phase 1: `.claude/rules/observability.md` の作成

AI エージェントがハンドラ・リポジトリ・クライアント・セッション関連ファイルを編集する際に自動適用されるルールファイルを作成した。`paths:` frontmatter でファイルパターンを指定し、計装パターンテーブル、PII ルール（`skip_all` デフォルト）、除外対象、属性配置順を定義。

### Phase 2: `scripts/check/instrumentation.sh` の作成

ハンドラとリポジトリ impl の計装漏れを検出する CI チェックスクリプトを作成した。既存の `impl-docs.sh` パターン（ERRORS 配列、exit 1 でブロック）を踏襲。

チェック対象:
- ハンドラ: `pub async fn` に `#[tracing::instrument]` があるか（`health_check` を除外）
- リポジトリ: `async fn`（impl メソッドのみ、trait 署名を除外）に `#[tracing::instrument]` があるか

### Phase 3: justfile + parallel.sh + CI 統合

`check-instrumentation` レシピを justfile に追加し、parallel.sh の Non-Rust レーンと ci.yaml の code-quality ジョブに統合した。

## 判断ログ

- trait 署名と impl メソッドの判別方式: `async fn` 行から前方スキャン（最大 20 行）で `;`（trait 署名）か `{`（impl メソッド）を検出する方式を採用した。正規表現のみの判別より正確
- BFF HTTP クライアントのスコープ: ルールファイルの `paths:` には含め AI ガードとするが、CI スクリプトの対象からは除外した。Issue #696 のスコープがハンドラ + リポジトリであり、クライアントは既に全て計装済みのため
- lookback 距離: `#[tracing::instrument]` の検出を `async fn` 行から上方 10 行とした。`#[utoipa::path(...)]` 等の属性が間に入るケースを考慮

## 成果物

### コミット

- `de2bb4b` #696 Add observability rule file for AI agent guidance
- `3029431` #696 Add instrumentation check script for handlers and repositories
- `1e283e7` #696 Integrate instrumentation check into justfile, parallel.sh, and CI
- `9c792a4` #696 Add implementation plan for observability CI check

### 作成ファイル

- `.claude/rules/observability.md` — AI エージェント用計装ルールファイル
- `scripts/check/instrumentation.sh` — 計装チェックスクリプト
- `prompts/plans/696_observability-ci-check.md` — 計画ファイル

### 更新ファイル

- `justfile` — `check-instrumentation` レシピ追加
- `scripts/check/parallel.sh` — Non-Rust レーンに追加
- `.github/workflows/ci.yaml` — code-quality ジョブに step 追加
