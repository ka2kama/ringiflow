# API テスト突合表

OpenAPI 仕様（`openapi/openapi.yaml`）と API テスト（Hurl: `tests/api/hurl/`）の対応関係。

計測日: 2026-02-09

## 対応表

| # | エンドポイント | メソッド | operationId | Hurl テスト | 状態 |
|---|---------------|---------|-------------|------------|------|
| 1 | /api/v1/auth/login | POST | login | auth/login.hurl | カバー済み |
| 2 | /api/v1/auth/logout | POST | logout | auth/logout.hurl | カバー済み |
| 3 | /api/v1/auth/me | GET | getCurrentUser | auth/me.hurl, auth/me_unauthorized.hurl | カバー済み |
| 4 | /api/v1/auth/csrf | GET | getCsrfToken | auth/csrf.hurl, auth/csrf_unauthorized.hurl | カバー済み |
| 5 | /api/v1/workflow-definitions | GET | listWorkflowDefinitions | workflow_definition/list_workflow_definitions.hurl | カバー済み |
| 6 | /api/v1/workflow-definitions/{id} | GET | getWorkflowDefinition | workflow_definition/get_workflow_definition.hurl | カバー済み |
| 7 | /api/v1/workflows | GET | listMyWorkflows | workflow/list_my_workflows.hurl | カバー済み |
| 8 | /api/v1/workflows | POST | createWorkflow | workflow/create_workflow.hurl | カバー済み |
| 9 | /api/v1/workflows/{display_number} | GET | getWorkflow | workflow/get_workflow.hurl | カバー済み |
| 10 | /api/v1/workflows/{display_number}/submit | POST | submitWorkflow | workflow/submit_workflow.hurl | カバー済み |
| 11 | /api/v1/workflows/{display_number}/steps/{step_display_number}/approve | POST | approveWorkflowStep | workflow/approve_step.hurl | カバー済み |
| 12 | /api/v1/workflows/{display_number}/steps/{step_display_number}/reject | POST | rejectWorkflowStep | workflow/reject_step.hurl | カバー済み |
| 13 | /api/v1/users | GET | listUsers | user/list_users.hurl | カバー済み |
| 14 | /api/v1/tasks/my | GET | listMyTasks | task/list_my_tasks.hurl | カバー済み |
| 15 | /api/v1/workflows/{workflow_display_number}/tasks/{step_display_number} | GET | getTaskByDisplayNumbers | task/get_task_by_display_numbers.hurl | カバー済み |
| 16 | /api/v1/dashboard/stats | GET | getDashboardStats | dashboard/stats.hurl | カバー済み |
| 17 | /health | GET | healthCheck | health.hurl | カバー済み |
| 18 | /health/ready | GET | readinessCheck | health_ready.hurl | カバー済み |

## サマリー

| 指標 | 値 |
|------|-----|
| 全エンドポイント数 | 18 |
| カバー済み | 18 (20 テストファイル) |
| ギャップ | 0 |
| カバー率 | 100% |

## 更新履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-09 | 全ギャップ解消（#321）。9 テストファイル追加で 100% 達成 |
| 2026-02-08 | 初版作成（#291） |
