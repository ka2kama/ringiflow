# 差し戻し → 再申請 → 全ステップ承認 フルフローテスト
#
# テスト対象: 2段階承認での差し戻し → 再申請 → 承認完了の E2E フロー
#
# テストシナリオ:
# 1. admin でワークフロー作成（2段階承認） → 申請
# 2. admin（上長）がステップ1を承認
# 3. user（経理）がステップ2を差し戻し → ChangesRequested
# 4. admin が修正して再申請 → InProgress
# 5. admin がステップ1を承認
# 6. user がステップ2を承認 → Approved

# ===== 前提構築: admin でログイン =====

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

GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_id}}
HTTP 200
[Captures]
admin_csrf_token: jsonpath "$.data.token"

# ===== ワークフロー作成（2段階承認） =====

POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "definition_id": "{{multi_step_definition_id}}",
  "title": "フルフロー差し戻しテスト",
  "form_data": {
    "reason": "初回申請",
    "amount": 50000
  }
}
HTTP 201
[Captures]
workflow_display_number: jsonpath "$.data.display_number"
workflow_id: jsonpath "$.data.id"

# ワークフロー申請（Step1: admin、Step2: user）
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "approvers": [
    {
      "step_id": "manager_approval",
      "assigned_to": "{{admin_id}}"
    },
    {
      "step_id": "finance_approval",
      "assigned_to": "{{user_id}}"
    }
  ]
}
HTTP 200

# ステップ情報取得
GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_id}}
HTTP 200
[Captures]
step1_display_number: jsonpath "$.data.steps[0].display_number"
step1_version: jsonpath "$.data.steps[0].version"
step2_display_number: jsonpath "$.data.steps[1].display_number"

# ===== ステップ1: admin による上長承認 =====

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step1_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "version": {{step1_version}}
}
HTTP 200
[Asserts]
jsonpath "$.data.status" == "InProgress"
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].decision" == "Approved"
jsonpath "$.data.steps[1].status" == "Active"
[Captures]
step2_version: jsonpath "$.data.steps[1].version"

# ===== ステップ2: user による差し戻し =====

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

GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{user_session_id}}
HTTP 200
[Captures]
user_csrf_token: jsonpath "$.data.token"

# 経理承認ステップで差し戻し
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{step2_display_number}}/request-changes
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{user_csrf_token}}
Cookie: session_id={{user_session_id}}
Content-Type: application/json
{
  "version": {{step2_version}},
  "comment": "金額の根拠を追記してください"
}
HTTP 200
[Asserts]
# 差し戻し後の状態（required フィールドすべて検証）
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "フルフロー差し戻しテスト"
jsonpath "$.data.definition_id" == "{{multi_step_definition_id}}"
jsonpath "$.data.status" == "ChangesRequested"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.completed_at" == null
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].decision" == "Approved"
jsonpath "$.data.steps[1].status" == "Completed"
jsonpath "$.data.steps[1].decision" == "RequestChanges"
jsonpath "$.data.steps[1].comment" == "金額の根拠を追記してください"
[Captures]
workflow_version: jsonpath "$.data.version"

# ===== admin が再申請する =====

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/resubmit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "form_data": {
    "reason": "金額の根拠を追記しました",
    "amount": 50000,
    "justification": "見積書 No.12345 参照"
  },
  "approvers": [
    {
      "step_id": "manager_approval",
      "assigned_to": "{{admin_id}}"
    },
    {
      "step_id": "finance_approval",
      "assigned_to": "{{user_id}}"
    }
  ],
  "version": {{workflow_version}}
}
HTTP 200
[Asserts]
# 再申請後の状態（required フィールドすべて検証）
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "フルフロー差し戻しテスト"
jsonpath "$.data.definition_id" == "{{multi_step_definition_id}}"
jsonpath "$.data.status" == "InProgress"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.form_data.justification" == "見積書 No.12345 参照"
jsonpath "$.data.completed_at" == null
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
# 新しいステップが作成されている
jsonpath "$.data.steps" count == 2
jsonpath "$.data.steps[0].status" == "Active"
jsonpath "$.data.steps[0].version" == 1
jsonpath "$.data.steps[1].status" == "Pending"
jsonpath "$.data.steps[1].version" == 1
[Captures]
new_step1_display_number: jsonpath "$.data.steps[0].display_number"
new_step1_version: jsonpath "$.data.steps[0].version"

# ===== ステップ1 再承認: admin による上長承認 =====

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{new_step1_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_id}}
Content-Type: application/json
{
  "version": {{new_step1_version}}
}
HTTP 200
[Asserts]
jsonpath "$.data.status" == "InProgress"
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].decision" == "Approved"
jsonpath "$.data.steps[1].status" == "Active"
[Captures]
new_step2_display_number: jsonpath "$.data.steps[1].display_number"
new_step2_version: jsonpath "$.data.steps[1].version"

# ===== ステップ2 承認: user による経理承認 → Approved =====

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/steps/{{new_step2_display_number}}/approve
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{user_csrf_token}}
Cookie: session_id={{user_session_id}}
Content-Type: application/json
{
  "version": {{new_step2_version}}
}
HTTP 200
[Asserts]
# ワークフロー完了（required フィールドすべて検証）
jsonpath "$.data.display_number" == {{workflow_display_number}}
jsonpath "$.data.title" == "フルフロー差し戻しテスト"
jsonpath "$.data.definition_id" == "{{multi_step_definition_id}}"
jsonpath "$.data.status" == "Approved"
jsonpath "$.data.initiated_by.id" == "{{admin_id}}"
jsonpath "$.data.completed_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
# 両ステップとも承認完了
jsonpath "$.data.steps[0].status" == "Completed"
jsonpath "$.data.steps[0].decision" == "Approved"
jsonpath "$.data.steps[1].status" == "Completed"
jsonpath "$.data.steps[1].decision" == "Approved"
