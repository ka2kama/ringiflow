# ワークフロー定義公開 API テスト
# POST /api/v1/workflow-definitions/{id}/publish
#
# Draft 状態の定義を公開する（Draft → Published）。楽観的ロック（version）の検証を含む。

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
# セットアップ: 公開対象の Draft 定義を作成する
# =============================================================================

POST {{bff_url}}/api/v1/workflow-definitions
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "公開対象の申請",
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
def_name: jsonpath "$.data.name"

# =============================================================================
# 正常系: Draft 状態の定義を公開できる（Draft → Published）
# =============================================================================
# Given: Draft 状態のワークフロー定義が存在する（version=1）
# When: POST /api/v1/workflow-definitions/{id}/publish にリクエストする
# Then: Published 状態に遷移し、200 OK が返される（version=2）

POST {{bff_url}}/api/v1/workflow-definitions/{{def_id}}/publish
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "version": 1
}

HTTP 200
[Asserts]
# required フィールド（WorkflowDefinitionData）
jsonpath "$.data.id" == "{{def_id}}"
jsonpath "$.data.name" == "{{def_name}}"
jsonpath "$.data.version" == 2
jsonpath "$.data.status" == "Published"
jsonpath "$.data.definition" exists
jsonpath "$.data.created_by" matches "^[a-f0-9-]{36}$"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# nullable フィールド
jsonpath "$.data.description" exists
