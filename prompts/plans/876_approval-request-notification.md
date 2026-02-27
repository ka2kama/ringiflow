# #876 承認依頼通知を実装する

## 概要

ワークフロー申請（submit）・再申請（resubmit）時に、最初の承認ステップの承認者へ承認依頼メール通知を送信する。#875 で構築した通知基盤（`NotificationService`）をワークフローユースケースに統合する。

## スコープ

対象:
- `submit_workflow` ユースケースに `NotificationService.notify()` 呼び出しを追加
- `resubmit_workflow` ユースケースにも同様に追加
- `WorkflowUseCaseImpl` に `NotificationService` を注入（DI）
- `#[allow(dead_code)]` アノテーションの解消
- テスト（ユニットテスト + API テスト）

対象外:
- `approve_step`, `reject_step`, `request_changes` への通知統合（#877 以降）
- SES 本番バックエンドの有効化（#879）
- E2E テスト（設計書の方針に従い、API テストでカバー）
- フロントエンド変更（なし）

## 設計判断

### 1. ユーザー情報（メール・名前）の取得タイミング

`WorkflowNotification::ApprovalRequest` に必要なデータ:
- `approver_email`: 承認者のメールアドレス → `UserRepository.find_by_id()` で取得
- `applicant_name`: 申請者の名前 → `UserRepository.find_by_id()` で取得

選択肢:
- (A) トランザクション前に取得（ユーザー情報を先に引いておく）
- **(B) トランザクション後に取得（採用）** — 通知はトランザクション成功後にのみ実行。ユーザー情報取得もトランザクション後に行うことで、トランザクションのスコープを最小に保つ

### 2. ユーザー情報取得失敗時の挙動

fire-and-forget パターンに合わせ、ユーザー情報取得失敗時は通知をスキップしてエラーログのみ記録する。ワークフロー操作自体は成功を返す。

### 3. 通知用ヘルパーメソッドの配置

`submit_workflow` と `resubmit_workflow` は通知のためのデータ収集（ユーザー情報取得、`WorkflowNotification` 構築）が共通。共通ヘルパーメソッド `send_approval_request_notification()` を `WorkflowUseCaseImpl` に追加する。

配置先: `backend/apps/core-service/src/usecase/workflow/command/lifecycle/common.rs`
理由: submit/resubmit 共通ロジックが既にここにある（`validate_approvers`, `create_approval_steps`）

## Phase 1: NotificationService の DI 配線

### 確認事項
- 型: `NotificationService` → `backend/apps/core-service/src/usecase/notification/service.rs`
- 型: `WorkflowUseCaseImpl` → `backend/apps/core-service/src/usecase/workflow.rs`
- パターン: `main.rs` の DI 配線 → 行267-276（WorkflowUseCaseImpl）、行293-325（NotificationService）
- パターン: `build_sut` テストヘルパー → `backend/apps/core-service/src/usecase/workflow/command.rs`

### 変更内容

1. `WorkflowUseCaseImpl` に `notification_service: Arc<NotificationService>` フィールドを追加
2. `new()` コンストラクタに引数を追加
3. `main.rs` で `_notification_service` → `notification_service` に変更し、`WorkflowUseCaseImpl::new()` に渡す
4. `notification/service.rs` から `#[allow(dead_code)]` を削除
5. テストヘルパー `build_sut` に `MockNotificationSender` + `MockNotificationLogRepository` → `NotificationService` を追加

### 操作パス

該当なし（DI 配線のみ、ユーザー操作パスは存在しない）

### テストリスト

ユニットテスト:
- [ ] 既存の submit_workflow テストが全て通ること（`build_sut` 変更の回帰テスト）
- [ ] 既存の resubmit_workflow テストが全て通ること

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: 承認依頼通知の送信（submit_workflow）

### 確認事項
- 型: `WorkflowNotification::ApprovalRequest` → `backend/crates/domain/src/notification.rs`
- 型: `User` — `email()` → `&Email`（`as_str()` で `&str` 取得）、`name()` → `&UserName`（`as_str()` で `&str` 取得）
- 型: `DisplayId::new(display_prefix::WORKFLOW_INSTANCE, display_number).to_string()` → `"WF-42"` 形式
- 型: `ApprovalStepDef.name` → ステップ名（`String`）
- パターン: `submit.rs` のトランザクション完了後の処理パターン（行110-121）

