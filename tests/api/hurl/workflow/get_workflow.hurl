# ワークフロー詳細 API テスト
# GET /api/v1/workflows/{display_number}
#
# ワークフロー詳細の E2E テスト。BFF → Core Service の連携を検証する。

# =============================================================================
# 前提: ログインしてワークフローを作成
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
session_cookie: cookie "session_id"

# CSRF トークンを取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"

# ワークフローを作成
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請（詳細テスト用）",
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

# =============================================================================
# 正常系: ワークフロー詳細を取得できる
# =============================================================================
# Given: 下書き状態のワークフローが存在する
# When: display_number を指定して GET /api/v1/workflows/{dn} にリクエストする
# Then: ワークフローの詳細が返される

GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# WorkflowInstance required フィールドすべて検証（Capture → == パターン）
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.display_id" == "{{workflow_display_id}}"
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "経費申請（詳細テスト用）"
jsonpath "$.data.definition_id" == "{{workflow_definition_id}}"
jsonpath "$.data.status" == "Draft"
jsonpath "$.data.version" == 1
jsonpath "$.data.form_data.title" == "交通費"
jsonpath "$.data.form_data.description" == "出張時の交通費"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.initiated_by.name" == "{{admin_name}}"
jsonpath "$.data.current_step_id" == null
jsonpath "$.data.steps" isCollection
jsonpath "$.data.submitted_at" == null
jsonpath "$.data.completed_at" == null
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 異常系: 存在しない display_number は 404
# =============================================================================
# Given: 存在しない display_number
# When: GET /api/v1/workflows/{dn} にリクエストする
# Then: 404 Not Found が返される

GET {{bff_url}}/api/v1/workflows/99999999
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 404
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/workflow-instance-not-found"
jsonpath "$.status" == 404
