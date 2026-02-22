# ワークフロー定義作成 API テスト
# POST /api/v1/workflow-definitions
#
# 有効なデータでワークフロー定義を作成するテスト。

# =============================================================================
# 前提: ログインしてセッションを確立
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

# =============================================================================
# 正常系: ワークフロー定義を作成できる
# =============================================================================
# Given: 管理者でログインしている
# When: 有効なデータでワークフロー定義を作成する
# Then: 定義が作成され、201 Created が返される（status=Draft, version=1）

POST {{bff_url}}/api/v1/workflow-definitions
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "テスト申請",
    "definition": {
        "steps": [
            {"id": "start", "type": "start", "name": "開始"},
            {"id": "approval_1", "type": "approval", "name": "承認"},
            {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
            {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
        ],
        "transitions": [
            {"from": "start", "to": "approval_1"},
            {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
            {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
        ]
    }
}

HTTP 201
[Asserts]
# required フィールド（WorkflowDefinitionData）
jsonpath "$.data.id" matches "^[a-f0-9-]{36}$"
jsonpath "$.data.name" == "テスト申請"
jsonpath "$.data.version" == 1
jsonpath "$.data.status" == "Draft"
jsonpath "$.data.definition" exists
jsonpath "$.data.created_by" matches "^[a-f0-9-]{36}$"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# nullable フィールド（description は省略時 null）
jsonpath "$.data.description" exists
