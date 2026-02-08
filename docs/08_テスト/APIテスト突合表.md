# API テスト突合表

OpenAPI 仕様（`openapi/openapi.yaml`）と API テスト（Hurl: `tests/api/hurl/`）の対応関係。

計測日: 2026-02-08

## 対応表

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 1 | /api/v1/auth/login | POST | login | auth/login.hurl | カバー済み |
| 2 | /api/v1/auth/logout | POST | logout | auth/logout.hurl | カバー済み |
| 3 | /api/v1/auth/me | GET | getCurrentUser | auth/me.hurl, auth/me_unauthorized.hurl | カバー済み |
| 4 | /api/v1/auth/csrf | GET | getCsrfToken | auth/csrf.hurl, auth/csrf_unauthorized.hurl | カバー済み |
| 5 | /api/v1/workflow-definitions | GET | listWorkflowDefinitions | — | ギャップ |
| 6 | /api/v1/workflow-definitions/{id} | GET | getWorkflowDefinition | — | ギャップ |
| 7 | /api/v1/workflows | GET | listMyWorkflows | — | ギャップ |
| 8 | /api/v1/workflows | POST | createWorkflow | workflow/create_workflow.hurl | カバー済み |
| 9 | /api/v1/workflows/{display_number} | GET | getWorkflow | — | ギャップ |
| 10 | /api/v1/workflows/{display_number}/submit | POST | submitWorkflow | workflow/submit_workflow.hurl | カバー済み |
| 11 | /api/v1/workflows/{display_number}/steps/{step_display_number}/approve | POST | approveWorkflowStep | — | ギャップ |
| 12 | /api/v1/workflows/{display_number}/steps/{step_display_number}/reject | POST | rejectWorkflowStep | — | ギャップ |
| 13 | /api/v1/users | GET | listUsers | — | ギャップ |
| 14 | /api/v1/tasks/my | GET | listMyTasks | task/list_my_tasks.hurl | カバー済み |
| 15 | /api/v1/workflows/{workflow_display_number}/tasks/{step_display_number} | GET | getTaskByDisplayNumbers | task/get_task_by_display_numbers.hurl | カバー済み |
| 16 | /api/v1/dashboard/stats | GET | getDashboardStats | — | ギャップ |
| 17 | /health | GET | healthCheck | health.hurl | カバー済み |
| 18 | /health/ready | GET | readinessCheck | — | ギャップ |

## サマリー

| 指標 | 値 |
|------|-----|
| 全エンドポイント数 | 18 |
| カバー済み | 9 (11 テストファイル) |
| ギャップ | 9 |
| カバー率 | 50% |

## ギャップの優先度

| 優先度 | エンドポイント | 根拠 |
|--------|--------------|------|
| 高 | approveWorkflowStep, rejectWorkflowStep | コアビジネスフロー（承認・却下）、楽観的ロックの E2E 検証 |
| 中 | listMyWorkflows, getWorkflow | 申請一覧・詳細表示、ユーザーの主要操作 |
| 中 | getDashboardStats | ダッシュボードのランディング |
| 低 | listWorkflowDefinitions, getWorkflowDefinition | 参照系、比較的シンプル |
| 低 | listUsers | 参照系 |
| 低 | readinessCheck | 運用系 |

## 更新履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-08 | 初版作成（#291） |
