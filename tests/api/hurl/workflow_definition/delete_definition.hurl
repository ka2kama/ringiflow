# ワークフロー定義削除 API テスト
# DELETE /api/v1/workflow-definitions/{id}
#
# Published 定義の削除失敗（400）と Draft 定義の削除成功（204）を検証する。

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
# 準正常系: Published 状態の定義は削除できない（400）
# =============================================================================
# Given: Published 状態のワークフロー定義（シードデータ）が存在する
# When: DELETE /api/v1/workflow-definitions/{id} にリクエストする
# Then: 400 Bad Request（validation-error）が返される

DELETE {{bff_url}}/api/v1/workflow-definitions/{{workflow_definition_id}}
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}

HTTP 400
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/validation-error"
jsonpath "$.status" == 400

# =============================================================================
# セットアップ: 削除対象の Draft 定義を作成する
# =============================================================================

POST {{bff_url}}/api/v1/workflow-definitions
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "削除対象の申請",
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
[Captures]
def_id: jsonpath "$.data.id"

# =============================================================================
# 正常系: Draft 状態の定義を削除できる
# =============================================================================
# Given: Draft 状態のワークフロー定義が存在する
# When: DELETE /api/v1/workflow-definitions/{id} にリクエストする
# Then: 204 No Content が返される

DELETE {{bff_url}}/api/v1/workflow-definitions/{{def_id}}
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}

HTTP 204
