# ワークフロー定義更新 API テスト
# PUT /api/v1/workflow-definitions/{id}
#
# Draft 状態の定義を更新する。楽観的ロック（version）の検証を含む。

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
# セットアップ: 更新対象の定義を作成する
# =============================================================================

POST {{bff_url}}/api/v1/workflow-definitions
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "更新前の申請",
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
# 正常系: Draft 状態の定義を更新できる
# =============================================================================
# Given: Draft 状態のワークフロー定義が存在する
# When: 名前と定義を変更して更新する（version=1）
# Then: 更新され、200 OK が返される（version=2 にインクリメント）

PUT {{bff_url}}/api/v1/workflow-definitions/{{def_id}}
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "更新後の申請",
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
    },
    "version": 1
}

HTTP 200
[Asserts]
# required フィールド（WorkflowDefinitionData）
jsonpath "$.data.id" == "{{def_id}}"
jsonpath "$.data.name" == "更新後の申請"
jsonpath "$.data.version" == 2
jsonpath "$.data.status" == "Draft"
jsonpath "$.data.definition" exists
jsonpath "$.data.created_by" matches "^[a-f0-9-]{36}$"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# nullable フィールド
jsonpath "$.data.description" exists

# =============================================================================
# 異常系: 存在しない定義の更新 → 404
# =============================================================================
# Given: 管理者でログインしている
# When: 存在しない UUID でワークフロー定義の更新を試みる
# Then: 404 Not Found が返される

PUT {{bff_url}}/api/v1/workflow-definitions/99999999-9999-9999-9999-999999999999
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "存在しない定義",
    "definition": {
        "steps": [
            {"id": "start", "type": "start", "name": "開始"},
            {"id": "end", "type": "end", "name": "終了", "status": "approved"}
        ],
        "transitions": [
            {"from": "start", "to": "end"}
        ]
    },
    "version": 1
}

HTTP 404
