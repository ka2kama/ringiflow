# 2段階承認 API テスト
# POST /api/v1/workflows/{display_number}/steps/{step_display_number}/approve
#
# 2段階承認ワークフローの E2E テスト。
# 作成 → 申請 → ステップ1承認 → ステップ2承認 → Approved

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
    "title": "2段階承認テスト",
    "form_data": {
        "title": "高額備品購入",
        "description": "ノートPC購入申請",
        "amount": 200000
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
[Asserts]
jsonpath "$.data.status" == "InProgress"
jsonpath "$.data.current_step_id" == "manager_approval"

# ワークフロー詳細を取得してステップ情報を確認
GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Captures]
step1_display_number: jsonpath "$.data.steps[0].display_number"
step1_version: jsonpath "$.data.steps[0].version"
step2_display_number: jsonpath "$.data.steps[1].display_number"
[Asserts]
# 2つのステップが作成されている
jsonpath "$.data.steps" count == 2
# ステップ1: 上長承認（Active）
jsonpath "$.data.steps[0].step_id" == "manager_approval"
jsonpath "$.data.steps[0].step_name" == "上長承認"
jsonpath "$.data.steps[0].status" == "Active"
jsonpath "$.data.steps[0].assigned_to.id" == "{{admin_id}}"
# ステップ2: 経理承認（Pending）
jsonpath "$.data.steps[1].step_id" == "finance_approval"
jsonpath "$.data.steps[1].step_name" == "経理承認"
jsonpath "$.data.steps[1].status" == "Pending"
jsonpath "$.data.steps[1].assigned_to.id" == "{{user_id}}"

# =============================================================================
# ステップ1: admin が上長承認する（中間ステップ）
# =============================================================================
# Given: admin に割り当てられた Active なステップ1が存在する
# When: admin がステップ1を承認する
# Then: ステップ1が Completed になり、ステップ2が Active になる
#       インスタンスは InProgress のまま

# Mailpit のメールをクリア（ステップ1通知テスト用）
DELETE {{mailpit_url}}/api/v1/messages
HTTP 200

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step1_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "version": {{step1_version}},
    "comment": "上長承認します"
}

HTTP 200
[Captures]
step2_version_after_step1: jsonpath "$.data.steps[1].version"
[Asserts]
# インスタンスは InProgress のまま
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.status" == "InProgress"
# current_step_id が次のステップに更新されている
jsonpath "$.data.current_step_id" == "finance_approval"
jsonpath "$.data.submitted_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.initiated_by.name" == "{{admin_name}}"
# ステップ1: 承認済み
jsonpath "$.data.steps[0].step_id" == "manager_approval"
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].decision" == "Approved"
jsonpath "$.data.steps[0].comment" == "上長承認します"
jsonpath "$.data.steps[0].completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
# ステップ2: Active に遷移
jsonpath "$.data.steps[1].step_id" == "finance_approval"
jsonpath "$.data.steps[1].status" == "Active"

# =============================================================================
# 通知検証: 中間ステップ承認で StepApproved + ApprovalRequest の2通
# =============================================================================

GET {{mailpit_url}}/api/v1/messages
[Options]
retry: 3
retry-interval: 1000
HTTP 200
[Asserts]
jsonpath "$.messages_count" == 2

# =============================================================================
# ステップ2: user が経理承認する（最終ステップ）
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
# When: user がステップ2を承認する
# Then: インスタンスが Approved になる

# Mailpit のメールをクリア（最終ステップ通知テスト用）
DELETE {{mailpit_url}}/api/v1/messages
HTTP 200

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step2_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{user_csrf_token}}
Cookie: session_id={{user_session_cookie}}
Content-Type: application/json
{
    "version": {{step2_version_after_step1}},
    "comment": "経理承認します"
}

HTTP 200
[Asserts]
# インスタンスが Approved になっている
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.display_id" == "{{workflow_display_id}}"
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "2段階承認テスト"
jsonpath "$.data.status" == "Approved"
jsonpath "$.data.form_data.title" == "高額備品購入"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.initiated_by.name" == "{{admin_name}}"
jsonpath "$.data.submitted_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
# 両ステップが Completed
jsonpath "$.data.steps" count == 2
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].decision" == "Approved"
jsonpath "$.data.steps[1].status" == "Completed"
jsonpath "$.data.steps[1].decision" == "Approved"
jsonpath "$.data.steps[1].comment" == "経理承認します"
jsonpath "$.data.steps[1].completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 通知検証: 最終ステップ承認で承認完了メールが申請者（admin）に送信
# =============================================================================

GET {{mailpit_url}}/api/v1/messages
[Options]
retry: 3
retry-interval: 1000
HTTP 200
[Asserts]
jsonpath "$.messages_count" == 1
jsonpath "$.messages[0].To[0].Address" == "{{admin_email}}"
jsonpath "$.messages[0].Subject" contains "[RingiFlow] 承認完了"
jsonpath "$.messages[0].Subject" contains "2段階承認テスト"
