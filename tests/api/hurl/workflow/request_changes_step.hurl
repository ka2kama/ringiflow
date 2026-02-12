# ワークフローステップ差し戻しテスト
#
# テスト対象: POST /api/v1/workflows/{display_number}/steps/{step_display_number}/request-changes
#
# テストシナリオ:
# 1. admin でワークフロー作成 → 申請
# 2. 異常系: CSRF トークンなし → 403
# 3. 異常系: 存在しないワークフロー → 404
# 4. 正常系: user がステップを差し戻し → ChangesRequested

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
  "title": "差し戻しテスト申請",
  "form_data": {
    "reason": "テスト"
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

# ===== 異常系テスト =====

# CSRF トークンなしで差し戻し → 403
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step_display_number}}/request-changes
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "version": {{step_version}},
  "comment": "差し戻します"
}
HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/csrf-validation-failed"

# 存在しないワークフローで差し戻し → 404
POST {{bff_url}}/api/v1/workflows/99999999/steps/99999999/request-changes
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "version": 1,
  "comment": "差し戻します"
}
HTTP 404

# ===== 正常系テスト: user がステップを差し戻す =====

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
  "comment": "フォームデータに不備があります。修正をお願いします。"
}
HTTP 200
[Asserts]
# ワークフローインスタンスの状態（required フィールドすべて検証）
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.display_id" == "{{workflow_display_id}}"
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "差し戻しテスト申請"
jsonpath "$.data.definition_id" == "{{workflow_definition_id}}"
jsonpath "$.data.status" == "ChangesRequested"
jsonpath "$.data.version" == 3
jsonpath "$.data.form_data.reason" == "テスト"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.initiated_by.name" == "{{admin_name}}"
jsonpath "$.data.submitted_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
# completed_at は設定されない（ChangesRequested は中間状態）
jsonpath "$.data.completed_at" == null
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# ステップの状態
jsonpath "$.data.steps" count == 1
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].version" == 2
jsonpath "$.data.steps[0].decision" == "RequestChanges"
jsonpath "$.data.steps[0].comment" == "フォームデータに不備があります。修正をお願いします。"
jsonpath "$.data.steps[0].completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
