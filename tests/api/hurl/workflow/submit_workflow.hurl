# ワークフロー申請 API テスト
# POST /api/v1/workflows/{display_number}/submit
#
# ワークフロー申請の E2E テスト。BFF → Core Service の連携を検証する。

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
    "title": "経費申請",
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
# 正常系: ワークフローを申請できる
# =============================================================================
# Given: 下書き状態のワークフローが存在する
# When: 承認者を指定してワークフローを申請する
# Then: ステータスが Pending に変わり、current_step_id が設定される

# Mailpit のメールをクリア（通知テスト用）
DELETE {{mailpit_url}}/api/v1/messages
HTTP 200

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "approvers": [
        { "step_id": "approval", "assigned_to": "{{user_id}}" }
    ]
}

HTTP 200
[Asserts]
# ワークフローのステータスが更新される
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.display_id" == "{{workflow_display_id}}"
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "経費申請"
jsonpath "$.data.status" == "InProgress"
jsonpath "$.data.version" == 2
jsonpath "$.data.current_step_id" == "approval"
jsonpath "$.data.submitted_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.initiated_by.name" == "{{admin_name}}"

# =============================================================================
# 通知検証: 承認依頼メールが Mailpit に送信されている
# =============================================================================

GET {{mailpit_url}}/api/v1/messages
[Options]
retry: 3
retry-interval: 1000
HTTP 200
[Asserts]
jsonpath "$.messages_count" == 1
jsonpath "$.messages[0].To[0].Address" == "{{user_email}}"
jsonpath "$.messages[0].Subject" contains "[RingiFlow] 承認依頼"
jsonpath "$.messages[0].Subject" contains "経費申請"

# =============================================================================
# 異常系: 未認証では申請できない（CSRF トークンなしで 403）
# =============================================================================
# Given: ログインしていない
# When: ワークフロー申請を試みる
# Then: 403 Forbidden が返される（CSRF ミドルウェアが先にチェック）

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "approvers": [
        { "step_id": "approval", "assigned_to": "{{user_id}}" }
    ]
}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/csrf-validation-failed"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: CSRF トークンなしでは申請できない
# =============================================================================
# Given: ログインしているが CSRF トークンがない
# When: ワークフロー申請を試みる
# Then: 403 Forbidden が返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "approvers": [
        { "step_id": "approval", "assigned_to": "{{user_id}}" }
    ]
}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/csrf-validation-failed"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: 存在しないワークフローは申請できない
# =============================================================================
# Given: 存在しない display_number
# When: ワークフロー申請を試みる
# Then: 404 Not Found が返される

POST {{bff_url}}/api/v1/workflows/99999999/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "approvers": [
        { "step_id": "approval", "assigned_to": "{{user_id}}" }
    ]
}

HTTP 404
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/workflow-instance-not-found"
jsonpath "$.status" == 404