### 変更内容

1. `common.rs` にヘルパーメソッドを追加:
```rust
/// 承認依頼通知を送信する（fire-and-forget）
///
/// ユーザー情報の取得に失敗した場合はログ出力のみでスキップする。
async fn send_approval_request_notification(
    &self,
    instance: &WorkflowInstance,
    first_step_name: &str,
    first_approver_user_id: &UserId,
    tenant_id: &TenantId,
)
```

2. `submit.rs` のトランザクション完了後（`commit_tx` 後、`log_business_event!` 後）に通知送信を追加

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ワークフロー申請 → 承認者にメール通知が届く | 正常系 | ユニットテスト |
| 2 | ワークフロー申請 → 通知送信失敗 → ワークフロー操作は成功 | 異常系 | ユニットテスト |
| 3 | ワークフロー申請 → 承認者ユーザー情報取得失敗 → 通知スキップ、操作は成功 | 異常系 | ユニットテスト |

### テストリスト

ユニットテスト:
- [ ] submit_workflow 正常系で MockNotificationSender にメールが記録される
- [ ] submit_workflow 正常系で通知の宛先・件名・ステップ名が正しい
- [ ] submit_workflow でユーザー情報取得失敗時もワークフロー操作は成功する

ハンドラテスト（該当なし）
API テスト（該当なし — Phase 3 で Mailpit 統合テストとして実施）
E2E テスト（該当なし）

## Phase 3: 承認依頼通知の送信（resubmit_workflow）

### 確認事項
- パターン: Phase 2 で作成した `send_approval_request_notification` ヘルパー
- パターン: `resubmit.rs` のトランザクション完了後の処理パターン（行126-137）

### 変更内容

1. `resubmit.rs` のトランザクション完了後に Phase 2 と同じヘルパーを使用して通知送信を追加

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ワークフロー再申請 → 承認者にメール通知が届く | 正常系 | ユニットテスト |
| 2 | ワークフロー再申請 → 通知送信失敗 → ワークフロー操作は成功 | 異常系 | ユニットテスト |

### テストリスト

ユニットテスト:
- [ ] resubmit_workflow 正常系で MockNotificationSender にメールが記録される
- [ ] resubmit_workflow 正常系で通知の宛先・件名が正しい

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 4: API テスト（Mailpit 統合）

### 確認事項
- パターン: 既存の Hurl API テスト → `backend/tests/api/`
- ライブラリ: Mailpit API → `GET /api/v1/messages` で受信メール一覧を取得
- パターン: Docker Compose の Mailpit サービス → `infra/docker/docker-compose.api-test.yaml`

### 変更内容

1. 既存の submit_workflow API テストの後に Mailpit API でメール受信を検証
2. 既存の resubmit_workflow API テストの後に同様に検証

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ワークフロー申請 → Mailpit にメールが届く → 件名・宛先が正しい | 正常系 | API テスト |
| 2 | ワークフロー再申請 → Mailpit にメールが届く → 件名・宛先が正しい | 正常系 | API テスト |

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）

API テスト:
- [ ] submit_workflow 後に Mailpit API で承認依頼メールの受信を確認
- [ ] resubmit_workflow 後に Mailpit API で承認依頼メールの受信を確認

E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | submit と resubmit でユーザー情報取得・通知構築が重複する | 重複の排除 | 共通ヘルパーメソッド `send_approval_request_notification` を common.rs に配置 |
| 2回目 | ユーザー情報取得失敗時の挙動が未定義 | 不完全なパス | fire-and-forget に合わせ、エラーログ + スキップで設計 |
| 3回目 | `display_id` の生成方法が曖昧 | 曖昧 | `DisplayId::new(display_prefix::WORKFLOW_INSTANCE, instance.display_number()).to_string()` を明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | submit, resubmit の両ユースケース + DI 配線 + テストを網羅。対象外（approve/reject/request_changes）を明示 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | ヘルパーメソッドのシグネチャ、データ取得方法、エラー時挙動を具体化 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | ユーザー情報取得タイミング、失敗時挙動、ヘルパー配置の3点を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 冒頭で対象/対象外を明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | fire-and-forget パターン、トランザクション後の通知、Mailpit API 仕様を確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 詳細設計書 16_通知機能設計.md のユースケース統合セクションと整合 |
