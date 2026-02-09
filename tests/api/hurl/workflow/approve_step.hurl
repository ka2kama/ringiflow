# ワークフロー承認 API テスト
# POST /api/v1/workflows/{display_number}/steps/{step_display_number}/approve
#
# ワークフロー承認の E2E テスト。BFF → Core Service の連携を検証する。
# 正常系（承認成功）+ 異常系（CSRF エラー、権限エラー、楽観的ロック、Not Found）

# =============================================================================
# 前提: admin でワークフローを作成・申請し、user に承認タスクを割り当てる
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

# ワークフローを作成
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請（承認テスト用）",
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
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "assigned_to": "{{user_id}}"
}

HTTP 200

# ワークフロー詳細を取得してステップ情報を capture
GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Captures]
step_display_number: jsonpath "$.data.steps[0].display_number"
step_version: jsonpath "$.data.steps[0].version"

# =============================================================================
# 異常系: CSRF トークンなしでは承認できない
# =============================================================================
# Given: ログインしているが CSRF トークンがない
# When: 承認を試みる
# Then: 403 Forbidden が返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "version": {{step_version}},
    "comment": "承認します"
}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/csrf-validation-failed"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: 権限のないユーザーは承認できない
# =============================================================================
# Given: admin でログインしている（ステップは user に割り当て済み）
# When: admin が承認を試みる
# Then: 403 Forbidden が返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "version": {{step_version}},
    "comment": "承認します"
}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/forbidden"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: 存在しないステップは承認できない
# =============================================================================
# Given: 存在しない step_display_number
# When: 承認を試みる
# Then: 404 Not Found が返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/99999999/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "version": 1,
    "comment": "承認します"
}

HTTP 404
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/step-not-found"
jsonpath "$.status" == 404

# =============================================================================
# user でログインして承認
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

# CSRF トークンを取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{user_session_cookie}}

HTTP 200
[Captures]
user_csrf_token: jsonpath "$.data.token"

# =============================================================================
# 異常系: バージョン不一致では承認できない（楽観的ロック）
# =============================================================================
# Given: ステップのバージョンが 1
# When: バージョン 999 で承認を試みる
# Then: 409 Conflict が返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{user_csrf_token}}
Cookie: session_id={{user_session_cookie}}
Content-Type: application/json
{
    "version": 999,
    "comment": "承認します"
}

HTTP 409
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/conflict"
jsonpath "$.status" == 409

# =============================================================================
# 正常系: ワークフローを承認できる
# =============================================================================
# Given: user に割り当てられた Active なステップが存在する
# When: 正しいバージョンで承認する
# Then: ワークフローが Approved になり、ステップが Completed になる

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{user_csrf_token}}
Cookie: session_id={{user_session_cookie}}
Content-Type: application/json
{
    "version": {{step_version}},
    "comment": "承認します"
}

HTTP 200
[Asserts]
# ワークフローインスタンスの状態遷移を検証
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.display_id" == "{{workflow_display_id}}"
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "経費申請（承認テスト用）"
jsonpath "$.data.definition_id" == "{{workflow_definition_id}}"
jsonpath "$.data.status" == "Approved"
jsonpath "$.data.version" == 3
jsonpath "$.data.form_data.title" == "交通費"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.initiated_by.name" == "{{admin_name}}"
jsonpath "$.data.submitted_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# ステップの状態遷移を検証
jsonpath "$.data.steps" count == 1
jsonpath "$.data.steps[0].step_id" == "approval"
jsonpath "$.data.steps[0].step_name" == "承認"
jsonpath "$.data.steps[0].step_type" == "approval"
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].version" == 2
jsonpath "$.data.steps[0].decision" == "Approved"
jsonpath "$.data.steps[0].comment" == "承認します"
jsonpath "$.data.steps[0].assigned_to.id" == "{{user_id}}"
jsonpath "$.data.steps[0].assigned_to.name" == "{{user_name}}"
jsonpath "$.data.steps[0].completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.steps[0].created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.steps[0].updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
