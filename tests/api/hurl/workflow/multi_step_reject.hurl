# 2段階却下 API テスト
# POST /api/v1/workflows/{display_number}/steps/{step_display_number}/reject
#
# 2段階承認ワークフローの却下 E2E テスト。
# 作成 → 申請 → ステップ1承認 → ステップ2却下 → Rejected

# =============================================================================
# 前提: admin でログインし、2段階承認のワークフローを作成・申請する
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
admin_csrf_token: jsonpath "$.data.token"

# 2段階承認のワークフローを作成
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{multi_step_definition_id}}",
    "title": "2段階却下テスト",
    "form_data": {
        "title": "高額備品購入",
        "description": "却下テスト用",
        "amount": 500000
    }
}

HTTP 201
[Captures]
workflow_id: jsonpath "$.data.id"
workflow_display_id: jsonpath "$.data.display_id"
workflow_display_number: jsonpath "$.data.display_number"

# ワークフローを申請（admin=上長承認、user=経理承認）
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "approvers": [
        { "step_id": "manager_approval", "assigned_to": "{{admin_id}}" },
        { "step_id": "finance_approval", "assigned_to": "{{user_id}}" }
    ]
}

HTTP 200

# ワークフロー詳細を取得してステップ情報を確認
GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Captures]
step1_display_number: jsonpath "$.data.steps[0].display_number"
step1_version: jsonpath "$.data.steps[0].version"
step2_display_number: jsonpath "$.data.steps[1].display_number"

# =============================================================================
# ステップ1: admin が上長承認する
# =============================================================================

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step1_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "version": {{step1_version}},
    "comment": "上長承認OK"
}

HTTP 200
[Captures]
step2_version_after_step1: jsonpath "$.data.steps[1].version"
[Asserts]
jsonpath "$.data.status" == "InProgress"
jsonpath "$.data.current_step_id" == "finance_approval"
jsonpath "$.data.steps[1].status" == "Active"

# =============================================================================
# ステップ2: user が経理却下する（最終ステップの却下）
# =============================================================================

# user でログイン
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

# CSRF トークンを取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{user_session_cookie}}

HTTP 200
[Captures]
user_csrf_token: jsonpath "$.data.token"

# Given: user に割り当てられた Active なステップ2が存在する
# When: user がステップ2を却下する
# Then: インスタンスが Rejected になる（スキップ対象のステップなし）

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step2_display_number}}/reject
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{user_csrf_token}}
Cookie: session_id={{user_session_cookie}}
Content-Type: application/json
{
    "version": {{step2_version_after_step1}},
    "comment": "予算超過のため却下"
}

HTTP 200
[Asserts]
# インスタンスが Rejected になっている
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.display_id" == "{{workflow_display_id}}"
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "2段階却下テスト"
jsonpath "$.data.status" == "Rejected"
jsonpath "$.data.form_data.title" == "高額備品購入"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.initiated_by.name" == "{{admin_name}}"
jsonpath "$.data.submitted_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
# ステップ1: 承認済み
jsonpath "$.data.steps" count == 2
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].decision" == "Approved"
# ステップ2: 却下済み
jsonpath "$.data.steps[1].status" == "Completed"
jsonpath "$.data.steps[1].decision" == "Rejected"
jsonpath "$.data.steps[1].comment" == "予算超過のため却下"
jsonpath "$.data.steps[1].completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
