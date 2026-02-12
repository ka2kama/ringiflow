# #476 差し戻し・再申請機能の実装計画

## Context

承認者が申請を差し戻し、申請者が修正して再申請できるバックエンド機能を実装する。Phase 1 では却下→新規再申請のみだったが、差し戻しにより同じ申請を修正して再申請可能にする。

前提: #475（複数ステップ承認バックエンド）が完了済み。

## スコープ

対象:
- `ChangesRequested` ステータスの追加（DB マイグレーション + ドメインモデル）
- 差し戻し API（request-changes）
- 再申請 API（resubmit）
- OpenAPI 仕様書の更新
- API テスト

対象外:
- フロントエンド（Elm）
- 通知機能
- コメント機能の拡張

## 設計判断

### 1. `WorkflowStep::request_changes()` 専用メソッド

`approve()` / `reject()` と同様に専用メソッドを追加。理由: version インクリメントの一貫性。既存の `completed()` は version をインクリメントしないため、楽観的ロックのパターンが崩れる。

### 2. `resubmitted()` に form_data を含める

再申請は必ずフォームデータ更新を伴うため、メソッドに含める。`with_form_data()` のような汎用メソッドは YAGNI。

### 3. resubmit で承認者を受け取る

submit_workflow と同じ approvers パラメータを取る設計。「ステップ1から再開」だが承認者の変更が可能。

### 4. completed_at の扱い

`complete_with_request_changes()` では `completed_at` を設定**しない**。ChangesRequested は中間状態（終了状態ではない）で、再申請で InProgress に戻れるため。reject/approve の `completed_at: Some(now)` との差異。

### 5. resubmit の権限チェック

`initiated_by == user_id` をチェック。申請者本人のみ再申請可能。submit にはこの権限チェックがないが、resubmit は「自分の申請を修正する」操作なので本人確認が妥当。

### 6. API レスポンスのステータス表記

既存の DTO 変換は `format!("{:?}", status)` （Debug 形式）を使用。`ChangesRequested` は API レスポンスで `"ChangesRequested"`（PascalCase）となる。

## Phase 計画

### Phase 1: ドメインモデル + DB マイグレーション

#### 確認事項
- [ ] 型: `WorkflowInstanceStatus` の derive マクロ群 → `instance.rs` L47-65
- [ ] パターン: `complete_with_rejection()` の実装 → `instance.rs` L396-411
- [ ] パターン: `reject()` メソッド → `step.rs` L367-384
- [ ] パターン: DB マイグレーション命名 → `backend/migrations/` の最新ファイル

#### 実装内容

1. **DB マイグレーション**: `workflow_instances.status` CHECK 制約に `changes_requested` を追加
2. **`WorkflowInstanceStatus::ChangesRequested`**: enum バリアント + FromStr に追加
3. **`WorkflowInstance::complete_with_request_changes(now)`**: InProgress → ChangesRequested, version++, completed_at は設定しない
4. **`WorkflowInstance::resubmitted(form_data, step_id, now)`**: ChangesRequested → InProgress, form_data 更新, version++, completed_at = None
5. **`WorkflowStep::request_changes(comment, now)`**: Active → Completed(RequestChanges), version++

#### テストリスト

ユニットテスト:
- [ ] `test_差し戻し完了後の状態`: InProgress → ChangesRequested, version++, completed_at = None
- [ ] `test_処理中以外で差し戻しするとエラー`: Draft からエラー
- [ ] `test_再申請後の状態`: ChangesRequested → InProgress, form_data 更新, version++, completed_at = None
- [ ] `test_要修正以外で再申請するとエラー`: InProgress からエラー
- [ ] `test_差し戻しステップの状態`: Active → Completed(RequestChanges), version++
- [ ] `test_コメント付き差し戻しステップの状態`: comment 保存
- [ ] `test_アクティブ以外で差し戻しするとエラー`: Pending からエラー
- [ ] `test_要修正状態からの取消後の状態`: ChangesRequested → Cancelled（既存 cancelled() で通過確認）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 2: ユースケース層

#### 確認事項
- [ ] パターン: `reject_step()` のロジック → `command.rs` L411-521
- [ ] パターン: `submit_workflow()` のステップ作成 → `command.rs` L115-227
- [ ] 型: `ApproveRejectInput` / `SubmitWorkflowInput` / `StepApprover` → `workflow.rs` L52-73
- [ ] パターン: `_by_display_number` 委譲 → `command.rs` L581-661

#### 実装内容

