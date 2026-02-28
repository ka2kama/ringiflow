# 実装計画: #875 通知基盤を構築する

## Context

Epic #406 (Phase 2-5) の最初の実装 Story。ワークフロー操作（承認・却下・差し戻し等）に伴うメール通知の基盤を構築する。後続 Story (#876-#879) がこの基盤の上にユースケース統合・ログ記録・SES 本番設定を積み上げる。

設計ドキュメント:
- 詳細設計: `docs/03_詳細設計書/16_通知機能設計.md`
- 機能仕様書: `docs/01_要件定義書/機能仕様書/05_通知機能.md`

## スコープ

**対象:**
- `NotificationSender` trait + SMTP / SES / Noop の 3 実装
- `NotificationError`, `EmailMessage`, `WorkflowNotification` 型定義
- `TemplateRenderer` (tera) + 5 種類の HTML/plaintext メールテンプレート
- `NotificationService` (render + send + log 統合)
- `NotificationLogRepository` trait + PostgreSQL 実装 + migration
- Mailpit Docker Compose (dev + api-test)
- 環境変数生成 (`generate.sh`) + `NotificationConfig`
- main.rs DI 配線
- event_log.rs への notification 定数追加
- Mock 実装 (`MockNotificationSender`, `MockNotificationLogRepository`)

