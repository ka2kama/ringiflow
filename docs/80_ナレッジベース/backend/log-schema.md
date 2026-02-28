# ログスキーマ

## 概要

RingiFlow の構造化ログにおけるフィールド命名規約とスキーマ定義。AI エージェントが `jq` で効率的にログを検索・フィルタできるよう、一貫したフィールド命名を提供する。

## 前提

- JSON ログ出力: `LOG_FORMAT=json` で有効化（[observability.rs](../../../backend/crates/shared/src/observability.rs)）
- `flatten_event(true)`: スパンフィールドとイベントフィールドがフラットに出力される
- Request ID: スパンフィールド `request_id` として自動注入（[#650](https://github.com/ka2kama/ringiflow-2/issues/650)）
- PII マスキング: `REDACTED` 定数で機密情報を置換（[#651](https://github.com/ka2kama/ringiflow-2/issues/651)）

## 自動注入フィールド

tracing-subscriber が自動的に付与するフィールド。アプリケーションコードで指定する必要はない。

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `timestamp` | string | ISO 8601 タイムスタンプ |
| `level` | string | ログレベル（INFO, WARN, ERROR） |
| `target` | string | Rust モジュールパス |
| `request_id` | string | UUID v7 ベースの Request ID（スパンフィールド） |
| `tenant_id` | string | テナント ID（`X-Tenant-ID` ヘッダーから取得、不在時 `"-"`） |
| `user_id` | string | ユーザー ID（BFF: 認証成功後に `record_user_id` で記録） |
| `span.service` | string | サービス名（bff, core-service, auth-service） |

## Canonical Log Line フィールド

リクエスト完了時に `CanonicalLogLineLayer`（[canonical_log.rs](../../../backend/crates/shared/src/canonical_log.rs)）が出力するサマリログのフィールド。`log.type = "canonical"` マーカーで識別する。

スパンフィールド（`request_id`, `tenant_id`, `user_id` 等）は `with_current_span(true)` により自動的にフラット化される。

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `log.type` | string | 常に `"canonical"`（Canonical Log Line の識別マーカー） |
| `http.status_code` | u16 | HTTP レスポンスステータスコード |
| `http.latency_ms` | u64 | リクエスト処理時間（ミリ秒） |
| `error.message` | string | Service エラー時のみ出力（ERROR レベル） |

ヘルスチェックパス（`/health`, `/health/ready`）は出力対象外。

JSON 出力例（正常系）:

```json
{
  "timestamp": "2026-02-27T12:34:56.789Z",
  "level": "INFO",
  "target": "ringiflow_shared::canonical_log",
  "message": "リクエスト完了",
  "span": { "name": "request", "service": "bff" },
  "request_id": "019501a0-1234-7abc-8000-000000000001",
  "method": "POST",
  "uri": "/api/v1/workflows",
  "tenant_id": "019501a0-0000-7000-8000-000000000001",
  "user_id": "019501a0-9abc-7012-8000-000000000003",
  "log.type": "canonical",
  "http.status_code": 201,
  "http.latency_ms": 45
}
```

## ビジネスイベントフィールド

`log_business_event!` マクロで出力する。`event.kind = "business_event"` が自動付与される。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| `event.kind` | string | 自動 | 常に `"business_event"`（マクロが付与） |
| `event.category` | string | 必須 | イベントカテゴリ（`"workflow"`, `"auth"`） |
| `event.action` | string | 必須 | アクション名（下表参照） |
| `event.tenant_id` | string | 必須 | テナント ID |
| `event.result` | string | 必須 | `"success"` または `"failure"` |
| `event.entity_type` | string | 推奨 | エンティティ種別（`"workflow_instance"`, `"workflow_step"`, `"user"`, `"session"`） |
| `event.entity_id` | string | 推奨 | エンティティ ID |
| `event.actor_id` | string | 推奨 | 操作者の User ID |
| `event.reason` | string | 任意 | 失敗理由（`"password_mismatch"`, `"user_not_found"`） |

### アクション一覧

| カテゴリ | アクション | 説明 |
|---------|----------|------|
| workflow | `workflow.created` | ワークフロー作成 |
| workflow | `workflow.submitted` | ワークフロー申請 |
| workflow | `step.approved` | ステップ承認 |
| workflow | `step.rejected` | ステップ却下 |
| workflow | `step.changes_requested` | ステップ差し戻し |
| workflow | `workflow.resubmitted` | ワークフロー再申請 |
| auth | `auth.login_success` | ログイン成功 |
| auth | `auth.login_failure` | ログイン失敗 |
| auth | `auth.logout` | ログアウト |

## エラーコンテキストフィールド

既存の `tracing::error!` に追加する構造化フィールド。

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `error.category` | string | エラーカテゴリ（`"infrastructure"`, `"external_service"`） |
| `error.kind` | string | エラー種別（`"database"`, `"session"`, `"internal"` 等） |

### エラーカテゴリと種別

| カテゴリ | 種別 | 説明 |
|---------|------|------|
| `infrastructure` | `database` | DB 接続・クエリエラー |
| `infrastructure` | `session` | セッションストア（Redis）エラー |
| `infrastructure` | `internal` | 内部ロジックエラー |
| `infrastructure` | `csrf_token` | CSRF トークン操作エラー |
| `external_service` | `user_lookup` | Core Service ユーザー検索エラー |
| `external_service` | `password_verification` | Auth Service パスワード検証エラー |
| `external_service` | `service_communication` | サービス間通信エラー（汎用） |

## jq クエリ例

```bash
# Canonical Log Line（リクエスト完了サマリ）
jq 'select(.["log.type"] == "canonical")'

# 遅いリクエスト（100ms 以上）
jq 'select(.["log.type"] == "canonical" and .["http.latency_ms"] >= 100)'

# エラーレスポンス（4xx/5xx）のサマリ
jq 'select(.["log.type"] == "canonical" and .["http.status_code"] >= 400)'

# 全ビジネスイベント
jq 'select(.["event.kind"] == "business_event")'

# ワークフロー関連のイベント
jq 'select(.["event.category"] == "workflow")'

# 特定テナントのログイン失敗
jq 'select(.["event.action"] == "auth.login_failure" and .["event.tenant_id"] == "テナントID")'

# 特定ワークフローの操作履歴
jq 'select(.["event.entity_id"] == "ワークフローID")'

# DB エラー
jq 'select(.["error.category"] == "infrastructure" and .["error.kind"] == "database")'

# 外部サービスエラー
jq 'select(.["error.category"] == "external_service")'

# 特定 Request ID のログを追跡
jq 'select(.request_id == "リクエストID")'
```

## メッセージ書き方ガイドライン

構造化ログにおける `message` フィールドの設計規約。判断基準（いつ・何をログすべきか）は [Observability 設計書 > ログポリシー](../../40_詳細設計書/14_Observability設計.md#ログポリシー) を参照。

### 基本原則

| 原則 | 説明 |
|------|------|
| 定数的な文字列 | 動的な値（ID、数値）をメッセージに埋め込まない。構造化フィールドに分離する |
| 完了状態の表現 | 「何が起きたか」を表現する（例: 「ワークフロー申請完了」） |
| 日本語で記述 | プロジェクトの言語方針に従う。英語が自然な技術用語はそのまま使用可 |
| 一意に識別可能 | 同じメッセージ文字列で異なるイベントを表さない |

### 良い例 / 悪い例

```rust
// 良い: 動的値は構造化フィールドに分離
tracing::info!(
    event.entity_id = %workflow_id,
    "ワークフロー申請完了"
);

// 悪い: 動的値をメッセージに埋め込み
tracing::info!("ワークフロー {} の申請が完了しました", workflow_id);
```

```rust
// 良い: エラーコンテキストを構造化フィールドで表現
tracing::error!(
    error.category = "infrastructure",
    error.kind = "database",
    "ユーザー検索で内部エラー: {}",
    e
);

// 悪い: エラー情報がメッセージのみ
tracing::error!("DB error: {}", e);
```

注: エラーメッセージの `{}` はエラー原因の表示であり、動的な値の埋め込みとは異なる。エラー原因はメッセージに含めてよい。

### 命名規約

| 用途 | パターン | 例 |
|------|---------|-----|
| 操作完了 | `<操作対象><動作>完了` | 「ワークフロー申請完了」「承認ステップ完了」 |
| エラー | `<操作>で<エラー種別>: {}` | 「ユーザー検索で内部エラー: {}」 |
| 初期化 | `<コンポーネント>を初期化しました` | 「S3 クライアントを初期化しました」 |
| サーバー起動 | `<サービス名>サーバーを起動/起動しました` | 「Core Service サーバーが起動しました」 |
| 障害検知 | `<チェック対象>: <障害内容>` | 「readiness check: database connection failed」 |
| 非致命的失敗 | `<操作>に失敗（無視）: {}` | 「CSRF トークン削除に失敗（無視）: {}」 |

## プロジェクトでの使用箇所

- Canonical Log Line: [`backend/crates/shared/src/canonical_log.rs`](../../../backend/crates/shared/src/canonical_log.rs)
- マクロ定義: [`backend/crates/shared/src/event_log.rs`](../../../backend/crates/shared/src/event_log.rs)
- ワークフローイベント: `backend/apps/core-service/src/usecase/workflow/command/` 配下
- 認証イベント: `backend/apps/bff/src/handler/auth/login.rs`
- エラーコンテキスト: 各サービスの `error.rs` + BFF auth ハンドラ

## 関連リソース

- [observability.rs](../../../backend/crates/shared/src/observability.rs) — トレーシング初期化
- [運用設計書](../../30_基本設計書/04_運用設計.md) — 監査ログ要件
- [Elastic Common Schema (ECS)](https://www.elastic.co/guide/en/ecs/current/index.html) — 命名規約の参考