1. **`ResubmitWorkflowInput`**: `{ form_data, approvers, version }` — `workflow.rs` に追加
2. **`request_changes_step(input, step_id, tenant_id, user_id)`**: reject_step のパターンを踏襲。差異: `step.request_changes()` + `instance.complete_with_request_changes()`
3. **`resubmit_workflow(input, instance_id, tenant_id, user_id)`**: ChangesRequested 確認 → 定義からステップ抽出 → 新ステップ作成 → `instance.resubmitted()` → 保存
4. **`_by_display_number` バリアント**: 2つ追加

#### テストリスト

ユニットテスト:
- [ ] `test_request_changes_step_正常系`: ステップ差し戻し → ChangesRequested
- [ ] `test_request_changes_step_未割り当てユーザーは403`
- [ ] `test_request_changes_step_active以外は400`
- [ ] `test_request_changes_step_バージョン不一致で409`
- [ ] `test_request_changes_step_残りのpendingステップがskipped`
- [ ] `test_resubmit_workflow_正常系`: form_data 更新 + 新ステップ作成
- [ ] `test_resubmit_workflow_要修正以外は400`
- [ ] `test_resubmit_workflow_バージョン不一致で409`
- [ ] `test_resubmit_workflow_approvers不一致でエラー`
- [ ] `test_resubmit_workflow_申請者以外は403`

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

### Phase 3: Core Service ハンドラ + ルート

#### 確認事項
- [ ] パターン: `reject_step` ハンドラ → `core-service/handler/workflow/command.rs` L176-210
- [ ] パターン: `submit_workflow` ハンドラ → `core-service/handler/workflow/command.rs` L84-118
- [ ] パターン: ルート登録 → `core-service/main.rs` のワークフロー関連ルート
- [ ] 型: `SubmitWorkflowRequest` → `core-service/handler/workflow.rs`

#### 実装内容

1. **`ResubmitWorkflowRequest`**: `{ form_data, approvers, version, tenant_id, user_id }` — Core Service 内部 API 用
2. **ハンドラ 4 つ**: request_changes_step, request_changes_step_by_display_number, resubmit_workflow, resubmit_workflow_by_display_number
3. **ルート登録 4 つ**: `/internal/workflows/...`

#### テストリスト

ユニットテスト（該当なし — ハンドラは薄い層。ユースケーステストと API テストでカバー）
ハンドラテスト（該当なし）
API テスト（該当なし — Phase 6 で実施）
E2E テスト（該当なし）

---

### Phase 4: BFF クライアント + ハンドラ + ルート

#### 確認事項
- [ ] パターン: `CoreServiceWorkflowClient` トレイト → `bff/client/core_service/workflow_client.rs`
- [ ] パターン: BFF `reject_step` ハンドラ → `bff/handler/workflow/command.rs`
- [ ] パターン: BFF `submit_workflow` ハンドラ → `bff/handler/workflow/command.rs`
- [ ] パターン: BFF ルート登録 → `bff/main.rs`

#### 実装内容

1. **Core Service クライアント**: `request_changes_step_by_display_number()`, `resubmit_workflow_by_display_number()` — トレイト + 実装
2. **BFF リクエスト型**: `ResubmitWorkflowRequest { form_data, approvers, version }` — BFF 公開 API 用
3. **BFF ハンドラ 2 つ**: request_changes_step, resubmit_workflow
4. **BFF ルート 2 つ**: `/api/v1/workflows/{dn}/steps/{sdn}/request-changes`, `/api/v1/workflows/{dn}/resubmit`

#### テストリスト

ユニットテスト（該当なし — BFF は薄い層。API テストでカバー）
ハンドラテスト（該当なし）
API テスト（該当なし — Phase 6 で実施）
E2E テスト（該当なし）

---

### Phase 5: OpenAPI 仕様書更新

#### 確認事項
- [ ] パターン: approve/reject エンドポイントの定義 → `openapi/openapi.yaml`
- [ ] パターン: submit エンドポイントの定義 → `openapi/openapi.yaml`

#### 実装内容

1. `POST /api/v1/workflows/{dn}/steps/{sdn}/request-changes` — approve/reject と同じリクエスト/レスポンス
2. `POST /api/v1/workflows/{dn}/resubmit` — ResubmitWorkflowRequest スキーマ追加
3. WorkflowInstanceStatus enum に `ChangesRequested` を追加
4. decision enum の明示化（Approved, Rejected, RequestChanges）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし — Phase 6 で実施）
E2E テスト（該当なし）

---

### Phase 6: API テスト (Hurl)

