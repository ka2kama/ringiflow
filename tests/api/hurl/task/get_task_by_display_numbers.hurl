# タスク詳細取得 API テスト
# GET /api/v1/workflows/{display_number}/tasks/{step_display_number}
#
# タスク詳細取得の E2E テスト。BFF → Core Service の連携を検証する。

# =============================================================================
# 前提: admin でワークフローを作成・申請し、user にタスクを割り当てる
# =============================================================================

POST {{bff_url}}/api/v1/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}

HTTP 200
[Captures]
admin_session_cookie: cookie "session_id"

# CSRF トークンを取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"

# ワークフローを作成
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請（タスク詳細テスト用）",
    "form_data": {
        "title": "交通費",
        "description": "出張時の交通費"
    }
}

HTTP 201
[Captures]
workflow_id: jsonpath "$.data.id"
workflow_display_id: jsonpath "$.data.display_id"
workflow_display_number: jsonpath "$.data.display_number"

# ワークフローを申請（user に割り当て）
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "assigned_to": "{{user_id}}"
}

HTTP 200

# ワークフロー詳細を取得（steps を含むレスポンス）
GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Captures]
step_display_number: jsonpath "$.data.steps[0].display_number"

# =============================================================================
# user でログインしてタスクを取得
# =============================================================================

POST {{bff_url}}/api/v1/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{user_email}}",
    "password": "{{password}}"
}

HTTP 200
[Captures]
user_session_cookie: cookie "session_id"

# =============================================================================
# 正常系: タスク詳細を取得できる
# =============================================================================
# Given: user に割り当てられたアクティブなタスクが存在する
# When: display_number でタスク詳細を取得する
# Then: ステップ情報とワークフロー全体が返される

GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/tasks/{{step_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{user_session_cookie}}

HTTP 200
[Asserts]
# ステップ情報
jsonpath "$.data.step.id" exists
jsonpath "$.data.step.display_id" exists
jsonpath "$.data.step.display_number" == {{step_display_number}}
jsonpath "$.data.step.step_id" == "approval"
jsonpath "$.data.step.step_name" == "承認"
jsonpath "$.data.step.step_type" == "approval"
jsonpath "$.data.step.status" == "Active"
jsonpath "$.data.step.version" == 1
jsonpath "$.data.step.assigned_to.id" == "{{user_id}}"
jsonpath "$.data.step.decision" == null
jsonpath "$.data.step.comment" == null
jsonpath "$.data.step.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.step.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# ワークフロー情報
jsonpath "$.data.workflow.id" == "{{workflow_id}}"
jsonpath "$.data.workflow.display_id" == "{{workflow_display_id}}"
jsonpath "$.data.workflow.display_number" == {{workflow_display_number}}
jsonpath "$.data.workflow.title" == "経費申請（タスク詳細テスト用）"
jsonpath "$.data.workflow.status" == "InProgress"
jsonpath "$.data.workflow.version" == 2
jsonpath "$.data.workflow.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.workflow.initiated_by.name" == "{{admin_name}}"
jsonpath "$.data.workflow.submitted_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.workflow.steps" count == 1

# =============================================================================
# 異常系: 未認証では取得できない
# =============================================================================
# 注: Hurl は同一ファイル内でクッキーを共有するため、
# 未認証テストは別ファイルで実施すべき。このテストでは省略する。

# =============================================================================
# 異常系: 権限のないタスクは取得できない
# =============================================================================
# Given: admin でログインしている（タスクは user に割り当てられている）
# When: タスク詳細を取得する
# Then: 403 Forbidden が返される

GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/tasks/{{step_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/forbidden"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: 存在しないタスクは取得できない
# =============================================================================
# Given: 存在しない display_number
# When: タスク詳細を取得する
# Then: 404 Not Found が返される

GET {{bff_url}}/api/v1/workflows/99999999/tasks/1
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{user_session_cookie}}

HTTP 404
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/task-not-found"
jsonpath "$.status" == 404
