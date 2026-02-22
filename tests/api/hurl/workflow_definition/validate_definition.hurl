# ワークフロー定義検証 API テスト
# POST /api/v1/workflow-definitions/validate
#
# 定義 JSON のバリデーション。有効な定義（valid: true）と無効な定義（valid: false）を検証する。

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
# 正常系: 有効な定義 JSON を検証する（valid: true）
# =============================================================================
# Given: 管理者でログインしている
# When: 有効な定義 JSON で検証 API にリクエストする
# Then: valid=true、errors=[] が返される

POST {{bff_url}}/api/v1/workflow-definitions/validate
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
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

HTTP 200
[Asserts]
# required フィールド（ValidationResultData）
jsonpath "$.data.valid" == true
jsonpath "$.data.errors" count == 0

# =============================================================================
# 準正常系: 無効な定義 JSON を検証する（valid: false）
# =============================================================================
# Given: 管理者でログインしている
# When: 承認ステップのない定義 JSON で検証 API にリクエストする
# Then: valid=false、missing_approval_step エラーが返される

POST {{bff_url}}/api/v1/workflow-definitions/validate
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "definition": {
        "steps": [
            {"id": "start", "type": "start", "name": "開始"},
            {"id": "end", "type": "end", "name": "終了", "status": "approved"}
        ],
        "transitions": [
            {"from": "start", "to": "end"}
        ]
    }
}

HTTP 200
[Asserts]
# required フィールド（ValidationResultData）
jsonpath "$.data.valid" == false
jsonpath "$.data.errors" count >= 1
jsonpath "$.data.errors[0].code" == "missing_approval_step"
jsonpath "$.data.errors[0].message" == "承認ステップが必要です"
