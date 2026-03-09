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
csrf_token: jsonpath "$.token"

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
workflow_id: jsonpath "$.id"
workflow_display_id: jsonpath "$.display_id"
workflow_display_number: jsonpath "$.display_number"

# ワークフローを申請（user に割り当て）
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "approvers": [
        { "step_id": "approval", "assigned_to": "{{user_id}}" }
    ]
}

HTTP 200

# ワークフロー詳細を取得（steps を含むレスポンス）
GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Captures]
step_display_number: jsonpath "$.steps[0].display_number"

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
jsonpath "$.step.id" exists
jsonpath "$.step.display_id" exists
jsonpath "$.step.display_number" == {{step_display_number}}
jsonpath "$.step.step_id" == "approval"
jsonpath "$.step.step_name" == "承認"
jsonpath "$.step.step_type" == "approval"
jsonpath "$.step.status" == "Active"
jsonpath "$.step.version" == 1
jsonpath "$.step.assigned_to.id" == "{{user_id}}"
jsonpath "$.step.decision" == null
jsonpath "$.step.comment" == null
jsonpath "$.step.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.step.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# ワークフロー情報（WorkflowInstance required フィールド）
jsonpath "$.workflow.id" == "{{workflow_id}}"
jsonpath "$.workflow.display_id" == "{{workflow_display_id}}"
jsonpath "$.workflow.display_number" == {{workflow_display_number}}
jsonpath "$.workflow.title" == "経費申請（タスク詳細テスト用）"
jsonpath "$.workflow.definition_id" == "{{workflow_definition_id}}"
jsonpath "$.workflow.status" == "InProgress"
jsonpath "$.workflow.version" == 2
jsonpath "$.workflow.form_data" exists
jsonpath "$.workflow.initiated_by.id" == "{{admin_id}}"
jsonpath "$.workflow.initiated_by.name" == "{{admin_name}}"
jsonpath "$.workflow.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.workflow.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.workflow.submitted_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.workflow.steps" count == 1

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
