# API テスト突合表

OpenAPI 仕様（`openapi/openapi.yaml`）と API テスト（Hurl: `tests/api/hurl/`）の対応関係。

計測日: 2026-02-22

## 対応表

### 認証

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 1 | /api/v1/auth/login | POST | login | auth/login.hurl | カバー済み |
| 2 | /api/v1/auth/logout | POST | logout | auth/logout.hurl | カバー済み |
| 3 | /api/v1/auth/me | GET | me | auth/me.hurl, auth/me_unauthorized.hurl | カバー済み |
| 4 | /api/v1/auth/csrf | GET | csrf | auth/csrf.hurl, auth/csrf_unauthorized.hurl | カバー済み |

### ユーザー管理

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 5 | /api/v1/users | GET | list_users | user/list_users.hurl | カバー済み |
| 6 | /api/v1/users | POST | create_user | user/create_user.hurl | カバー済み |
| 7 | /api/v1/users/{display_number} | GET | get_user_detail | user/get_user_detail.hurl | カバー済み |
| 8 | /api/v1/users/{display_number} | PATCH | update_user | user/update_user.hurl | カバー済み |
| 9 | /api/v1/users/{display_number}/status | PATCH | update_user_status | user/update_user_status.hurl | カバー済み |

### ロール管理

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 10 | /api/v1/roles | GET | list_roles | role/list_roles.hurl | カバー済み |
| 11 | /api/v1/roles | POST | create_role | role/create_role.hurl | カバー済み |
| 12 | /api/v1/roles/{role_id} | GET | get_role | role/get_role.hurl | カバー済み |
| 13 | /api/v1/roles/{role_id} | PATCH | update_role | role/update_role.hurl | カバー済み |
| 14 | /api/v1/roles/{role_id} | DELETE | delete_role | role/delete_role.hurl | カバー済み |

### ワークフロー

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 15 | /api/v1/workflows | GET | list_my_workflows | workflow/list_my_workflows.hurl | カバー済み |
| 16 | /api/v1/workflows | POST | create_workflow | workflow/create_workflow.hurl | カバー済み |
| 17 | /api/v1/workflows/{display_number} | GET | get_workflow | workflow/get_workflow.hurl | カバー済み |
| 18 | /api/v1/workflows/{display_number}/submit | POST | submit_workflow | workflow/submit_workflow.hurl | カバー済み |
| 19 | /api/v1/workflows/{display_number}/resubmit | POST | resubmit_workflow | workflow/resubmit_workflow.hurl, workflow/full_request_changes_resubmit_flow.hurl | カバー済み |
| 20 | /api/v1/workflows/{display_number}/steps/{step_display_number}/approve | POST | approve_step | workflow/approve_step.hurl, workflow/multi_step_approve.hurl | カバー済み |
| 21 | /api/v1/workflows/{display_number}/steps/{step_display_number}/reject | POST | reject_step | workflow/reject_step.hurl, workflow/multi_step_reject.hurl | カバー済み |
| 22 | /api/v1/workflows/{display_number}/steps/{step_display_number}/request-changes | POST | request_changes_step | workflow/request_changes_step.hurl, workflow/full_request_changes_resubmit_flow.hurl | カバー済み |
| 23 | /api/v1/workflows/{display_number}/comments | GET | list_comments | workflow/comments.hurl | カバー済み |
| 24 | /api/v1/workflows/{display_number}/comments | POST | post_comment | workflow/comments.hurl | カバー済み |

### ワークフロー定義

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 25 | /api/v1/workflow-definitions | GET | list_workflow_definitions | workflow_definition/list_workflow_definitions.hurl | カバー済み |
| 26 | /api/v1/workflow-definitions | POST | create_definition | workflow_definition/create_definition.hurl | カバー済み |
| 27 | /api/v1/workflow-definitions/validate | POST | validate_definition | workflow_definition/validate_definition.hurl | カバー済み |
| 28 | /api/v1/workflow-definitions/{id} | GET | get_workflow_definition | workflow_definition/get_workflow_definition.hurl | カバー済み |
| 29 | /api/v1/workflow-definitions/{id} | PUT | update_definition | workflow_definition/update_definition.hurl | カバー済み |
| 30 | /api/v1/workflow-definitions/{id} | DELETE | delete_definition | workflow_definition/delete_definition.hurl | カバー済み |
| 31 | /api/v1/workflow-definitions/{id}/publish | POST | publish_definition | workflow_definition/publish_definition.hurl | カバー済み |
| 32 | /api/v1/workflow-definitions/{id}/archive | POST | archive_definition | workflow_definition/archive_definition.hurl | カバー済み |

### タスク

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 33 | /api/v1/tasks/my | GET | list_my_tasks | task/list_my_tasks.hurl | カバー済み |
| 34 | /api/v1/workflows/{workflow_display_number}/tasks/{step_display_number} | GET | get_task_by_display_numbers | task/get_task_by_display_numbers.hurl | カバー済み |

### ダッシュボード

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 35 | /api/v1/dashboard/stats | GET | get_dashboard_stats | dashboard/stats.hurl | カバー済み |

### 監査ログ

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 36 | /api/v1/audit-logs | GET | list_audit_logs | — | ギャップ |

### ヘルスチェック

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 37 | /health/ready | GET | readiness_check | health_ready.hurl | カバー済み |

注: `/health`（liveness check）は OpenAPI 仕様外だが、health.hurl でテスト済み。

## サマリー

| 指標 | 値 |
|------|-----|
| 全エンドポイント数（OpenAPI） | 37 |
| カバー済み | 36 (42 テストファイル) |
| ギャップ | 1 |
| カバー率 | 97.3% |

### カテゴリ別カバレッジ

| カテゴリ | 全 EP | カバー済み | カバー率 |
|---------|-------|-----------|---------|
| 認証 | 4 | 4 | 100% |
| ユーザー管理 | 5 | 5 | 100% |
| ロール管理 | 5 | 5 | 100% |
| ワークフロー | 10 | 10 | 100% |
| ワークフロー定義 | 8 | 8 | 100% |
| タスク | 2 | 2 | 100% |
| ダッシュボード | 1 | 1 | 100% |
| 監査ログ | 1 | 0 | 0% |
| ヘルスチェック | 1 | 1 | 100% |

## ギャップ解消計画

Epic #774 で対応中。

| Story | カテゴリ | ギャップ数 |
|-------|---------|-----------|
| #776 | ユーザー管理 | ~~4~~ 完了 |
| #777 | ロール管理 | ~~5~~ 完了 |
| #778 | ワークフロー定義（書込系） | ~~6~~ 完了 |
| #779 | 監査ログ | 1 |

## 更新履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-22 | ワークフロー定義（書込系）6 EP のテスト追加を反映（#778）。ギャップ 7→1、カバー率 81.1%→97.3% |
| 2026-02-22 | ロール管理 5 EP のテスト追加を反映（#777）。ギャップ 12→7、カバー率 67.6%→81.1% |
| 2026-02-22 | ユーザー管理 4 EP のテスト追加を反映（#776）。ギャップ 16→12、カバー率 56.8%→67.6% |
| 2026-02-22 | OpenAPI 全 37 EP に同期（#775）。16 ギャップを明示。カテゴリ別セクションに再構成 |
| 2026-02-09 | 全ギャップ解消（#321）。9 テストファイル追加で 100% 達成 |
| 2026-02-08 | 初版作成（#291） |
