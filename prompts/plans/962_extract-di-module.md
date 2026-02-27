# #962 main.rs の DI 定義をモジュール分離する

## 概要

core-service と bff の `main.rs` から DI（リポジトリ・UseCase・State）初期化とルーター構築を `app_builder.rs` に分離する。

## 対象

- `backend/apps/core-service/src/main.rs`（506 行 → 目標 ~110 行）
- `backend/apps/bff/src/main.rs`（519 行 → 目標 ~140 行）

## 対象外

- `backend/apps/auth-service/src/main.rs`（161 行、閾値以下）

## 設計判断

### モジュール名: `app_builder`

`config` と同じく `main.rs` のプライベートモジュール（`mod app_builder;`）として配置する。`lib.rs` には追加しない（アプリケーション構築はバイナリエントリーポイント固有の責務）。

### 関数シグネチャ

core-service:
```rust
pub(crate) fn build_app(
    pool: PgPool,
    s3_client: Arc<dyn S3Client>,
    config: &CoreConfig,
) -> Router
```

bff:
```rust
pub(crate) fn build_app(
    config: &BffConfig,
    session_manager: Arc<dyn SessionManager>,
    readiness_state: Arc<ReadinessState>,
    dynamodb_client: aws_sdk_dynamodb::Client,
) -> Router
```

### 分離の境界

| main.rs に残すもの | app_builder に移すもの |
|-------------------|---------------------|
| .env 読み込み | リポジトリ初期化 |
| トレーシング初期化 | UseCase 初期化 |
| Config 読み込み | State 初期化 |
| DB 接続プール作成 + マイグレーション | 通知サービス初期化（core） |
| S3 クライアント初期化 | Authz State 初期化（bff） |
| Redis 接続（bff） | ルーター構築 |
| DynamoDB 初期化（bff） | |
| DevAuth セットアップ（bff） | |
| サーバー起動 | |

原則: async インフラ初期化 → main.rs、sync アプリケーション組み立て → app_builder

## Phase 1: core-service の分離

#### 確認事項
- 型: `PgPool` → `sqlx::PgPool`（既存 main.rs の import で確認済み）
- 型: `CoreConfig` → `backend/apps/core-service/src/config.rs`
- パターン: `config` モジュールの配置（`main.rs` の `mod config;`）

#### 操作パス: 該当なし（リファクタリング、外部動作の変更なし）

#### テストリスト

ユニットテスト（該当なし — 外部動作の変更なし、コンパイル + 既存テストで検証）
ハンドラテスト（該当なし）
API テスト（該当なし — 既存テストがリグレッション検証を担う）
E2E テスト（該当なし）

検証方法: `just check-all` で既存テスト全通過

## Phase 2: bff の分離

#### 確認事項
- 型: `BffConfig` → `backend/apps/bff/src/config.rs`
- 型: `ReadinessState` → `backend/apps/bff/src/handler/health.rs`
- パターン: Phase 1 の core-service 分離パターンを踏襲

#### 操作パス: 該当なし（リファクタリング、外部動作の変更なし）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし — 既存テストがリグレッション検証を担う）
E2E テスト（該当なし）

検証方法: `just check-all` で既存テスト全通過

## Phase 3: ベースライン更新

#### 確認事項: なし（既知のパターンのみ）

#### 操作パス: 該当なし

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手順:
1. `just check-file-size` で改善を確認
2. `.config/baselines.env` の `FILE_SIZE_MAX_COUNT` を更新
3. main.rs のファイルサイズ例外コメントを削除

### ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | bff の `build_app` で CoreServiceClient / AuthServiceClient の生成を内部化すべきか外部パラメータにすべきか | シンプルさ | クライアント生成は sync かつ config のみ依存。app_builder 内部で生成する（パラメータ数を削減） |
| 2回目 | core-service の通知サービス初期化（SmtpNotificationSender::new）は sync だが config 参照が必要 | アーキテクチャ不整合 | config を build_app のパラメータとして渡すため問題なし。通知サービス初期化は app_builder 内で実行 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | core-service + bff の 2 ファイルが対象。auth-service は閾値以下で対象外を明示 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 関数シグネチャ、分離境界が具体的に定義済み |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | モジュール配置（private mod）、パラメータ設計の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | auth-service が対象外として明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | async/sync の境界、axum Router の State 型消去が確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | structural-review.md のルーター例外許容と整合 |
