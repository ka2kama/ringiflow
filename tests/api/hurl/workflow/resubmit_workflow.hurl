# ワークフロー再申請テスト
#
# テスト対象: POST /api/v1/workflows/{display_number}/resubmit
#
# テストシナリオ:
# 1. admin でワークフロー作成 → 申請
# 2. user がステップを差し戻し → ChangesRequested
# 3. 異常系: 状態不正（InProgress の WF に再申請）→ 400
# 4. 正常系: admin が再申請 → InProgress に戻り、form_data が更新される

# ===== 前提構築: admin でワークフロー作成・申請 =====

# admin でログイン
POST {{bff_url}}/api/v1/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
  "email": "{{admin_email}}",
  "password": "password123"
}
HTTP 200
[Captures]
admin_session_id: cookie "session_id"

# CSRF トークン取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_id}}
HTTP 200
[Captures]
admin_csrf_token: jsonpath "$.data.token"

# ワークフロー作成
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "definition_id": "{{workflow_definition_id}}",
  "title": "再申請テスト申請",
  "form_data": {
    "reason": "初回申請データ",
    "amount": 10000
  }
}
HTTP 201
[Captures]
workflow_display_number: jsonpath "$.data.display_number"
workflow_display_id: jsonpath "$.data.display_id"
workflow_id: jsonpath "$.data.id"

# ワークフロー申請（user に承認タスクを割り当て）
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "approvers": [
    {
      "step_id": "approval",
      "assigned_to": "{{user_id}}"
    }
  ]
}
HTTP 200

# ワークフロー詳細取得（ステップ情報をキャプチャ）
GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_id}}
HTTP 200
[Captures]
step_display_number: jsonpath "$.data.steps[0].display_number"
step_version: jsonpath "$.data.steps[0].version"

# ===== user がステップを差し戻す =====

# user でログイン
POST {{bff_url}}/api/v1/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
  "email": "{{user_email}}",
  "password": "password123"
}
HTTP 200
[Captures]
user_session_id: cookie "session_id"

# user の CSRF トークン取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{user_session_id}}
HTTP 200
[Captures]
user_csrf_token: jsonpath "$.data.token"

# ステップ差し戻し
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step_display_number}}/request-changes
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{user_csrf_token}}
Cookie: session_id={{user_session_id}}
Content-Type: application/json
{
  "version": {{step_version}},
  "comment": "金額を確認してください"
}
HTTP 200
[Asserts]
jsonpath "$.data.status" == "ChangesRequested"
[Captures]
workflow_version: jsonpath "$.data.version"

# ===== 異常系テスト =====

# 別のワークフローを作成して InProgress 状態にする（状態不正テスト用）
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "definition_id": "{{workflow_definition_id}}",
  "title": "状態不正テスト用申請",
  "form_data": {
    "reason": "テスト"
  }
}
HTTP 201
[Captures]
invalid_wf_display_number: jsonpath "$.data.display_number"

POST {{bff_url}}/api/v1/workflows/{{invalid_wf_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "approvers": [
    {
      "step_id": "approval",
      "assigned_to": "{{user_id}}"
    }
  ]
}
HTTP 200

# InProgress のワークフローに再申請 → 400
POST {{bff_url}}/api/v1/workflows/{{invalid_wf_display_number}}/resubmit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "form_data": {
    "reason": "修正"
  },
  "approvers": [
    {
      "step_id": "approval",
      "assigned_to": "{{user_id}}"
    }
  ],
  "version": 2
}
HTTP 400

# ===== 正常系テスト: admin が再申請する =====

# Mailpit のメールをクリア（通知テスト用）
DELETE {{mailpit_url}}/api/v1/messages
HTTP 200

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/resubmit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "form_data": {
    "reason": "修正後のデータ",
    "amount": 15000
  },
  "approvers": [
    {
      "step_id": "approval",
      "assigned_to": "{{user_id}}"
    }
  ],
  "version": {{workflow_version}}
}
HTTP 200
[Asserts]
# ワークフローインスタンスの状態（required フィールドすべて検証）
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.display_id" == "{{workflow_display_id}}"
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "再申請テスト申請"
jsonpath "$.data.definition_id" == "{{workflow_definition_id}}"
jsonpath "$.data.status" == "InProgress"
jsonpath "$.data.version" == 4
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.initiated_by.name" == "{{admin_name}}"
# form_data が更新されている
jsonpath "$.data.form_data.reason" == "修正後のデータ"
jsonpath "$.data.form_data.amount" == 15000
# completed_at はクリアされる
jsonpath "$.data.completed_at" == null
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# 新しいステップが作成されている
jsonpath "$.data.steps" count == 1
jsonpath "$.data.steps[0].status" == "Active"
jsonpath "$.data.steps[0].version" == 1
jsonpath "$.data.steps[0].decision" == null
jsonpath "$.data.steps[0].comment" == null

# =============================================================================
# 通知検証: 再申請後に承認依頼メールが Mailpit に送信されている
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
jsonpath "$.messages[0].Subject" contains "再申請テスト申請"

# =============================================================================
# 異常系: 存在しないワークフローの再申請 → 404
# =============================================================================
# Given: 管理者でログインしている
# When: 存在しない display_number でワークフロー再申請を試みる
# Then: 404 Not Found が返される

POST {{bff_url}}/api/v1/workflows/99999999/resubmit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
    "form_data": {
        "reason": "テスト"
    },
    "approvers": [
        {
            "step_id": "approval",
            "assigned_to": "{{user_id}}"
        }
    ],
    "version": 1
}

HTTP 404