#### 確認事項
- [ ] パターン: `reject_step.hurl` の構造 → `tests/api/hurl/workflow/reject_step.hurl`
- [ ] パターン: `multi_step_approve.hurl` → `tests/api/hurl/workflow/multi_step_approve.hurl`
- [ ] パターン: シードデータとユーザー情報 → 既存 Hurl テスト

#### 実装内容

1. **`request_changes_step.hurl`**: 1段階承認での差し戻し + CSRF エラー + 存在しない WF
2. **`resubmit_workflow.hurl`**: 差し戻し → 再申請 → InProgress 確認 + form_data 更新検証 + 状態エラー
3. **`full_request_changes_resubmit_flow.hurl`**: 2段階承認での差し戻し → 再申請 → 全承認 → Approved（E2E フロー）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）

API テスト:
- [ ] `request_changes_step.hurl`: 正常系（差し戻し → ChangesRequested） + CSRF エラー + Not Found
- [ ] `resubmit_workflow.hurl`: 正常系（再申請 → InProgress, form_data 更新） + 状態エラー
- [ ] `full_request_changes_resubmit_flow.hurl`: 差し戻し → 再申請 → 全ステップ承認 → Approved

E2E テスト（該当なし — フロントエンドはスコープ外）

---

## 主要ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/migrations/新規` | status CHECK 制約に changes_requested 追加 |
| `backend/crates/domain/src/workflow/instance.rs` | ChangesRequested バリアント、complete_with_request_changes、resubmitted メソッド |
| `backend/crates/domain/src/workflow/step.rs` | request_changes メソッド |
| `backend/apps/core-service/src/usecase/workflow.rs` | ResubmitWorkflowInput 追加 |
| `backend/apps/core-service/src/usecase/workflow/command.rs` | request_changes_step、resubmit_workflow ユースケース |
| `backend/apps/core-service/src/handler/workflow.rs` | ResubmitWorkflowRequest 型追加 |
| `backend/apps/core-service/src/handler/workflow/command.rs` | 4 ハンドラ追加 |
| `backend/apps/core-service/src/main.rs` | 4 ルート追加 |
| `backend/apps/bff/src/client/core_service/workflow_client.rs` | 2 クライアントメソッド追加 |
| `backend/apps/bff/src/handler/workflow/command.rs` | 2 ハンドラ追加 |
| `backend/apps/bff/src/main.rs` | 2 ルート追加 |
| `openapi/openapi.yaml` | 2 エンドポイント + スキーマ追加 |
| `tests/api/hurl/workflow/` | 3 テストファイル追加 |

## 検証方法

1. `just check` — 各 Phase のコミット前に実行
2. `just sqlx-prepare` — Phase 1（マイグレーション後）と全 Phase 完了後
3. `just check-all` — 全 Phase 完了後（API テスト含む）
4. API テスト（Hurl）で差し戻し → 再申請 → 承認のフルフローを検証

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `completed_at` の扱い: request_changes で設定するか | 不完全なパス | ChangesRequested は中間状態 → completed_at を設定しない |
| 2回目 | cancelled() が ChangesRequested を受け入れるか | 不完全なパス | Approved/Rejected/Cancelled のみブロック → ChangesRequested はそのまま通過。変更不要 |
| 3回目 | 既存テスト `completed()` の version 不整合 | アーキテクチャ不整合 | `request_changes()` 専用メソッドを追加し version++ を含める。既存 `completed()` は維持 |
| 4回目 | resubmit の権限チェック未定義 | 未定義 | `initiated_by == user_id` チェックを追加 |
| 5回目 | API レスポンスの status 表記 | 既存手段の見落とし | `format!("{:?}", status)` で Debug 形式（PascalCase）→ `"ChangesRequested"` |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | ドメイン、ユースケース、ハンドラ、クライアント、ルート、OpenAPI、マイグレーション、API テスト全て計画に含む |
| 2 | 曖昧さ排除 | OK | メソッドシグネチャ、状態遷移、テストケースが具体的 |
| 3 | 設計判断の完結性 | OK | 6 つの設計判断をすべて理由付きで記載 |
| 4 | スコープ境界 | OK | 対象（バックエンド差し戻し・再申請）・対象外（フロントエンド、通知）を明記 |
| 5 | 技術的前提 | OK | DB CHECK 制約の ALTER、sqlx-prepare、Debug 形式の PascalCase 変換、strum snake_case 変換を確認 |
| 6 | 既存ドキュメント整合 | OK | 機能仕様書シナリオ6・セクション4.6、状態遷移図と整合 |