**対象外（後続 Story）:**
- ワークフローユースケースからの `NotificationService.notify()` 呼び出し (#876, #877)
- テナント退会 deletion registry 登録 (#878)
- SES 本番設定・Terraform (#879)

## 設計判断

### 判断 1: NotificationSender trait の配置先

infra クレートの `notification` モジュールに配置する。

理由:
- 既存の repository traits はすべて infra クレートに定義されている
- `async fn` を使うため `async-trait` が必要 — domain クレートは `async-trait` に非依存
- `Clock` は同期 trait でドメイン固有のため domain だが、`NotificationSender` は外部サービス通信でありインフラの関心事

`EmailMessage`, `WorkflowNotification`, `NotificationError` は domain クレートに配置（ビジネスロジック表現の型、infra 非依存）。

### 判断 2: テンプレートファイルの配置

`backend/apps/core-service/templates/notifications/` に配置。`include_str!` でコンパイル時に埋め込むため、`TemplateRenderer` と同じクレート内に必要。

### 判断 3: NotificationLogRepository のスコープ

`NotificationService.notify()` が送信結果をログに記録するため、本 Story に含める。migration + trait + PostgreSQL 実装 + mock を含む。

### 判断 4: lettre vs 生の SMTP

lettre 0.11 を使用。SMTP プロトコル実装を自作する理由がない。`AsyncSmtpTransport<Tokio1Executor>` で Mailpit に接続する。

---

## Phase 1: ドメイン型の定義

domain クレートに通知関連の型を定義する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/crates/domain/src/notification.rs` | 新規 |
| `backend/crates/domain/src/lib.rs` | `pub mod notification;` 追加 |

### notification.rs の内容

```rust
// NotificationLogId — define_uuid_id! マクロ
// NotificationError — SendFailed, TemplateFailed, LogFailed
// NotificationEventType — enum (strum Display/EnumString, snake_case)
// EmailMessage — to, subject, html_body, text_body
// WorkflowNotification — 5 バリアント (設計書準拠)
//   + event_type(), recipient_email(), recipient_user_id(),
//     workflow_title(), workflow_display_id() メソッド
```

### 確認事項
- 型: `UserId` → `backend/crates/domain/src/user.rs`（WorkflowNotification のフィールド型）
- パターン: `DomainError` → `backend/crates/domain/src/error.rs`（エラー型定義パターン）
- パターン: `define_uuid_id!` → `backend/crates/domain/src/macros.rs`（NotificationLogId 定義用）
- パターン: `strum` → 既存の enum 使用パターン（EnumString, Display）

### 操作パス
該当なし（ドメインロジックのみ）

### テストリスト

ユニットテスト:
- [ ] `NotificationEventType` の Display/FromStr が snake_case で正しく動作する
- [ ] `WorkflowNotification::event_type()` が各バリアントで正しい `NotificationEventType` を返す
- [ ] `WorkflowNotification::recipient_email()` が各バリアントで正しい値を返す
- [ ] `WorkflowNotification::recipient_user_id()` が各バリアントで正しい値を返す

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 2: NotificationSender trait + 3 実装

infra クレートに `NotificationSender` trait と SMTP / SES / Noop の 3 実装を作成する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/Cargo.toml` | workspace deps に `tera`, `lettre`, `aws-sdk-sesv2` 追加 |
| `backend/crates/infra/Cargo.toml` | `lettre`, `aws-sdk-sesv2` 追加 |
| `backend/crates/infra/src/notification.rs` | 新規（モジュールルート） |
| `backend/crates/infra/src/notification/smtp.rs` | 新規 |
| `backend/crates/infra/src/notification/ses.rs` | 新規 |
| `backend/crates/infra/src/notification/noop.rs` | 新規 |
| `backend/crates/infra/src/lib.rs` | `pub mod notification;` 追加 |

### trait 定義

```rust
#[async_trait]
pub trait NotificationSender: Send + Sync {
    async fn send_email(&self, email: &EmailMessage) -> Result<(), NotificationError>;
}
```

### 各実装

- `SmtpNotificationSender` — `lettre::AsyncSmtpTransport<Tokio1Executor>`, `new(host, port, from_address)`
- `SesNotificationSender` — `aws_sdk_sesv2::Client`, `new(client, from_address)`
- `NoopNotificationSender` — `tracing::info!` でログ出力のみ

### 確認事項
- パターン: `#[async_trait] pub trait ... : Send + Sync` → `backend/crates/infra/src/repository/user_repository.rs`
- ライブラリ: `lettre` — Grep で既存使用を確認（未使用のはず）、docs.rs で `AsyncSmtpTransport::builder()` API 確認
- ライブラリ: `aws-sdk-sesv2` — Grep で既存使用を確認（未使用）、`aws-config` は workspace に既存
- パターン: infra モジュール構造 → `backend/crates/infra/src/lib.rs`（notification をどう公開するか）

### 操作パス
該当なし（ドメインロジックのみ）

### テストリスト

ユニットテスト:
- [ ] `SmtpNotificationSender` が Send + Sync を実装している（コンパイル時検証）
- [ ] `SesNotificationSender` が Send + Sync を実装している
- [ ] `NoopNotificationSender::send_email()` がエラーを返さない

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 3: TemplateRenderer + メールテンプレート

tera テンプレートエンジンで 5 種類の通知メールを HTML/plaintext 両形式で生成する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/apps/core-service/Cargo.toml` | `tera` 追加 |
| `backend/apps/core-service/templates/notifications/` | 新規ディレクトリ |
| → `approval_request.html` / `.txt` | 新規（5 種類 × 2 形式 = 10 ファイル） |
| → `step_approved.html` / `.txt` | 新規 |
| → `approved.html` / `.txt` | 新規 |
| → `rejected.html` / `.txt` | 新規 |
| → `changes_requested.html` / `.txt` | 新規 |
| `backend/apps/core-service/src/usecase/notification/template_renderer.rs` | 新規 |
| `backend/apps/core-service/src/usecase/notification.rs` | 新規（モジュールルート） |
| `backend/apps/core-service/src/usecase.rs` | `pub mod notification;` 追加 |

### TemplateRenderer

```rust
pub struct TemplateRenderer {
    engine: tera::Tera,
}

impl TemplateRenderer {
    pub fn new() -> Result<Self, NotificationError> { ... }
    pub fn render(
        &self,
        notification: &WorkflowNotification,
        base_url: &str,
    ) -> Result<EmailMessage, NotificationError> { ... }
}
```

- `include_str!` で 10 テンプレートをコンパイル時に埋め込む
- `tera::Context` にフィールドを設定し、HTML + plaintext を生成
- 件名パターンは設計書準拠: `[RingiFlow] 承認依頼: {title} {display_id}` 等

### テンプレート変数（共通）
`workflow_title`, `workflow_display_id`, `base_url`, `workflow_url`

### テンプレート変数（イベント固有）
- `approval_request`: `applicant_name`, `step_name`
- `step_approved`: `step_name`, `approver_name`
- `rejected` / `changes_requested`: `comment`（Option）

### 確認事項
- ライブラリ: `tera` — Grep で既存使用を確認（未使用）、docs.rs で `Tera::default()` + `add_raw_templates()` API 確認
- 型: `WorkflowNotification` のフィールド — Phase 1 で定義した型
- パターン: 件名パターン → `docs/03_詳細設計書/16_通知機能設計.md` のメール件名パターン表

### 操作パス
該当なし（ドメインロジックのみ）

### テストリスト

ユニットテスト:
- [ ] `TemplateRenderer::new()` が正常に初期化される
- [ ] `ApprovalRequest` のレンダリング: 件名が `[RingiFlow] 承認依頼: {title} {display_id}` パターンに一致
- [ ] `StepApproved` のレンダリング: 件名・HTML・plaintext が正しい
- [ ] `Approved` のレンダリング: 件名・HTML・plaintext が正しい
- [ ] `Rejected` のレンダリング: comment あり/なし両方
- [ ] `ChangesRequested` のレンダリング: comment あり/なし両方
- [ ] テンプレート変数が正しく展開される（workflow_title, base_url 等）
- [ ] HTML 本文にワークフロー詳細ページへのリンクが含まれる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 4: NotificationLogRepository + Migration + NotificationService

DB migration で `notification_logs` テーブルを作成し、NotificationLogRepository + NotificationService を実装する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/migrations/20260225000001_create_notification_logs.sql` | 新規 |
| `backend/crates/infra/src/repository/notification_log_repository.rs` | 新規 |
| `backend/crates/infra/src/repository.rs` | `pub mod notification_log_repository;` + re-export 追加 |
| `backend/crates/infra/src/mock.rs` | `MockNotificationSender` + `MockNotificationLogRepository` 追加 |
| `backend/apps/core-service/src/usecase/notification/service.rs` | 新規 |
| `backend/crates/shared/src/event_log.rs` | notification 定数追加 |

### notification_logs テーブル（設計書準拠）

```sql
CREATE TABLE notification_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    workflow_instance_id UUID NOT NULL REFERENCES workflow_instances(id) ON DELETE CASCADE,
    workflow_title VARCHAR(255) NOT NULL,
    workflow_display_id VARCHAR(50) NOT NULL,
    recipient_user_id UUID NOT NULL,
    recipient_email VARCHAR(255) NOT NULL,
    subject VARCHAR(500) NOT NULL,
    status VARCHAR(20) NOT NULL,
    error_message TEXT,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- + RLS ポリシー + インデックス
```

### NotificationLogRepository

```rust
#[async_trait]
pub trait NotificationLogRepository: Send + Sync {
    async fn insert(&self, log: &NotificationLog) -> Result<(), InfraError>;
}
```

`NotificationLog` は infra 内部の構造体（repository の insert 用データ型）。

### NotificationService

```rust
pub struct NotificationService {
    sender: Arc<dyn NotificationSender>,
    template_renderer: TemplateRenderer,
    log_repo: Arc<dyn NotificationLogRepository>,
    base_url: String,
}

impl NotificationService {
    pub async fn notify(
        &self,
        notification: WorkflowNotification,
        tenant_id: &TenantId,
        workflow_instance_id: &WorkflowInstanceId,
    ) { /* fire-and-forget: 送信失敗してもエラーを返さない */ }
}
```

### event_log.rs 追加定数

```rust
pub mod category { pub const NOTIFICATION: &str = "notification"; }
pub mod action {
    pub const NOTIFICATION_SENT: &str = "notification.sent";
    pub const NOTIFICATION_FAILED: &str = "notification.failed";
}
pub mod entity_type { pub const NOTIFICATION_LOG: &str = "notification_log"; }
```

### Mock 実装（mock.rs に追加）

- `MockNotificationSender` — 送信メッセージを `Arc<Mutex<Vec<EmailMessage>>>` に記録
- `MockNotificationLogRepository` — ログを `Arc<Mutex<Vec<NotificationLog>>>` に記録

### 確認事項
- パターン: migration ファイル命名 → `20260224000001_fix_seed_current_step_id.sql`（最新）
- パターン: repository trait + PostgreSQL 実装 → `backend/crates/infra/src/repository/user_repository.rs`
- パターン: mock repository → `backend/crates/infra/src/mock.rs`
- 型: `TenantId`, `WorkflowInstanceId`, `UserId` → domain クレート
- パターン: event_log 定数 → `backend/crates/shared/src/event_log.rs`

### 操作パス
該当なし（NotificationService はユースケースから呼ばれるが、本 Story では統合対象外）

### テストリスト

ユニットテスト:
- [ ] `NotificationService::notify()` — 送信成功時に log_repo に status="sent" で記録する
- [ ] `NotificationService::notify()` — 送信失敗時に log_repo に status="failed" + error_message で記録する
- [ ] `NotificationService::notify()` — テンプレートレンダリング失敗時にエラーログを出力するが panic しない
- [ ] `NotificationService::notify()` — 送信失敗してもエラーを返さない（fire-and-forget）
- [ ] `MockNotificationSender` が送信されたメッセージを記録する
- [ ] `MockNotificationLogRepository` が挿入されたログを保持する

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 5: Docker Compose + 環境変数 + DI 配線 + 統合テスト

Mailpit を Docker Compose に追加し、環境変数を更新し、main.rs で DI を配線し、統合テストで検証する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `infra/docker/docker-compose.yaml` | Mailpit サービス追加 |
| `infra/docker/docker-compose.api-test.yaml` | Mailpit サービス追加 |
| `scripts/env/generate.sh` | Mailpit ポート + 通知環境変数追加 |
| `backend/apps/core-service/src/config.rs` | `NotificationConfig` 追加 |
| `backend/apps/core-service/src/main.rs` | DI 配線追加 |

### Docker Compose（設計書準拠）

```yaml
mailpit:
  image: axllent/mailpit:latest
  ports:
    - "${MAILPIT_SMTP_PORT}:1025"
    - "${MAILPIT_UI_PORT}:8025"
  healthcheck:
    test: ["CMD-SHELL", "wget -qO- http://localhost:8025/api/v1/info || exit 1"]
    interval: 5s
    timeout: 5s
    retries: 5
    start_period: 5s
  restart: unless-stopped
```

### generate.sh 追加

基準ポート（開発環境）:
- `BASE_MAILPIT_SMTP_PORT=11025`
- `BASE_MAILPIT_UI_PORT=18025`

基準ポート（API テスト）:
- `BASE_API_TEST_MAILPIT_SMTP_PORT=11026`
- `BASE_API_TEST_MAILPIT_UI_PORT=18026`

ルート `.env` に `MAILPIT_SMTP_PORT`, `MAILPIT_UI_PORT`, `API_TEST_MAILPIT_SMTP_PORT`, `API_TEST_MAILPIT_UI_PORT` 追加。

`backend/.env` に追加:
```
NOTIFICATION_BACKEND=smtp
SMTP_HOST=localhost
SMTP_PORT=$MAILPIT_SMTP_PORT
NOTIFICATION_FROM_ADDRESS=noreply@ringiflow.example.com
NOTIFICATION_BASE_URL=http://localhost:$VITE_PORT
```

`backend/.env.api-test` にも同様（ポートは API テスト用）。

### NotificationConfig

```rust
pub struct NotificationConfig {
    pub backend: String,       // "smtp" | "ses" | "noop"
    pub smtp_host: String,
    pub smtp_port: u16,
    pub from_address: String,
    pub base_url: String,
}
```

### main.rs DI 配線

```rust
// NOTIFICATION_BACKEND に基づいて NotificationSender を選択
let notification_sender: Arc<dyn NotificationSender> = match config.notification.backend.as_str() {
    "smtp" => Arc::new(SmtpNotificationSender::new(...)),
    "ses" => Arc::new(SesNotificationSender::new(...)),
    _ => Arc::new(NoopNotificationSender),
};

let notification_log_repo: Arc<dyn NotificationLogRepository> =
    Arc::new(PostgresNotificationLogRepository::new(pool.clone()));
let template_renderer = TemplateRenderer::new().expect("テンプレートエンジンの初期化に失敗");
let notification_service = Arc::new(NotificationService::new(
    notification_sender, template_renderer, notification_log_repo, config.notification.base_url,
));
// → 後続 Story (#876-#879) で WorkflowUseCaseImpl に注入
```

### 確認事項
- パターン: docker-compose.yaml のサービス定義 → 既存の postgres/redis/dynamodb
- パターン: generate.sh の BASE_XXX_PORT + OFFSET → `scripts/env/generate.sh`
- パターン: `CoreConfig::from_env()` → `backend/apps/core-service/src/config.rs`
- パターン: main.rs の DI → `backend/apps/core-service/src/main.rs` L192-260

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | `just dev-deps` で Mailpit が起動し Web UI にアクセスできる | 正常系 | 手動確認 |
| 2 | `NOTIFICATION_BACKEND=smtp` で SmtpNotificationSender が選択される | 正常系 | ユニット |
| 3 | `NOTIFICATION_BACKEND=noop` で NoopNotificationSender が選択される | 正常系 | ユニット |
| 4 | SmtpNotificationSender で Mailpit にメールが送信される | 正常系 | 統合テスト |

### テストリスト

ユニットテスト:
- [ ] `NotificationConfig::from_env()` が smtp/ses/noop を正しくパースする

統合テスト (Mailpit):
- [ ] `SmtpNotificationSender` で Mailpit にメール送信 → Mailpit API で受信を検証
- [ ] 送信メールの件名が `[RingiFlow] 承認依頼: ...` パターンに一致
- [ ] 送信メールが HTML + plaintext 両方を含む

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | NotificationSender の配置先（domain vs infra）が未決定 | アーキテクチャ不整合 | async-trait 依存と既存パターン分析 → trait は infra、値型は domain |
| 2回目 | Noop 実装が Issue 完了基準文面と不一致 | 未定義 | Issue コメントで追加された Noop を含む 3 実装とする |
| 3回目 | NotificationLogRepository のスコープ境界が曖昧 | 不完全なパス | NotificationService の必須依存として本 Story に含める |
| 4回目 | テンプレートファイルの配置先が未決定 | 未定義 | include_str! の制約から core-service 内に配置 |
| 5回目 | aws-sdk-ses vs aws-sdk-sesv2 | 既存手段の見落とし | SES v2 API が推奨。aws-config は workspace 共有 |
| 6回目 | Mailpit API テスト用ポートの考慮漏れ | 不完全なパス | dev と api-test で異なるポートを割り当て |
| 7回目 | WorkflowUseCaseImpl への注入タイミング | スコープ境界 | 本 Story では構築のみ、注入は #876-#879 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 完了基準 4 項目: trait定義(P1-2), テンプレート(P3), Mailpit(P5), 環境変数切替(P5) |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 配置先、ポート番号、ライブラリ、ファイルパスを具体的に記載 |
| 3 | 設計判断の完結性 | 全差異に判断が記載 | OK | trait配置(判断1), テンプレート配置(判断2), LogRepo(判断3), lettre(判断4) |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 冒頭のスコープ判定で対象/対象外を列挙 |
| 5 | 技術的前提 | 前提が考慮されている | OK | include_str! 制約, async-trait 依存, lettre TLS feature, Mailpit SMTP ポート |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 設計書 16_通知機能設計.md と照合。型定義、テーブル定義、テンプレートパターン準拠 |

## 検証方法

1. `just check-all` — リント + テスト + API テスト + E2E テスト全通過
2. `just dev-deps` → `http://localhost:18025` で Mailpit Web UI にアクセス
3. 統合テスト: SmtpNotificationSender → Mailpit API でメール受信確認
4. 環境変数切替: `NOTIFICATION_BACKEND=noop` で core-service 起動、ログ出力のみを確認

## 参照ファイル（既存パターン）

| 用途 | ファイル |
|------|---------|
| trait パターン | `backend/crates/infra/src/repository/user_repository.rs` |
| mock パターン | `backend/crates/infra/src/mock.rs` |
| DI 配線 | `backend/apps/core-service/src/main.rs` L192-260 |
| config パターン | `backend/apps/core-service/src/config.rs` |
| エラー型 | `backend/crates/domain/src/error.rs`, `backend/crates/infra/src/error.rs` |
| event_log | `backend/crates/shared/src/event_log.rs` |
| Docker Compose | `infra/docker/docker-compose.yaml` |
| 環境変数生成 | `scripts/env/generate.sh` |
| UUID ID マクロ | `backend/crates/domain/src/macros.rs` |
| usecase モジュール | `backend/apps/core-service/src/usecase.rs` |
